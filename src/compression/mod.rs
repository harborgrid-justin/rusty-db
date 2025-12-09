// Compression Module - Oracle-like HCC and Advanced Compression Engine
// Provides enterprise-grade compression capabilities for RustyDB

pub mod algorithms;
pub mod hcc;
pub mod oltp;
pub mod dedup;
pub mod tiered;

use std::sync::Arc;
use std::error::Error;
use std::fmt;


/// Compression level from 0 (none) to 9 (maximum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionLevel {
    None = 0,
    Fast = 1,
    Default = 6,
    Maximum = 9,
}

impl From<u8> for CompressionLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => CompressionLevel::None,
            1..=3 => CompressionLevel::Fast,
            4..=7 => CompressionLevel::Default,
            _ => CompressionLevel::Maximum,
        }
    }
}

/// Compression algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    None,
    LZ4,
    Zstandard,
    Dictionary,
    Arithmetic,
    Huffman,
    Adaptive,
    HCC,
}

impl fmt::Display for CompressionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionAlgorithm::None => write!(f, "None"),
            CompressionAlgorithm::LZ4 => write!(f, "LZ4"),
            CompressionAlgorithm::Zstandard => write!(f, "Zstandard"),
            CompressionAlgorithm::Dictionary => write!(f, "Dictionary"),
            CompressionAlgorithm::Arithmetic => write!(f, "Arithmetic"),
            CompressionAlgorithm::Huffman => write!(f, "Huffman"),
            CompressionAlgorithm::Adaptive => write!(f, "Adaptive"),
            CompressionAlgorithm::HCC => write!(f, "HCC"),
        }
    }
}

/// Compression error types
#[derive(Debug, Clone)]
pub enum CompressionError {
    InvalidInput(String),
    CompressionFailed(String),
    DecompressionFailed(String),
    BufferTooSmall(usize, usize), // (required, available)
    UnsupportedAlgorithm(String),
    CorruptedData(String),
    InvalidMetadata(String),
    ResourceExhausted(String),
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CompressionError::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            CompressionError::DecompressionFailed(msg) => write!(f, "Decompression failed: {}", msg),
            CompressionError::BufferTooSmall(required, available) => {
                write!(f, "Buffer too small: need {} bytes, have {}", required, available)
            }
            CompressionError::UnsupportedAlgorithm(algo) => {
                write!(f, "Unsupported algorithm: {}", algo)
            }
            CompressionError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
            CompressionError::InvalidMetadata(msg) => write!(f, "Invalid metadata: {}", msg),
            CompressionError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
        }
    }
}

impl Error for CompressionError {}

pub type CompressionResult<T> = Result<T, CompressionError>;

/// Compression statistics
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub compression_time_us: u64,
    pub decompression_time_us: u64,
    pub algorithm: Option<CompressionAlgorithm>,
    pub level: Option<CompressionLevel>,
    pub blocks_compressed: usize,
    pub blocks_decompressed: usize,
}

impl CompressionStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size == 0 {
            0.0
        } else {
            self.uncompressed_size as f64 / self.compressed_size.max(1) as f64
        }
    }

    pub fn space_savings_percent(&self) -> f64 {
        if self.uncompressed_size == 0 {
            0.0
        } else {
            100.0 * (1.0 - (self.compressed_size as f64 / self.uncompressed_size as f64))
        }
    }

    pub fn compression_throughput_mbps(&self) -> f64 {
        if self.compression_time_us == 0 {
            0.0
        } else {
            (self.uncompressed_size as f64 / 1_048_576.0) / (self.compression_time_us as f64 / 1_000_000.0)
        }
    }

    pub fn decompression_throughput_mbps(&self) -> f64 {
        if self.decompression_time_us == 0 {
            0.0
        } else {
            (self.uncompressed_size as f64 / 1_048_576.0) / (self.decompression_time_us as f64 / 1_000_000.0)
        }
    }
}

/// Core compression trait - all compressors must implement this
pub trait Compressor: Send + Sync {
    /// Compress data from input buffer into output buffer
    /// Returns the number of bytes written to output
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize>;

    /// Decompress data from input buffer into output buffer
    /// Returns the number of bytes written to output
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize>;

