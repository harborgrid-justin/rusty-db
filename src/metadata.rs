//! Instance Metadata Management for RustyDB
//!
//! This module provides types and utilities for managing instance metadata
//! according to Instance Layout Spec v1.0.
//!
//! # Overview
//!
//! Instance metadata is stored under `<INSTANCE_ROOT>/data/meta/` and includes:
//! - Layout version (instance directory structure version)
//! - Instance ID (stable UUID for telemetry correlation)
//! - Data format version (on-disk format version)
//! - WAL format version
//! - Protocol version (client-server protocol)
//! - Compatibility hints
//!
//! # Example
//!
//! ```rust,no_run
//! use rusty_db::metadata::{InstanceMetadata, MetaPaths, LayoutVersion};
//! use std::path::Path;
//!
//! let instance_root = Path::new("/var/lib/rustydb/instances/prod");
//! let paths = MetaPaths::from_instance_root(instance_root);
//!
//! // Load existing metadata
//! let metadata = InstanceMetadata::load(&paths.meta_dir).unwrap();
//! println!("Instance ID: {}", metadata.instance_id);
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use crate::error::{DbError, Result};

/// RustyDB binary version (0.3.1)
pub const RUSTYDB_VERSION: &str = "0.3.1";

/// Current layout version
pub const CURRENT_LAYOUT_VERSION: &str = "1.0";

/// Current data format version
pub const CURRENT_DATA_FORMAT: u32 = 2;

/// Current WAL format version
pub const CURRENT_WAL_FORMAT: u32 = 2;

/// Current protocol version
pub const CURRENT_PROTOCOL: u32 = 2;

/// Binary version using semver-style versioning.
///
/// Represents the RustyDB binary version in major.minor.patch format.
/// Used for compatibility checking and upgrade tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryVersion {
    /// Major version number (breaking changes)
    pub major: u64,
    /// Minor version number (new features, backward compatible)
    pub minor: u64,
    /// Patch version number (bug fixes)
    pub patch: u64,
}

impl BinaryVersion {
    /// Create a new binary version.
    ///
    /// # Arguments
    /// * `major` - Major version number
    /// * `minor` - Minor version number
    /// * `patch` - Patch version number
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Get the current RustyDB binary version.
    pub fn current() -> Self {
        Self::new(0, 3, 1) // 0.3.1
    }

    /// Parse a version string like "0.3.1".
    ///
    /// # Arguments
    /// * `s` - Version string to parse
    ///
    /// # Returns
    /// * `Ok(BinaryVersion)` - Parsed version
    /// * `Err(DbError)` - Parse error
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 3 {
            return Err(DbError::InvalidInput(format!(
                "Invalid version format: '{}'. Expected 'major.minor.patch'",
                s
            )));
        }

        let major = parts[0].parse::<u64>().map_err(|_| {
            DbError::InvalidInput(format!("Invalid major version: '{}'", parts[0]))
        })?;

        let minor = parts[1].parse::<u64>().map_err(|_| {
            DbError::InvalidInput(format!("Invalid minor version: '{}'", parts[1]))
        })?;

        let patch = parts[2].parse::<u64>().map_err(|_| {
            DbError::InvalidInput(format!("Invalid patch version: '{}'", parts[2]))
        })?;

