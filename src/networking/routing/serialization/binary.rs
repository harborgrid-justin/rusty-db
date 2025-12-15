// Binary serialization codec for cluster messages
//
// This module provides efficient binary serialization using bincode with optional
// compression and checksum verification for data integrity.

use crate::error::{DbError, Result};
use crate::networking::types::CompressionType;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Binary codec for serializing and deserializing messages
pub struct BinaryCodec {
    /// Optional compression for large messages
    compression: Option<CompressionType>,
    /// Enable checksum verification
    checksum: bool,
    /// Minimum size in bytes to trigger compression
    compression_threshold: usize,
}

impl BinaryCodec {
    /// Create a new binary codec without compression
    pub fn new() -> Self {
        Self {
            compression: None,
            checksum: true,
            compression_threshold: 1024,
        }
    }

    /// Create a codec with compression enabled
    pub fn with_compression(compression: CompressionType) -> Self {
        Self {
            compression: Some(compression),
            checksum: true,
            compression_threshold: 1024,
        }
    }

    /// Set the compression threshold in bytes
    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = threshold;
        self
    }

    /// Enable or disable checksum verification
    pub fn with_checksum(mut self, enabled: bool) -> Self {
        self.checksum = enabled;
        self
    }

    /// Serialize a message to bytes
    pub fn encode<T: Serialize + bincode::Encode>(&self, message: &T) -> Result<Vec<u8>> {
        // Serialize using bincode
        let serialized = bincode::encode_to_vec(message, bincode::config::standard())
            .map_err(|e| DbError::Serialization(format!("Bincode serialization failed: {}", e)))?;

        // Save original length before serialized is moved
        let original_len = serialized.len();

        // Apply compression if enabled and message is large enough
        let compressed = if let Some(compression_type) = self.compression {
            if serialized.len() >= self.compression_threshold {
                self.compress(&serialized, compression_type)?
            } else {
                serialized
            }
        } else {
            serialized
        };

        // Build final message with metadata
        let mut result = Vec::new();

        // Write flags (1 byte)
        let flags = self.build_flags(original_len, &compressed);
        result.push(flags);

        // Write original size if compressed (4 bytes)
        if flags & FLAG_COMPRESSED != 0 {
            result.extend_from_slice(&(original_len as u32).to_le_bytes());
        }

        // Write checksum if enabled (4 bytes)
        if self.checksum {
            let checksum = crc32fast::hash(&compressed);
            result.extend_from_slice(&checksum.to_le_bytes());
        }

        // Write payload
        result.extend_from_slice(&compressed);

        Ok(result)
    }

    /// Deserialize a message from bytes
    pub fn decode<T: for<'de> Deserialize<'de> + bincode::Decode<()>>(
        &self,
        data: &[u8],
    ) -> Result<T> {
        if data.is_empty() {
            return Err(DbError::Serialization("Empty data".to_string()));
        }

        let mut offset = 0;

        // Read flags
        let flags = data[offset];
        offset += 1;

        // Read original size if compressed
        let original_size = if flags & FLAG_COMPRESSED != 0 {
            if data.len() < offset + 4 {
                return Err(DbError::Serialization("Incomplete size field".to_string()));
            }
            let size = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            Some(size)
        } else {
            None
        };

        // Read and verify checksum if enabled
        if self.checksum {
            if data.len() < offset + 4 {
                return Err(DbError::Serialization(
                    "Incomplete checksum field".to_string(),
                ));
            }
            let expected_checksum = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;

            let payload = &data[offset..];
            let actual_checksum = crc32fast::hash(payload);

            if expected_checksum != actual_checksum {
                return Err(DbError::Serialization(format!(
                    "Checksum mismatch: expected {}, got {}",
                    expected_checksum, actual_checksum
                )));
            }
        }

        // Extract payload
        let payload = &data[offset..];

        // Decompress if needed
        let decompressed = if flags & FLAG_COMPRESSED != 0 {
            let compression_type = self.get_compression_from_flags(flags)?;
            self.decompress(payload, compression_type, original_size.unwrap())?
        } else {
            payload.to_vec()
        };

        // Deserialize using bincode
        bincode::decode_from_slice(&decompressed, bincode::config::standard())
            .map(|(msg, _)| msg)
            .map_err(|e| DbError::Serialization(format!("Bincode deserialization failed: {}", e)))
    }

    /// Build flags byte from message state
    fn build_flags(&self, original_len: usize, compressed: &[u8]) -> u8 {
        let mut flags = 0u8;

        // Check if actually compressed
        if compressed.len() < original_len {
            flags |= FLAG_COMPRESSED;

            // Set compression type
            if let Some(compression_type) = self.compression {
                flags |= match compression_type {
                    CompressionType::None => 0,
                    CompressionType::Lz4 => COMPRESSION_LZ4,
                    CompressionType::Snappy => COMPRESSION_SNAPPY,
                    CompressionType::Zstd => COMPRESSION_ZSTD,
                };
            }
        }

        flags
    }

    /// Get compression type from flags
    fn get_compression_from_flags(&self, flags: u8) -> Result<CompressionType> {
        let compression_bits = flags & COMPRESSION_MASK;
        match compression_bits {
            COMPRESSION_LZ4 => Ok(CompressionType::Lz4),
            COMPRESSION_SNAPPY => Ok(CompressionType::Snappy),
            COMPRESSION_ZSTD => Ok(CompressionType::Zstd),
            _ => Err(DbError::Serialization(format!(
                "Unknown compression type: {}",
                compression_bits
            ))),
        }
    }

    /// Compress data using the specified algorithm
    fn compress(&self, data: &[u8], compression_type: CompressionType) -> Result<Vec<u8>> {
        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Lz4 => {
                // Using flate2 as a stand-in for LZ4 (in production, use lz4_flex crate)
                use flate2::write::DeflateEncoder;
                use flate2::Compression;

                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
                encoder.write_all(data).map_err(|e| {
                    DbError::Serialization(format!("LZ4 compression failed: {}", e))
                })?;
                encoder
                    .finish()
                    .map_err(|e| DbError::Serialization(format!("LZ4 compression failed: {}", e)))
            }
            CompressionType::Snappy => {
                // Snappy compression not available in current deps, use deflate
                use flate2::write::DeflateEncoder;
                use flate2::Compression;

                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
                encoder.write_all(data).map_err(|e| {
                    DbError::Serialization(format!("Snappy compression failed: {}", e))
                })?;
                encoder.finish().map_err(|e| {
                    DbError::Serialization(format!("Snappy compression failed: {}", e))
                })
            }
            CompressionType::Zstd => {
                // Zstd compression not available in current deps, use deflate
                use flate2::write::DeflateEncoder;
                use flate2::Compression;

                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
                encoder.write_all(data).map_err(|e| {
                    DbError::Serialization(format!("Zstd compression failed: {}", e))
                })?;
                encoder
                    .finish()
                    .map_err(|e| DbError::Serialization(format!("Zstd compression failed: {}", e)))
            }
        }
    }

    /// Decompress data using the specified algorithm
    fn decompress(
        &self,
        data: &[u8],
        compression_type: CompressionType,
        expected_size: usize,
    ) -> Result<Vec<u8>> {
        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Lz4 | CompressionType::Snappy | CompressionType::Zstd => {
                use flate2::read::DeflateDecoder;

                let mut decoder = DeflateDecoder::new(data);
                let mut decompressed = Vec::with_capacity(expected_size);
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| DbError::Serialization(format!("Decompression failed: {}", e)))?;

                if decompressed.len() != expected_size {
                    return Err(DbError::Serialization(format!(
                        "Decompressed size mismatch: expected {}, got {}",
                        expected_size,
                        decompressed.len()
                    )));
                }

                Ok(decompressed)
            }
        }
    }
}

