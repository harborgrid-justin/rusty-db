// Message encoding and decoding for the wire protocol
//
// This module provides codecs for serializing and deserializing messages
// using the RustyDB P2P protocol format.

use crate::error::{DbError, Result};
use crate::networking::protocol::{CompressionType, Message, MessageHeader, MAX_MESSAGE_SIZE};
use bytes::{Buf, BufMut, BytesMut};
use crc32fast::Hasher;

/// Message codec for encoding and decoding messages
pub struct MessageCodec {
    /// Maximum allowed message size
    max_message_size: usize,

    /// Whether to enable compression for outgoing messages
    enable_compression: bool,

    /// Compression type to use
    compression_type: CompressionType,
}

impl MessageCodec {
    /// Create a new message codec
    pub fn new() -> Self {
        Self {
            max_message_size: MAX_MESSAGE_SIZE,
            enable_compression: false,
            compression_type: CompressionType::None,
        }
    }

    /// Set maximum message size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_message_size = max_size;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.enable_compression = compression != CompressionType::None;
        self.compression_type = compression;
        self
    }

    /// Encode a message to bytes
    pub fn encode(&self, message_id: u64, message: &Message) -> Result<BytesMut> {
        // Serialize message using bincode
        let payload = bincode::encode_to_vec(message, bincode::config::standard())
            .map_err(|e| DbError::Serialization(format!("Failed to serialize message: {}", e)))?;

        // Apply compression if enabled
        let (final_payload, compression_type) = if self.enable_compression {
            let compressed = self.compress(&payload)?;
            if compressed.len() < payload.len() {
                (compressed, self.compression_type)
            } else {
                // Compression not effective, use original
                (payload, CompressionType::None)
            }
        } else {
            (payload, CompressionType::None)
        };

        // Create header
        let mut header = MessageHeader::new(message_id, final_payload.len());
        header.flags.compression = compression_type;

        // Calculate checksum if enabled
        let checksum = if header.flags.has_checksum {
            let mut hasher = Hasher::new();
            hasher.update(&final_payload);
            hasher.finalize()
        } else {
            0
        };

        // Build final message
        let total_size = MessageHeader::SIZE
            + final_payload.len()
            + if header.flags.has_checksum { 4 } else { 0 };

        let mut buf = BytesMut::with_capacity(total_size);

        // Write header
        buf.put_slice(&header.encode());

        // Write payload
        buf.put_slice(&final_payload);

        // Write checksum if enabled
        if header.flags.has_checksum {
            buf.put_u32(checksum);
        }

        Ok(buf)
    }

    /// Decode a message from bytes
    pub fn decode(&self, mut buf: BytesMut) -> Result<(u64, Message)> {
        // Check minimum size
        if buf.len() < MessageHeader::SIZE {
            return Err(DbError::Network("Incomplete message header".to_string()));
        }

        // Parse header
        let header = MessageHeader::decode(&buf[..MessageHeader::SIZE])
            .map_err(|e| DbError::Network(format!("Failed to decode header: {}", e)))?;

        // Validate message size
        if header.length as usize > self.max_message_size {
            return Err(DbError::Network(format!(
                "Message too large: {} bytes (max: {})",
                header.length, self.max_message_size
            )));
        }

        // Check if we have the complete message
        let total_size = MessageHeader::SIZE + header.length as usize;
        if buf.len() < total_size {
            return Err(DbError::Network("Incomplete message".to_string()));
        }

        // Advance past header
        buf.advance(MessageHeader::SIZE);

        // Calculate payload size
        let payload_size = header.length as usize - if header.flags.has_checksum { 4 } else { 0 };

        // Extract payload
        let payload = buf.split_to(payload_size);

        // Verify checksum if present
        if header.flags.has_checksum {
            let received_checksum = buf.get_u32();
            let mut hasher = Hasher::new();
            hasher.update(&payload);
            let computed_checksum = hasher.finalize();

            if received_checksum != computed_checksum {
                return Err(DbError::Network(format!(
                    "Checksum mismatch: expected {}, got {}",
                    computed_checksum, received_checksum
                )));
            }
        }

        // Decompress if needed
        let final_payload = if header.flags.compression != CompressionType::None {
            self.decompress(&payload, header.flags.compression)?
        } else {
            payload.to_vec()
        };

        // Deserialize message
        let message: Message =
            bincode::decode_from_slice(&final_payload, bincode::config::standard())
                .map(|(msg, _)| msg)
                .map_err(|e| {
                    DbError::Serialization(format!("Failed to deserialize message: {}", e))
                })?;

        Ok((header.message_id, message))
    }

    /// Compress data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Lz4 => {
                // LZ4 compression would go here
                // For now, just return uncompressed
                tracing::warn!("LZ4 compression not yet implemented");
                Ok(data.to_vec())
            }
            CompressionType::Zstd => {
                // Zstd compression would go here
                // For now, just return uncompressed
                tracing::warn!("Zstd compression not yet implemented");
                Ok(data.to_vec())
            }
        }
    }

    /// Decompress data
    fn decompress(&self, data: &[u8], compression: CompressionType) -> Result<Vec<u8>> {
        match compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Lz4 => {
                // LZ4 decompression would go here
                tracing::warn!("LZ4 decompression not yet implemented");
                Ok(data.to_vec())
            }
            CompressionType::Zstd => {
                // Zstd decompression would go here
                tracing::warn!("Zstd decompression not yet implemented");
                Ok(data.to_vec())
            }
        }
    }
}