        Ok(Self::new(major, minor, patch))
    }

    /// Convert to string representation.
    pub fn to_string_repr(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for BinaryVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for BinaryVersion {
    fn default() -> Self {
        Self::current()
    }
}

/// Instance Layout Version (ILV).
///
/// Represents the version of the instance directory structure.
/// Currently supports v1.0 layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutVersion(pub String);

impl LayoutVersion {
    /// Create a new layout version.
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    /// Get the current layout version (1.0).
    pub fn current() -> Self {
        Self(CURRENT_LAYOUT_VERSION.to_string())
    }

    /// Check if this layout version is supported.
    pub fn is_supported(&self) -> bool {
        self.0 == "1.0"
    }

    /// Get the version string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LayoutVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for LayoutVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Data Format Version (DFV).
///
/// Represents the on-disk data format version.
/// Used for compatibility checking when opening databases.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DataFormatVersion(pub u32);

impl DataFormatVersion {
    /// Create a new data format version.
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    /// Get the current data format version.
    pub fn current() -> Self {
        Self(CURRENT_DATA_FORMAT)
    }

    /// Get the version number.
    pub fn version(&self) -> u32 {
        self.0
    }
}

impl Default for DataFormatVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for DataFormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// WAL Format Version (WFV).
///
/// Represents the Write-Ahead Log format version.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WalFormatVersion(pub u32);

impl WalFormatVersion {
    /// Create a new WAL format version.
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    /// Get the current WAL format version.
    pub fn current() -> Self {
        Self(CURRENT_WAL_FORMAT)
    }

    /// Get the version number.
    pub fn version(&self) -> u32 {
        self.0
    }
}

impl Default for WalFormatVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for WalFormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Protocol Version (PV).
///
/// Represents the client-server protocol version.
/// Can be negotiated between CLI and server.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion(pub u32);

impl ProtocolVersion {
    /// Create a new protocol version.
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    /// Get the current protocol version.
    pub fn current() -> Self {
        Self(CURRENT_PROTOCOL)
    }

    /// Get the version number.
    pub fn version(&self) -> u32 {
        self.0
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Compatibility hints for quick decisions and nicer error messages.
///
/// Stored as JSON in the metadata directory to provide
/// guidance on version compatibility without needing to
/// read the full binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatHints {
    /// Minimum binary version that can read this instance
    pub min_binary: Option<BinaryVersion>,
    /// Maximum binary version known to work with this instance
    pub max_binary: Option<BinaryVersion>,
    /// Human-readable notes about compatibility
    pub notes: Option<String>,
}

impl CompatHints {
    /// Create new compatibility hints.
    pub fn new() -> Self {
        Self {
            min_binary: Some(BinaryVersion::new(0, 3, 1)),
            max_binary: None,
            notes: None,
        }
    }

    /// Create hints with a note.
    pub fn with_note(note: impl Into<String>) -> Self {
        Self {
            min_binary: Some(BinaryVersion::new(0, 3, 1)),
            max_binary: None,
            notes: Some(note.into()),
        }
    }
}

impl Default for CompatHints {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable instance identity and layout/format information.
///
/// Stored primarily under `<INSTANCE_ROOT>/data/meta/`.
/// This is the main metadata structure for an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMetadata {
    /// Instance layout version (required)
    pub layout_version: LayoutVersion,

    /// Stable instance identifier (UUIDv4, required)
    pub instance_id: String,

    /// Instance creation timestamp (RFC3339 format, required)
    pub created_at_rfc3339: String,

    /// Data format version (required)
    pub data_format: DataFormatVersion,

    /// WAL format version (optional)
    pub wal_format: Option<WalFormatVersion>,

    /// Protocol version (optional)
    pub protocol: Option<ProtocolVersion>,

    /// Previous binary version if upgraded (optional)
    pub last_upgraded_from: Option<BinaryVersion>,

    /// Engine-specific feature flags (optional)
    pub engine_features: Option<serde_json::Value>,

    /// Compatibility hints (optional)
    pub compat_hints: Option<CompatHints>,
}

impl InstanceMetadata {
    /// Create new instance metadata with a given instance ID.
    ///
    /// # Arguments
    /// * `instance_id` - Unique instance identifier (usually UUIDv4)
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            layout_version: LayoutVersion::current(),
            instance_id: instance_id.into(),
            created_at_rfc3339: chrono::Utc::now().to_rfc3339(),
            data_format: DataFormatVersion::current(),
            wal_format: Some(WalFormatVersion::current()),
            protocol: Some(ProtocolVersion::current()),
            last_upgraded_from: None,
            engine_features: None,
            compat_hints: Some(CompatHints::new()),
        }
    }

    /// Create new instance metadata with auto-generated UUID.
    pub fn new_with_uuid() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }

    /// Load instance metadata from a meta directory.
    ///
    /// # Arguments
    /// * `meta_dir` - Path to the data/meta directory
    ///
    /// # Returns
    /// * `Ok(InstanceMetadata)` - Loaded metadata
    /// * `Err(DbError)` - Load error
    pub fn load(meta_dir: &Path) -> Result<Self> {
        let paths = MetaPaths::from_meta_dir(meta_dir);

        // Read required files
        let layout_version = fs::read_to_string(&paths.layout_version)
            .map_err(|e| DbError::Storage(format!("Failed to read layout-version: {}", e)))?;

        let instance_id = fs::read_to_string(&paths.instance_id)
            .map_err(|e| DbError::Storage(format!("Failed to read instance-id: {}", e)))?;

        let created_at = fs::read_to_string(&paths.created_at)
            .map_err(|e| DbError::Storage(format!("Failed to read created-at: {}", e)))?;

        let data_format_str = fs::read_to_string(&paths.data_format)
            .map_err(|e| DbError::Storage(format!("Failed to read data-format-version: {}", e)))?;

        let data_format: u32 = data_format_str.trim().parse().map_err(|_| {
            DbError::InvalidInput(format!("Invalid data format version: {}", data_format_str))
        })?;

        // Read optional files
        let wal_format = fs::read_to_string(&paths.wal_format)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .map(WalFormatVersion);

        let protocol = fs::read_to_string(&paths.protocol)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .map(ProtocolVersion);

        let last_upgraded_from = fs::read_to_string(&paths.last_upgraded_from)
            .ok()
            .and_then(|s| BinaryVersion::parse(s.trim()).ok());

        let engine_features = fs::read_to_string(&paths.engine_features)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        let compat_hints = fs::read_to_string(&paths.compat_json)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(Self {
            layout_version: LayoutVersion(layout_version.trim().to_string()),
            instance_id: instance_id.trim().to_string(),
            created_at_rfc3339: created_at.trim().to_string(),
            data_format: DataFormatVersion(data_format),
            wal_format,
            protocol,
            last_upgraded_from,
            engine_features,
            compat_hints,
        })
    }

    /// Save instance metadata to a meta directory.
    ///
    /// # Arguments
    /// * `meta_dir` - Path to the data/meta directory
    ///
    /// # Returns
    /// * `Ok(())` - Save successful
    /// * `Err(DbError)` - Save error
    pub fn save(&self, meta_dir: &Path) -> Result<()> {
        // Ensure directory exists
        fs::create_dir_all(meta_dir)
            .map_err(|e| DbError::Storage(format!("Failed to create meta directory: {}", e)))?;

        let paths = MetaPaths::from_meta_dir(meta_dir);

        // Write required files atomically
        write_file_atomic(&paths.layout_version, &self.layout_version.0)?;
        write_file_atomic(&paths.instance_id, &self.instance_id)?;
        write_file_atomic(&paths.created_at, &self.created_at_rfc3339)?;
        write_file_atomic(&paths.data_format, &self.data_format.0.to_string())?;

        // Write optional files
        if let Some(ref wf) = self.wal_format {
            write_file_atomic(&paths.wal_format, &wf.0.to_string())?;
        }

        if let Some(ref p) = self.protocol {
            write_file_atomic(&paths.protocol, &p.0.to_string())?;
        }

        if let Some(ref luf) = self.last_upgraded_from {
            write_file_atomic(&paths.last_upgraded_from, &luf.to_string())?;
        }

        if let Some(ref ef) = self.engine_features {
            let json = serde_json::to_string_pretty(ef)
                .map_err(|e| DbError::Serialization(format!("Failed to serialize engine features: {}", e)))?;
            write_file_atomic(&paths.engine_features, &json)?;
        }

        if let Some(ref ch) = self.compat_hints {
            let json = serde_json::to_string_pretty(ch)
                .map_err(|e| DbError::Serialization(format!("Failed to serialize compat hints: {}", e)))?;
            write_file_atomic(&paths.compat_json, &json)?;
        }

        Ok(())
    }

    /// Get version information summary.
    pub fn version_info(&self) -> VersionInfo {
        VersionInfo {
            binary_version: BinaryVersion::current(),
            layout_version: self.layout_version.clone(),
            data_format_version: self.data_format,
            wal_format_version: self.wal_format,
            protocol_version: self.protocol,
        }
    }
}

