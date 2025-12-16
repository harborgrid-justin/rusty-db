// Message Types Module
//
// Protocol message types, versions, and packet structures

use std::collections::HashMap;

// ============================================================================
// Protocol Version and Compression
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const V1_0_0: Self = Self { major: 1, minor: 0 };
    pub const V1_1_0: Self = Self { major: 1, minor: 1 };
    pub const V2_0_0: Self = Self { major: 2, minor: 0 };

    /// Check if this version is compatible with another version
    /// A version is compatible if it has the same major version and a minor version >= the other
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
    Snappy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Query,
    Response,
    Ping,
    Pong,
    Handshake,
    Error,
}

#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub message_type: MessageType,
    pub length: u32,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new packet with the given message type and payload
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        let length = payload.len() as u32;
        Self {
            header: PacketHeader {
                message_type,
                length,
                sequence: 0, // Will be set by the protocol handler
            },
            payload,
        }
    }

    /// Create a new packet with a specific sequence number
    pub fn with_sequence(message_type: MessageType, payload: Vec<u8>, sequence: u64) -> Self {
        let length = payload.len() as u32;
        Self {
            header: PacketHeader {
                message_type,
                length,
                sequence,
            },
            payload,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolCapabilities {
    pub compression: Vec<CompressionType>,
    pub max_message_size: usize,
}

impl Default for ProtocolCapabilities {
    fn default() -> Self {
        Self {
            compression: vec![CompressionType::None, CompressionType::Lz4],
            max_message_size: 16 * 1024 * 1024, // 16 MB default
        }
    }
}

#[derive(Debug, Clone)]
pub struct NegotiatedProtocol {
    pub version: ProtocolVersion,
    pub compression: CompressionType,
}

// ============================================================================
// Streaming Types
// ============================================================================

pub struct StreamingResultSet {
    pub(crate) chunks: Vec<StreamChunk>,
}

impl StreamingResultSet {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn add_chunk(&mut self, chunk: StreamChunk) {
        self.chunks.push(chunk);
    }

    pub fn chunks(&self) -> &[StreamChunk] {
        &self.chunks
    }
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub data: Vec<u8>,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct StreamStats {
    pub chunks_sent: u64,
    pub bytes_sent: u64,
}