    /// Get the maximum compressed size for given input size
    fn max_compressed_size(&self, input_size: usize) -> usize;

    /// Get the algorithm identifier
    fn algorithm(&self) -> CompressionAlgorithm;

    /// Get the compression level
    fn level(&self) -> CompressionLevel;

    /// Get statistics
    fn stats(&self) -> CompressionStats;

    /// Reset statistics
    fn reset_stats(&mut self);
}

/// Streaming compression trait for large data
pub trait StreamingCompressor: Send + Sync {
    /// Initialize a new compression stream
    fn init_stream(&mut self) -> CompressionResult<()>;

    /// Compress a chunk of data in streaming mode
    fn compress_chunk(&mut self, input: &[u8], output: &mut Vec<u8>) -> CompressionResult<usize>;

    /// Finalize the compression stream
    fn finalize_stream(&mut self, output: &mut Vec<u8>) -> CompressionResult<usize>;

    /// Initialize a new decompression stream
    fn init_decompress_stream(&mut self) -> CompressionResult<()>;

    /// Decompress a chunk of data in streaming mode
    fn decompress_chunk(&mut self, input: &[u8], output: &mut Vec<u8>) -> CompressionResult<usize>;

    /// Finalize the decompression stream
    fn finalize_decompress_stream(&mut self, output: &mut Vec<u8>) -> CompressionResult<usize>;
}

/// Columnar compression trait for HCC
pub trait ColumnarCompressor: Send + Sync {
    /// Transform row-major data to column-major format
    fn transform_to_columnar(&self, rows: &[Vec<u8>], num_columns: usize) -> CompressionResult<Vec<Vec<u8>>>;

    /// Transform column-major data back to row-major format
    fn transform_to_rows(&self, columns: &[Vec<u8>], num_rows: usize) -> CompressionResult<Vec<Vec<u8>>>;

    /// Compress a single column
    fn compress_column(&self, column: &[u8], output: &mut [u8]) -> CompressionResult<usize>;

    /// Decompress a single column
    fn decompress_column(&self, compressed: &[u8], output: &mut [u8]) -> CompressionResult<usize>;

    /// Compress all columns in a Compression Unit
    fn compress_cu(&self, columns: &[Vec<u8>]) -> CompressionResult<Vec<Vec<u8>>>;

    /// Decompress all columns in a Compression Unit
    fn decompress_cu(&self, compressed_columns: &[Vec<u8>]) -> CompressionResult<Vec<Vec<u8>>>;
}

/// Deduplication trait
pub trait Deduplicator: Send + Sync {
    /// Compute hash for content-defined chunking
    fn compute_chunk_hash(&self, data: &[u8]) -> u64;

    /// Find chunk boundaries using content-defined chunking
    fn find_chunk_boundaries(&self, data: &[u8]) -> Vec<usize>;

    /// Detect if a chunk is a duplicate
    fn is_duplicate(&self, chunk_hash: u64) -> bool;

    /// Store a chunk and return its identifier
    fn store_chunk(&mut self, chunk: &[u8], hash: u64) -> CompressionResult<Vec<u8>>;

    /// Retrieve a chunk by its identifier
    fn retrieve_chunk(&self, chunk_id: &[u8]) -> CompressionResult<Vec<u8>>;

    /// Get deduplication ratio
    fn dedup_ratio(&self) -> f64;
}

/// Temperature-based data classification for tiered compression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTemperature {
    Hot,    // Frequently accessed, low compression
    Warm,   // Moderately accessed, medium compression
    Cold,   // Rarely accessed, high compression
    Frozen, // Archive, maximum compression
}

impl DataTemperature {
    pub fn recommended_compression_level(&self) -> CompressionLevel {
        match self {
            DataTemperature::Hot => CompressionLevel::Fast,
            DataTemperature::Warm => CompressionLevel::Default,
            DataTemperature::Cold => CompressionLevel::Maximum,
            DataTemperature::Frozen => CompressionLevel::Maximum,
        }
    }

    pub fn recommended_algorithm(&self) -> CompressionAlgorithm {
        match self {
            DataTemperature::Hot => CompressionAlgorithm::LZ4,
            DataTemperature::Warm => CompressionAlgorithm::Zstandard,
            DataTemperature::Cold => CompressionAlgorithm::HCC,
            DataTemperature::Frozen => CompressionAlgorithm::Arithmetic,
        }
    }
}

