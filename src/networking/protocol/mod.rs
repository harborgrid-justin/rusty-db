//! Wire protocol for P2P communication
//!
//! This module defines the binary protocol used for communication between
//! RustyDB nodes in a distributed cluster. It includes message framing,
//! versioning, compression, and type definitions.
//!
//! # Protocol Design
//!
//! The protocol uses length-prefixed framing with the following structure:
//!
//! ```text
//! +--------+--------+------------+---------+
//! | Length | Flags  | Message ID | Payload |
//! | 4 bytes| 2 bytes| 8 bytes    | N bytes |
//! +--------+--------+------------+---------+
//! ```
//!
//! - **Length**: Total message length (excluding length field itself)
//! - **Flags**: Protocol version, compression type, etc.
//! - **Message ID**: Unique message identifier for request/response matching
//! - **Payload**: Serialized message data
//!
//! # Features
//!
//! - Protocol versioning for backward compatibility
//! - Optional compression (LZ4, Zstd)
//! - Message type safety through enums
//! - Checksum validation
//! - Request/response correlation

pub mod codec;
pub mod handshake;

pub use codec::{MessageCodec, ProtocolCodec};
pub use handshake::{Handshake, HandshakeRequest, HandshakeResponse, NodeCapabilities};

use serde::{Deserialize, Serialize};

/// Protocol version
pub const PROTOCOL_VERSION: u16 = 1;

/// Maximum message size (16 MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Protocol flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolFlags {
    /// Protocol version
    pub version: u8,

    /// Compression type
    pub compression: CompressionType,

    /// Whether checksum is included
    pub has_checksum: bool,

    /// Reserved for future use
    pub reserved: u8,
}

impl ProtocolFlags {
    /// Create default protocol flags
    pub fn new() -> Self {
        Self {
            version: PROTOCOL_VERSION as u8,
            compression: CompressionType::None,
            has_checksum: true,
            reserved: 0,
        }
    }

    /// Encode flags to 16-bit value
    pub fn encode(&self) -> u16 {
        let mut flags: u16 = 0;

        // Version (bits 0-3)
        flags |= (self.version as u16) & 0x0F;

        // Compression (bits 4-6)
        flags |= ((self.compression as u16) & 0x07) << 4;

        // Checksum flag (bit 7)
        if self.has_checksum {
            flags |= 1 << 7;
        }

        // Reserved (bits 8-15)
        flags |= ((self.reserved as u16) & 0xFF) << 8;

        flags
    }

    /// Decode flags from 16-bit value
    pub fn decode(flags: u16) -> Self {
        let version = (flags & 0x0F) as u8;
        let compression = ((flags >> 4) & 0x07) as u8;
        let has_checksum = (flags & (1 << 7)) != 0;
        let reserved = ((flags >> 8) & 0xFF) as u8;

        Self {
            version,
            compression: CompressionType::from_u8(compression),
            has_checksum,
            reserved,
        }
    }
}

impl Default for ProtocolFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompressionType {
    None = 0,
    Lz4 = 1,
    Zstd = 2,
}

impl CompressionType {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => CompressionType::Lz4,
            2 => CompressionType::Zstd,
            _ => CompressionType::None,
        }
    }
}

/// Message header
#[derive(Debug, Clone)]
pub struct MessageHeader {
    /// Total message length (excluding length field)
    pub length: u32,

    /// Protocol flags
    pub flags: ProtocolFlags,

    /// Message ID for request/response correlation
    pub message_id: u64,
}

impl MessageHeader {
    /// Size of the header in bytes (4 + 2 + 8 = 14 bytes)
    pub const SIZE: usize = 14;

    /// Create a new message header
    pub fn new(message_id: u64, payload_length: usize) -> Self {
        // Length includes flags (2) + message_id (8) + payload + checksum (4 if enabled)
        let flags = ProtocolFlags::new();
        let total_length = 2 + 8 + payload_length + if flags.has_checksum { 4 } else { 0 };

        Self {
            length: total_length as u32,
            flags,
            message_id,
        }
    }

    /// Encode header to bytes
    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        // Length (4 bytes, big-endian)
        buf[0..4].copy_from_slice(&self.length.to_be_bytes());

        // Flags (2 bytes)
        buf[4..6].copy_from_slice(&self.flags.encode().to_be_bytes());