impl Default for InstanceMetadata {
    fn default() -> Self {
        Self::new_with_uuid()
    }
}

/// Version information summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Current binary version
    pub binary_version: BinaryVersion,
    /// Instance layout version
    pub layout_version: LayoutVersion,
    /// Data format version
    pub data_format_version: DataFormatVersion,
    /// WAL format version (if applicable)
    pub wal_format_version: Option<WalFormatVersion>,
    /// Protocol version (if applicable)
    pub protocol_version: Option<ProtocolVersion>,
}

impl VersionInfo {
    /// Get current version information.
    pub fn current() -> Self {
        Self {
            binary_version: BinaryVersion::current(),
            layout_version: LayoutVersion::current(),
            data_format_version: DataFormatVersion::current(),
            wal_format_version: Some(WalFormatVersion::current()),
            protocol_version: Some(ProtocolVersion::current()),
        }
    }
}

/// Paths to canonical metadata files under data/meta/.
///
/// Centralizes all metadata file paths to avoid drift.
#[derive(Debug, Clone)]
pub struct MetaPaths {
    /// The meta directory itself
    pub meta_dir: PathBuf,
    /// Path to layout-version file
    pub layout_version: PathBuf,
    /// Path to instance-id file
    pub instance_id: PathBuf,
    /// Path to created-at file
    pub created_at: PathBuf,
    /// Path to data-format-version file
    pub data_format: PathBuf,
    /// Path to wal-format-version file
    pub wal_format: PathBuf,
    /// Path to protocol-version file
    pub protocol: PathBuf,
    /// Path to last-upgraded-from file
    pub last_upgraded_from: PathBuf,
    /// Path to engine-features.json file
    pub engine_features: PathBuf,
    /// Path to compat.json file
    pub compat_json: PathBuf,
}