/// Tiered compression manager trait
pub trait TieredCompressionManager: Send + Sync {
    /// Classify data temperature based on access patterns
    fn classify_temperature(&self, block_id: u64) -> DataTemperature;

    /// Update access pattern for a block
    fn record_access(&mut self, block_id: u64);

    /// Migrate block to appropriate tier
    fn migrate_block(&mut self, block_id: u64, new_temp: DataTemperature) -> CompressionResult<()>;

    /// Get compression recommendation for a block
    fn get_compression_recommendation(&self, block_id: u64) -> (CompressionAlgorithm, CompressionLevel);

    /// Get tier statistics
    fn tier_stats(&self) -> TierStats;
}

/// Statistics for compression tiers
#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub hot_blocks: usize,
    pub warm_blocks: usize,
    pub cold_blocks: usize,
    pub frozen_blocks: usize,
    pub migrations_performed: usize,
    pub total_space_saved: usize,
}

/// Compression context - maintains state for compression operations
pub struct CompressionContext {
    pub algorithm: CompressionAlgorithm,
    pub level: CompressionLevel,
    pub stats: CompressionStats,
    pub dictionary: Option<Arc<Vec<u8>>>,
    pub enable_parallel: bool,
    pub chunk_size: usize,
}

impl CompressionContext {
    pub fn new(algorithm: CompressionAlgorithm, level: CompressionLevel) -> Self {
        Self {
            algorithm,
            level,
            stats: CompressionStats::new(),
            dictionary: None,
            enable_parallel: true,
            chunk_size: 64 * 1024, // 64KB default
        }
    }

    pub fn with_dictionary(mut self, dictionary: Vec<u8>) -> Self {
        self.dictionary = Some(Arc::new(dictionary));
        self
    }

    pub fn with_parallel(mut self, enable: bool) -> Self {
        self.enable_parallel = enable;
        self
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}

/// Compression metadata stored with compressed data
#[derive(Debug, Clone)]
pub struct CompressionMetadata {
    pub algorithm: CompressionAlgorithm,
    pub level: CompressionLevel,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub checksum: u32,
    pub version: u8,
    pub flags: u32,
}

impl CompressionMetadata {
    pub fn new(algorithm: CompressionAlgorithm, level: CompressionLevel,
               uncompressed_size: usize, compressed_size: usize) -> Self {
        Self {
            algorithm,
            level,
            uncompressed_size,
            compressed_size,
            checksum: 0,
            version: 1,
            flags: 0,
        }
    }

    /// Serialize metadata to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.push(self.version);
        bytes.push(self.algorithm as u8);
        bytes.push(self.level as u8);
        bytes.push(0); // padding
        bytes.extend_from_slice(&(self.uncompressed_size as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.compressed_size as u64).to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes
    }

    /// Deserialize metadata from bytes
    pub fn from_bytes(bytes: &[u8]) -> CompressionResult<Self> {
        if bytes.len() < 32 {
            return Err(CompressionError::InvalidMetadata(
                format!("Metadata too short: {} bytes", bytes.len())
            ))));
        }

        let version = bytes[0];
        if version != 1 {
            return Err(CompressionError::InvalidMetadata(
                format!("Unsupported metadata version: {}", version)
            ))));
        }

        let algorithm = match bytes[1] {
            0 => CompressionAlgorithm::None,
            1 => CompressionAlgorithm::LZ4,
            2 => CompressionAlgorithm::Zstandard,
            3 => CompressionAlgorithm::Dictionary,
            4 => CompressionAlgorithm::Arithmetic,
            5 => CompressionAlgorithm::Huffman,
            6 => CompressionAlgorithm::Adaptive,
            7 => CompressionAlgorithm::HCC,
            _ => return Err(CompressionError::InvalidMetadata(
                format!("Unknown algorithm: {}", bytes[1])
            )),
        }));

        let level = CompressionLevel::from(bytes[2]);

        let uncompressed_size = u64::from_le_bytes(bytes[4..12].try_into().unwrap()) as usize;
        let compressed_size = u64::from_le_bytes(bytes[12..20].try_into().unwrap()) as usize;
        let checksum = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        let flags = u32::from_le_bytes(bytes[24..28].try_into().unwrap());

        Ok(Self {
            algorithm,
            level,
            uncompressed_size,
            compressed_size,
            checksum,
            version,
            flags,
        })
    }
}