impl Default for MessageCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol codec for use with tokio codec framework
pub struct ProtocolCodec {
    codec: MessageCodec,
    current_message_id: u64,
}

impl ProtocolCodec {
    /// Create a new protocol codec
    pub fn new() -> Self {
        Self {
            codec: MessageCodec::new(),
            current_message_id: 0,
        }
    }

    /// Create with custom message codec
    pub fn with_codec(codec: MessageCodec) -> Self {
        Self {
            codec,
            current_message_id: 0,
        }
    }

    /// Get next message ID
    pub fn next_message_id(&mut self) -> u64 {
        let id = self.current_message_id;
        self.current_message_id = self.current_message_id.wrapping_add(1);
        id
    }

    /// Encode a message
    pub fn encode_message(&mut self, message: &Message) -> Result<BytesMut> {
        let message_id = self.next_message_id();
        self.codec.encode(message_id, message)
    }

    /// Decode a message
    pub fn decode_message(&self, buf: BytesMut) -> Result<(u64, Message)> {
        self.codec.decode(buf)
    }
}

impl Default for ProtocolCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::protocol::handshake::HandshakeRequest;

    #[test]
    fn test_message_codec_encode_decode() {
        let codec = MessageCodec::new();
        let message = Message::Ping { timestamp: 12345 };

        // Encode
        let encoded = codec.encode(1, &message).unwrap();
        assert!(encoded.len() > MessageHeader::SIZE);

        // Decode
        let (message_id, decoded) = codec.decode(encoded).unwrap();
        assert_eq!(message_id, 1);

        match decoded {
            Message::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong message type decoded"),
        }
    }

    #[test]
    fn test_message_codec_with_checksum() {
        let codec = MessageCodec::new();
        let message = Message::Pong { timestamp: 67890 };

        let encoded = codec.encode(2, &message).unwrap();
        let (message_id, decoded) = codec.decode(encoded).unwrap();

        assert_eq!(message_id, 2);
        match decoded {
            Message::Pong { timestamp } => assert_eq!(timestamp, 67890),
            _ => panic!("Wrong message type decoded"),
        }
    }

    #[test]
    fn test_message_codec_error_message() {
        let codec = MessageCodec::new();
        let message = Message::Error {
            error_code: 404,
            message: "Not found".to_string(),
        };

        let encoded = codec.encode(3, &message).unwrap();
        let (message_id, decoded) = codec.decode(encoded).unwrap();

        assert_eq!(message_id, 3);
        match decoded {
            Message::Error {
                error_code,
                message,
            } => {
                assert_eq!(error_code, 404);
                assert_eq!(message, "Not found");
            }
            _ => panic!("Wrong message type decoded"),
        }
    }

    #[test]
    fn test_protocol_codec_message_ids() {
        let mut codec = ProtocolCodec::new();

        assert_eq!(codec.next_message_id(), 0);
        assert_eq!(codec.next_message_id(), 1);
        assert_eq!(codec.next_message_id(), 2);
    }

    #[test]
    fn test_protocol_codec_encode_decode() {
        let mut codec = ProtocolCodec::new();
        let message = Message::Ping { timestamp: 99999 };

        let encoded = codec.encode_message(&message).unwrap();
        let (message_id, decoded) = codec.decode_message(encoded).unwrap();

        assert_eq!(message_id, 0);
        match decoded {
            Message::Ping { timestamp } => assert_eq!(timestamp, 99999),
            _ => panic!("Wrong message type decoded"),
        }
    }

    #[test]
    fn test_message_too_large() {
        let codec = MessageCodec::new().with_max_size(100);

        // Create a large message
        let large_data = vec![0u8; 1000];
        let message = Message::DataTransfer {
            transfer_id: 1,
            chunk_index: 0,
            total_chunks: 1,
            data: large_data,
        };

        let encoded = codec.encode(1, &message).unwrap();

        // Should fail when decoding due to size limit
        let result = codec.decode(encoded);
        assert!(result.is_err());
    }
}