        // Message ID (8 bytes, big-endian)
        buf[6..14].copy_from_slice(&self.message_id.to_be_bytes());

        buf
    }

    /// Decode header from bytes
    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        if buf.len() < Self::SIZE {
            return Err("Buffer too small for message header".to_string());
        }

        let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let flags_raw = u16::from_be_bytes([buf[4], buf[5]]);
        let flags = ProtocolFlags::decode(flags_raw);
        let message_id = u64::from_be_bytes([
            buf[6], buf[7], buf[8], buf[9], buf[10], buf[11], buf[12], buf[13],
        ]);

        Ok(Self {
            length,
            flags,
            message_id,
        })
    }
}

/// Message types for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Handshake request
    HandshakeRequest(HandshakeRequest),

    /// Handshake response
    HandshakeResponse(HandshakeResponse),

    /// Ping message for keepalive
    Ping {
        timestamp: u64,
    },

    /// Pong response to ping
    Pong {
        timestamp: u64,
    },

    /// Query request
    QueryRequest {
        query_id: u64,
        sql: String,
    },

    /// Query response
    QueryResponse {
        query_id: u64,
        result: Vec<u8>,
    },

    /// Replication log entry
    ReplicationLog {
        log_sequence_number: u64,
        data: Vec<u8>,
    },

    /// Consensus message (Raft)
    Consensus {
        term: u64,
        message_type: ConsensusMessageType,
        payload: Vec<u8>,
    },

    /// Data transfer
    DataTransfer {
        transfer_id: u64,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    },

    /// Error message
    Error {
        error_code: u32,
        message: String,
    },

    /// Generic request
    Request {
        request_id: u64,
        data: Vec<u8>,
    },

    /// Generic response
    Response {
        request_id: u64,
        data: Vec<u8>,
    },
}

impl Message {
    /// Get message type as string
    pub fn message_type(&self) -> &'static str {
        match self {
            Message::HandshakeRequest(_) => "HandshakeRequest",
            Message::HandshakeResponse(_) => "HandshakeResponse",
            Message::Ping { .. } => "Ping",
            Message::Pong { .. } => "Pong",
            Message::QueryRequest { .. } => "QueryRequest",
            Message::QueryResponse { .. } => "QueryResponse",
            Message::ReplicationLog { .. } => "ReplicationLog",
            Message::Consensus { .. } => "Consensus",
            Message::DataTransfer { .. } => "DataTransfer",
            Message::Error { .. } => "Error",
            Message::Request { .. } => "Request",
            Message::Response { .. } => "Response",
        }
    }
}

/// Consensus message types (for Raft)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessageType {
    RequestVote,
    RequestVoteResponse,
    AppendEntries,
    AppendEntriesResponse,
    InstallSnapshot,
    InstallSnapshotResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_flags_encoding() {
        let flags = ProtocolFlags {
            version: 1,
            compression: CompressionType::Lz4,
            has_checksum: true,
            reserved: 0,
        };

        let encoded = flags.encode();
        let decoded = ProtocolFlags::decode(encoded);

        assert_eq!(decoded.version, flags.version);
        assert_eq!(decoded.compression, flags.compression);
        assert_eq!(decoded.has_checksum, flags.has_checksum);
    }

    #[test]
    fn test_message_header_encoding() {
        let header = MessageHeader::new(12345, 1024);
        let encoded = header.encode();
        let decoded = MessageHeader::decode(&encoded).unwrap();

        assert_eq!(decoded.message_id, header.message_id);
        assert_eq!(decoded.length, header.length);
    }

    #[test]
    fn test_compression_type_conversion() {
        assert_eq!(CompressionType::from_u8(0), CompressionType::None);
        assert_eq!(CompressionType::from_u8(1), CompressionType::Lz4);
        assert_eq!(CompressionType::from_u8(2), CompressionType::Zstd);
        assert_eq!(CompressionType::from_u8(99), CompressionType::None);
    }

    #[test]
    fn test_message_type_names() {
        let msg = Message::Ping { timestamp: 123 };
        assert_eq!(msg.message_type(), "Ping");

        let msg = Message::Error {
            error_code: 500,
            message: "Test error".to_string(),
        };
        assert_eq!(msg.message_type(), "Error");
    }
}