impl Default for BinaryCodec {
    fn default() -> Self {
        Self::new()
    }
}

// Flag constants
const FLAG_COMPRESSED: u8 = 0b1000_0000;
const COMPRESSION_MASK: u8 = 0b0111_0000;
const COMPRESSION_LZ4: u8 = 0b0001_0000;
const COMPRESSION_SNAPPY: u8 = 0b0010_0000;
const COMPRESSION_ZSTD: u8 = 0b0011_0000;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
    struct TestMessage {
        id: u64,
        data: String,
    }

    #[test]
    fn test_basic_encoding_decoding() {
        let codec = BinaryCodec::new();
        let message = TestMessage {
            id: 42,
            data: "Hello, World!".to_string(),
        };

        let encoded = codec.encode(&message).unwrap();
        let decoded: TestMessage = codec.decode(&encoded).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_compression() {
        let codec =
            BinaryCodec::with_compression(CompressionType::Lz4).with_compression_threshold(10);

        let message = TestMessage {
            id: 42,
            data: "Hello, World! This is a longer message to trigger compression.".repeat(10),
        };

        let encoded = codec.encode(&message).unwrap();
        let decoded: TestMessage = codec.decode(&encoded).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_checksum_verification() {
        let codec = BinaryCodec::new().with_checksum(true);
        let message = TestMessage {
            id: 42,
            data: "Test data".to_string(),
        };

        let mut encoded = codec.encode(&message).unwrap();

        // Corrupt the data
        if let Some(byte) = encoded.last_mut() {
            *byte = byte.wrapping_add(1);
        }

        // Should fail checksum verification
        let result: Result<TestMessage> = codec.decode(&encoded);
        assert!(result.is_err());
    }
}