/// Utility functions for compression
pub mod utils {
    use super::*;

    /// Calculate CRC32 checksum
    pub fn crc32(data: &[u8]) -> u32 {
        const CRC32_TABLE: [u32; 256] = generate_crc32_table();

        let mut crc = 0xFFFFFFFF_u32;
        for &byte in data {
            let index = ((crc ^ byte as u32) & 0xFF) as usize;
            crc = (crc >> 8) ^ CRC32_TABLE[index];
        }
        !crc
    }

    const fn generate_crc32_table() -> [u32; 256] {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut crc = i as u32;
            let mut j = 0;
            while j < 8 {
                if crc & 1 == 1 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
                j += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    }

    /// Calculate Adler32 checksum (faster than CRC32)
    pub fn adler32(data: &[u8]) -> u32 {
        const MOD_ADLER: u32 = 65521;
        let mut a = 1u32;
        let mut b = 0u32;

        for &byte in data {
            a = (a + byte as u32) % MOD_ADLER;
            b = (b + a) % MOD_ADLER;
        }

        (b << 16) | a
    }

    /// Estimate compression ratio for data
    pub fn estimate_compressibility(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 1.0;
        }

        // Simple entropy-based estimation
        let mut histogram = [0u32; 256];
        for &byte in data {
            histogram[byte as usize] += 1;
        }

        let mut entropy = 0.0;
        let len = data.len() as f64;
        for &count in &histogram {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        // Estimate compression ratio based on entropy
        // Lower entropy = better compression
        let max_entropy = 8.0; // Maximum entropy for 8-bit data
        1.0 + (max_entropy - entropy) / max_entropy * 3.0
    }

    /// Find the best compression algorithm for given data
    pub fn select_best_algorithm(data: &[u8]) -> CompressionAlgorithm {
        let compressibility = estimate_compressibility(data);

        if compressibility < 1.2 {
            // Poor compression - use fast algorithm
            CompressionAlgorithm::LZ4
        } else if compressibility < 2.0 {
            // Moderate compression
            CompressionAlgorithm::Zstandard
        } else if compressibility < 3.0 {
            // Good compression
            CompressionAlgorithm::Dictionary
        } else {
            // Excellent compression potential
            CompressionAlgorithm::Arithmetic
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_compression_stats() {
        let mut stats = CompressionStats::new();
        stats.uncompressed_size = 10000;
        stats.compressed_size = 2500;
        stats.compression_time_us = 1000;

        assert_eq!(stats.compression_ratio(), 4.0);
        assert_eq!(stats.space_savings_percent(), 75.0);
    }

    #[test]
    fn test_compression_metadata() {
        let metadata = CompressionMetadata::new(
            CompressionAlgorithm::LZ4,
            CompressionLevel::Default,
            10000,
            2500,
        );

        let bytes = metadata.to_bytes();
        let restored = CompressionMetadata::from_bytes(&bytes).unwrap();

        assert_eq!(restored.algorithm as u8, metadata.algorithm as u8);
        assert_eq!(restored.uncompressed_size, metadata.uncompressed_size);
        assert_eq!(restored.compressed_size, metadata.compressed_size);
    }

    #[test]
    fn test_crc32() {
        let data = b"Hello, World!";
        let crc = utils::crc32(data);
        assert_ne!(crc, 0);

        // Same data should produce same checksum
        let crc2 = utils::crc32(data);
        assert_eq!(crc, crc2);
    }

    #[test]
    fn test_adler32() {
        let data = b"Hello, World!";
        let checksum = utils::adler32(data);
        assert_ne!(checksum, 0);

        // Same data should produce same checksum
        let checksum2 = utils::adler32(data);
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_data_temperature_recommendations() {
        assert_eq!(
            DataTemperature::Hot.recommended_algorithm() as u8,
            CompressionAlgorithm::LZ4 as u8
        );
        assert_eq!(
            DataTemperature::Cold.recommended_algorithm() as u8,
            CompressionAlgorithm::HCC as u8
        );
    }
}