impl MetaPaths {
    /// Construct all metadata paths from the instance root.
    ///
    /// # Arguments
    /// * `instance_root` - Root directory of the instance (--home)
    pub fn from_instance_root(instance_root: &Path) -> Self {
        let meta_dir = instance_root.join("data").join("meta");
        Self::from_meta_dir(&meta_dir)
    }

    /// Construct all metadata paths from the meta directory.
    ///
    /// # Arguments
    /// * `meta_dir` - The data/meta directory path
    pub fn from_meta_dir(meta_dir: &Path) -> Self {
        Self {
            layout_version: meta_dir.join("layout-version"),
            instance_id: meta_dir.join("instance-id"),
            created_at: meta_dir.join("created-at"),
            data_format: meta_dir.join("data-format-version"),
            wal_format: meta_dir.join("wal-format-version"),
            protocol: meta_dir.join("protocol-version"),
            last_upgraded_from: meta_dir.join("last-upgraded-from"),
            engine_features: meta_dir.join("engine-features.json"),
            compat_json: meta_dir.join("compat.json"),
            meta_dir: meta_dir.to_path_buf(),
        }
    }

    /// Check if the meta directory exists.
    pub fn exists(&self) -> bool {
        self.meta_dir.exists()
    }

    /// Check if required metadata files exist.
    pub fn has_required_files(&self) -> bool {
        self.layout_version.exists()
            && self.instance_id.exists()
            && self.created_at.exists()
            && self.data_format.exists()
    }
}

/// Write a file atomically using temp file + fsync + rename.
///
/// # Arguments
/// * `path` - Target file path
/// * `content` - Content to write
fn write_file_atomic(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        DbError::Storage(format!("Invalid path: {:?}", path))
    })?;

    // Create temp file in same directory
    let temp_path = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));

    // Write to temp file
    let mut file = fs::File::create(&temp_path)
        .map_err(|e| DbError::Storage(format!("Failed to create temp file: {}", e)))?;

    file.write_all(content.as_bytes())
        .map_err(|e| DbError::Storage(format!("Failed to write temp file: {}", e)))?;

    // Fsync
    file.sync_all()
        .map_err(|e| DbError::Storage(format!("Failed to sync temp file: {}", e)))?;

    // Rename atomically
    fs::rename(&temp_path, path)
        .map_err(|e| DbError::Storage(format!("Failed to rename temp file: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_binary_version_parse() {
        let v = BinaryVersion::parse("0.3.1").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 1);

        let v2 = BinaryVersion::parse("1.2.3").unwrap();
        assert_eq!(v2.major, 1);
        assert_eq!(v2.minor, 2);
        assert_eq!(v2.patch, 3);
    }

    #[test]
    fn test_binary_version_display() {
        let v = BinaryVersion::new(0, 3, 1);
        assert_eq!(v.to_string(), "0.3.1");
    }

    #[test]
    fn test_layout_version() {
        let lv = LayoutVersion::current();
        assert_eq!(lv.as_str(), "1.0");
        assert!(lv.is_supported());
    }

    #[test]
    fn test_instance_metadata_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let meta_dir = temp_dir.path().join("data").join("meta");

        let metadata = InstanceMetadata::new("test-instance-123");
        metadata.save(&meta_dir).unwrap();

        let loaded = InstanceMetadata::load(&meta_dir).unwrap();
        assert_eq!(loaded.instance_id, "test-instance-123");
        assert_eq!(loaded.layout_version.as_str(), "1.0");
    }

    #[test]
    fn test_meta_paths() {
        let root = Path::new("/var/lib/rustydb/instances/prod");
        let paths = MetaPaths::from_instance_root(root);

        assert_eq!(
            paths.meta_dir,
            PathBuf::from("/var/lib/rustydb/instances/prod/data/meta")
        );
        assert_eq!(
            paths.instance_id,
            PathBuf::from("/var/lib/rustydb/instances/prod/data/meta/instance-id")
        );
    }
}
