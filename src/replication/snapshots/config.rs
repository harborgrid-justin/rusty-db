// Snapshot configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Snapshot configuration
///
/// Comprehensive configuration for snapshot operations including
/// storage, compression, encryption, and retention policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Base storage path for snapshots
    pub storage_path: PathBuf,
    /// Default compression algorithm
    pub compression: CompressionType,
    /// Encryption configuration
    pub encryption: Option<EncryptionConfig>,
    /// Snapshot retention policy
    pub retention_policy: RetentionPolicy,
    /// Maximum parallel operations
    pub max_parallel_operations: usize,
    /// Chunk size for large files
    pub chunk_size: usize,
    /// Enable integrity verification
    pub verify_integrity: bool,
    /// Buffer size for I/O operations
    pub buffer_size: usize,
    /// Enable deduplication
    pub enable_deduplication: bool,
    /// Metadata cache size
    pub metadata_cache_size: usize,
    /// Temporary directory for staging
    pub temp_directory: PathBuf,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Progress reporting interval
    pub progress_interval: Duration,
    /// Enable background cleanup
    pub enable_background_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("/data/snapshots"),
            compression: CompressionType::Lz4,
            encryption: None,
            retention_policy: RetentionPolicy::default(),
            max_parallel_operations: 4,
            chunk_size: 1024 * 1024 * 8, // 8MB
            verify_integrity: true,
            buffer_size: 1024 * 64, // 64KB
            enable_deduplication: false,
            metadata_cache_size: 10000,
            temp_directory: PathBuf::from("/tmp/snapshots"),
            enable_statistics: true,
            progress_interval: Duration::from_secs(10),
            enable_background_cleanup: true,
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Compression algorithms supported
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Fast compression with LZ4
    Lz4,
    /// Balanced compression with Zstandard
    Zstd,
    /// High compression with LZMA
    Lzma,
    /// Gzip compression
    Gzip,
    /// Brotli compression
    Brotli,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation configuration
    pub key_source: KeySource,
    /// Additional authenticated data
    pub aad: Option<String>,
}

/// Encryption algorithms supported
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// AES-256-CBC with HMAC
    Aes256CbcHmac,
}

/// Key source for encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeySource {
    /// Key from environment variable
    Environment(String),
    /// Key from file
    File(PathBuf),
    /// Key from key management service
    Kms { service: String, key_id: String },
    /// Inline key (for testing only)
    Inline(Vec<u8>),
}

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum number of snapshots to keep
    pub max_snapshots: usize,
    /// Maximum age of snapshots
    pub max_age: Duration,
    /// Minimum number of full snapshots to keep
    pub min_full_snapshots: usize,
    /// Keep hourly snapshots for this duration
    pub hourly_retention: Option<Duration>,
    /// Keep daily snapshots for this duration
    pub daily_retention: Option<Duration>,
    /// Keep weekly snapshots for this duration
    pub weekly_retention: Option<Duration>,
    /// Keep monthly snapshots for this duration
    pub monthly_retention: Option<Duration>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_snapshots: 50,
            max_age: Duration::from_secs(86400 * 90), // 90 days
            min_full_snapshots: 3,
            hourly_retention: Some(Duration::from_secs(86400 * 7)), // 7 days
            daily_retention: Some(Duration::from_secs(86400 * 30)), // 30 days
            weekly_retention: Some(Duration::from_secs(86400 * 90)), // 90 days
            monthly_retention: Some(Duration::from_secs(86400 * 365)), // 1 year
        }
    }
}
