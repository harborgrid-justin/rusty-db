//! Compatibility Checking for RustyDB
//!
//! This module provides compile-time compatibility policies and runtime checks
//! for instance layout, data format, WAL format, and protocol versions.
//!
//! # Overview
//!
//! RustyDB maintains strict version compatibility to ensure data integrity:
//! - Layout Version: Directory structure compatibility
//! - Data Format Version: On-disk data format compatibility
//! - WAL Format Version: Write-ahead log compatibility
//! - Protocol Version: Client-server protocol compatibility
//!
//! # Example
//!
//! ```rust,no_run
//! use rusty_db::compat::{check_compat, OpenMode, COMPAT_POLICY_V1};
//! use rusty_db::metadata::{LayoutVersion, DataFormatVersion};
//!
//! let result = check_compat(
//!     &COMPAT_POLICY_V1,
//!     &LayoutVersion::new("1.0"),
//!     DataFormatVersion::new(2),
//!     None,
//!     None,
//!     OpenMode::ReadWrite,
//! );
//!
//! match result {
//!     Ok(()) => println!("Instance is compatible"),
//!     Err(e) => eprintln!("Compatibility error: {}", e),
//! }
//! ```

use thiserror::Error;

use crate::metadata::{DataFormatVersion, LayoutVersion, ProtocolVersion, WalFormatVersion};

/// Errors that can occur during compatibility checking.
#[derive(Debug, Error)]
pub enum CompatError {
    /// Instance layout version is not supported by this binary.
    #[error("unsupported instance layout version '{found}'. Supported: {supported:?}")]
    UnsupportedLayout {
        /// The layout version found in the instance
        found: String,
        /// List of supported layout versions
        supported: Vec<String>,
    },

    /// Data format version is not supported for the requested mode.
    #[error("unsupported data format version {found} for mode {mode:?}. Supported RW: {rw_min}-{rw_max}, RO: {ro_min:?}-{ro_max:?}")]
    UnsupportedDataFormat {
        /// The data format version found
        found: u32,
        /// The requested open mode
        mode: OpenMode,
        /// Minimum version for read-write access
        rw_min: u32,
        /// Maximum version for read-write access
        rw_max: u32,
        /// Minimum version for read-only access (if different)
        ro_min: Option<u32>,
        /// Maximum version for read-only access (if different)
        ro_max: Option<u32>,
    },

    /// WAL format version is not supported for the requested mode.
    #[error("unsupported WAL format version {found} for mode {mode:?}. Supported RW: {rw_min}-{rw_max}")]
    UnsupportedWalFormat {
        /// The WAL format version found
        found: u32,
        /// The requested open mode
        mode: OpenMode,
        /// Minimum version for read-write access
        rw_min: u32,
        /// Maximum version for read-write access
        rw_max: u32,
    },

    /// Protocol version is not supported.
    #[error("unsupported protocol version {found}. Supported: {min}-{max}")]
    UnsupportedProtocol {
        /// The protocol version found
        found: u32,
        /// Minimum supported version
        min: u32,
        /// Maximum supported version
        max: u32,
    },

    /// Required metadata field is missing.
    #[error("metadata missing required field: {0}")]
    MissingRequired(&'static str),

    /// Instance appears to be from a newer version.
    #[error("instance was created by a newer version of RustyDB. Found data format {found}, max supported: {max_supported}")]
    NewerVersion {
        /// The version found
        found: u32,
        /// Maximum version this binary supports
        max_supported: u32,
    },

    /// Instance needs upgrade before use.
    #[error("instance needs upgrade. Found data format {found}, current: {current}. Run 'rusty-db-cli upgrade --home <path>' first")]
    NeedsUpgrade {
        /// The version found
        found: u32,
        /// Current version
        current: u32,
    },
}

/// Mode in which the server is attempting to open the instance.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpenMode {
    /// Full read-write access
    ReadWrite,
    /// Read-only access (may support older formats)
    ReadOnly,
}

impl std::fmt::Display for OpenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenMode::ReadWrite => write!(f, "read-write"),
            OpenMode::ReadOnly => write!(f, "read-only"),
        }
    }
}

/// Supported version range for a particular format.
///
/// Allows different ranges for read-write vs read-only access.
#[derive(Debug, Clone)]
pub struct SupportedRange<T: Copy> {
    /// Minimum version for read-write access
    pub rw_min: T,
    /// Maximum version for read-write access
    pub rw_max: T,
    /// Minimum version for read-only access (optional, extends rw range if set)
    pub ro_min: Option<T>,
    /// Maximum version for read-only access (optional, extends rw range if set)
    pub ro_max: Option<T>,
}

impl<T: Copy> SupportedRange<T> {
    /// Create a new supported range for read-write only.
    pub fn rw_only(min: T, max: T) -> Self {
        Self {
            rw_min: min,
            rw_max: max,
            ro_min: None,
            ro_max: None,
        }
    }

    /// Create a new supported range with extended read-only support.
    pub fn with_ro(rw_min: T, rw_max: T, ro_min: T, ro_max: T) -> Self {
        Self {
            rw_min,
            rw_max,
            ro_min: Some(ro_min),
            ro_max: Some(ro_max),
        }
    }
}

/// Compatibility policy defining what versions this binary supports.
///
/// This is typically defined as a compile-time constant.
#[derive(Debug, Clone)]
pub struct CompatibilityPolicy {
    /// Supported instance layout versions
    pub supported_layouts: &'static [&'static str],
    /// Supported data format version range
    pub data_format: SupportedRange<DataFormatVersion>,
    /// Supported WAL format version range (optional)
    pub wal_format: Option<SupportedRange<WalFormatVersion>>,
    /// Supported protocol version range (optional)
    pub protocol: Option<SupportedRange<ProtocolVersion>>,
}

/// RustyDB v0.3.1 compatibility policy.
///
/// This defines what versions of instance layouts, data formats,
/// WAL formats, and protocols this version of RustyDB supports.
pub static COMPAT_POLICY_V1: CompatibilityPolicy = CompatibilityPolicy {
    // Support layout version 1.0
    supported_layouts: &["1.0"],

    // Data format: RW supports 1-2, RO supports 1 (older format)
    data_format: SupportedRange {
        rw_min: DataFormatVersion(1),
        rw_max: DataFormatVersion(2),
        ro_min: Some(DataFormatVersion(1)),
        ro_max: Some(DataFormatVersion(2)),
    },

    // WAL format: RW supports 1-2
    wal_format: Some(SupportedRange {
        rw_min: WalFormatVersion(1),
        rw_max: WalFormatVersion(2),
        ro_min: None,
        ro_max: None,
    }),

    // Protocol: supports 1-2
    protocol: Some(SupportedRange {
        rw_min: ProtocolVersion(1),
        rw_max: ProtocolVersion(2),
        ro_min: None,
        ro_max: None,
    }),
};

/// Check whether a found version is compatible with the supported range.
///
/// # Arguments
/// * `found` - The version found in the instance
/// * `mode` - The requested open mode
/// * `rw_min` - Minimum version for read-write
/// * `rw_max` - Maximum version for read-write
/// * `ro_min` - Minimum version for read-only (optional)
/// * `ro_max` - Maximum version for read-only (optional)
fn check_range_u32(
    found: u32,
    mode: OpenMode,
    rw_min: u32,
    rw_max: u32,
    ro_min: Option<u32>,
    ro_max: Option<u32>,
) -> bool {
    match mode {
        OpenMode::ReadWrite => found >= rw_min && found <= rw_max,
        OpenMode::ReadOnly => {
            // First check if it's in the RW range
            if found >= rw_min && found <= rw_max {
                return true;
            }
            // Then check extended RO range if defined
            match (ro_min, ro_max) {
                (Some(min), Some(max)) => found >= min && found <= max,
                _ => false,
            }
        }
    }
}

/// Check compatibility of an instance against a policy.
///
/// # Arguments
/// * `policy` - The compatibility policy to check against
/// * `found_layout` - The layout version found in the instance
/// * `found_data` - The data format version found
/// * `found_wal` - The WAL format version found (optional)
/// * `found_proto` - The protocol version found (optional)
/// * `mode` - The requested open mode
///
/// # Returns
/// * `Ok(())` - Instance is compatible
/// * `Err(CompatError)` - Compatibility error with details
///
/// # Example
///
/// ```rust,no_run
/// use rusty_db::compat::{check_compat, OpenMode, COMPAT_POLICY_V1};
/// use rusty_db::metadata::{LayoutVersion, DataFormatVersion};
///
/// let result = check_compat(
///     &COMPAT_POLICY_V1,
///     &LayoutVersion::new("1.0"),
///     DataFormatVersion::new(2),
///     None,
///     None,
///     OpenMode::ReadWrite,
/// );
/// assert!(result.is_ok());
/// ```
pub fn check_compat(
    policy: &CompatibilityPolicy,
    found_layout: &LayoutVersion,
    found_data: DataFormatVersion,
    found_wal: Option<WalFormatVersion>,
    found_proto: Option<ProtocolVersion>,
    mode: OpenMode,
) -> Result<(), CompatError> {
    // Check layout version
    let found_str = found_layout.0.as_str();
    if !policy.supported_layouts.iter().any(|s| *s == found_str) {
        return Err(CompatError::UnsupportedLayout {
            found: found_layout.0.clone(),
            supported: policy
                .supported_layouts
                .iter()
                .map(|s| s.to_string())
                .collect(),
        });
    }

    // Check data format version
    let df = found_data.0;
    let df_ok = check_range_u32(
        df,
        mode,
        policy.data_format.rw_min.0,
        policy.data_format.rw_max.0,
        policy.data_format.ro_min.map(|v| v.0),
        policy.data_format.ro_max.map(|v| v.0),
    );

    if !df_ok {
        // Provide more specific error message
        if df > policy.data_format.rw_max.0 {
            return Err(CompatError::NewerVersion {
                found: df,
                max_supported: policy.data_format.rw_max.0,
            });
        }

        return Err(CompatError::UnsupportedDataFormat {
            found: df,
            mode,
            rw_min: policy.data_format.rw_min.0,
            rw_max: policy.data_format.rw_max.0,
            ro_min: policy.data_format.ro_min.map(|v| v.0),
            ro_max: policy.data_format.ro_max.map(|v| v.0),
        });
    }

    // Check WAL format version (if both policy and instance have it)
    if let (Some(wal_range), Some(wal_found)) = (&policy.wal_format, found_wal) {
        let wf = wal_found.0;
        let ok = check_range_u32(
            wf,
            mode,
            wal_range.rw_min.0,
            wal_range.rw_max.0,
            wal_range.ro_min.map(|v| v.0),
            wal_range.ro_max.map(|v| v.0),
        );
        if !ok {
            return Err(CompatError::UnsupportedWalFormat {
                found: wf,
                mode,
                rw_min: wal_range.rw_min.0,
                rw_max: wal_range.rw_max.0,
            });
        }
    }

    // Check protocol version (if both policy and instance have it)
    if let (Some(proto_range), Some(proto_found)) = (&policy.protocol, found_proto) {
        let pf = proto_found.0;
        // Protocol is typically negotiated; treat as strict
        let ok = check_range_u32(
            pf,
            OpenMode::ReadWrite, // Protocol must be in RW range
            proto_range.rw_min.0,
            proto_range.rw_max.0,
            None,
            None,
        );
        if !ok {
            return Err(CompatError::UnsupportedProtocol {
                found: pf,
                min: proto_range.rw_min.0,
                max: proto_range.rw_max.0,
            });
        }
    }

    Ok(())
}

/// Quick check if an instance can be opened in the requested mode.
///
/// This is a convenience function that loads metadata and checks compatibility.
///
/// # Arguments
/// * `meta_dir` - Path to the data/meta directory
/// * `mode` - The requested open mode
///
/// # Returns
/// * `Ok(())` - Instance can be opened
/// * `Err(CompatError)` - Compatibility error
pub fn check_instance_compat(
    meta_dir: &std::path::Path,
    mode: OpenMode,
) -> Result<(), CompatError> {
    use crate::metadata::InstanceMetadata;

    let metadata = InstanceMetadata::load(meta_dir).map_err(|_| {
        CompatError::MissingRequired("instance metadata")
    })?;

    check_compat(
        &COMPAT_POLICY_V1,
        &metadata.layout_version,
        metadata.data_format,
        metadata.wal_format,
        metadata.protocol,
        mode,
    )
}

/// Compatibility result with additional information.
#[derive(Debug, Clone)]
pub struct CompatCheckResult {
    /// Whether the instance is compatible
    pub compatible: bool,
    /// The checked layout version
    pub layout_version: String,
    /// The checked data format version
    pub data_format_version: u32,
    /// Whether an upgrade is recommended
    pub upgrade_recommended: bool,
    /// Human-readable message
    pub message: String,
}

impl CompatCheckResult {
    /// Create a successful result.
    pub fn ok(layout: &str, data_format: u32) -> Self {
        Self {
            compatible: true,
            layout_version: layout.to_string(),
            data_format_version: data_format,
            upgrade_recommended: false,
            message: "Instance is compatible".to_string(),
        }
    }

    /// Create a result with upgrade recommendation.
    pub fn upgrade_recommended(layout: &str, data_format: u32, current: u32) -> Self {
        Self {
            compatible: true,
            layout_version: layout.to_string(),
            data_format_version: data_format,
            upgrade_recommended: true,
            message: format!(
                "Instance uses data format {}, current is {}. Consider upgrading.",
                data_format, current
            ),
        }
    }

    /// Create an incompatible result.
    pub fn incompatible(error: &CompatError) -> Self {
        Self {
            compatible: false,
            layout_version: String::new(),
            data_format_version: 0,
            upgrade_recommended: false,
            message: error.to_string(),
        }
    }
}

/// Detailed compatibility check returning rich result information.
pub fn check_compat_detailed(
    policy: &CompatibilityPolicy,
    found_layout: &LayoutVersion,
    found_data: DataFormatVersion,
    found_wal: Option<WalFormatVersion>,
    found_proto: Option<ProtocolVersion>,
    mode: OpenMode,
) -> CompatCheckResult {
    match check_compat(policy, found_layout, found_data, found_wal, found_proto, mode) {
        Ok(()) => {
            // Check if upgrade is recommended
            if found_data.0 < policy.data_format.rw_max.0 {
                CompatCheckResult::upgrade_recommended(
                    found_layout.as_str(),
                    found_data.0,
                    policy.data_format.rw_max.0,
                )
            } else {
                CompatCheckResult::ok(found_layout.as_str(), found_data.0)
            }
        }
        Err(e) => CompatCheckResult::incompatible(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_compat_success() {
        let result = check_compat(
            &COMPAT_POLICY_V1,
            &LayoutVersion::new("1.0"),
            DataFormatVersion::new(2),
            Some(WalFormatVersion::new(2)),
            Some(ProtocolVersion::new(2)),
            OpenMode::ReadWrite,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_compat_unsupported_layout() {
        let result = check_compat(
            &COMPAT_POLICY_V1,
            &LayoutVersion::new("2.0"),
            DataFormatVersion::new(2),
            None,
            None,
            OpenMode::ReadWrite,
        );
        assert!(matches!(result, Err(CompatError::UnsupportedLayout { .. })));
    }

    #[test]
    fn test_check_compat_newer_version() {
        let result = check_compat(
            &COMPAT_POLICY_V1,
            &LayoutVersion::new("1.0"),
            DataFormatVersion::new(99),
            None,
            None,
            OpenMode::ReadWrite,
        );
        assert!(matches!(result, Err(CompatError::NewerVersion { .. })));
    }

    #[test]
    fn test_check_compat_readonly_extended() {
        // Version 1 should work in read-only mode
        let result = check_compat(
            &COMPAT_POLICY_V1,
            &LayoutVersion::new("1.0"),
            DataFormatVersion::new(1),
            None,
            None,
            OpenMode::ReadOnly,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_mode_display() {
        assert_eq!(OpenMode::ReadWrite.to_string(), "read-write");
        assert_eq!(OpenMode::ReadOnly.to_string(), "read-only");
    }

    #[test]
    fn test_compat_check_result() {
        let result = CompatCheckResult::ok("1.0", 2);
        assert!(result.compatible);
        assert!(!result.upgrade_recommended);

        let result = CompatCheckResult::upgrade_recommended("1.0", 1, 2);
        assert!(result.compatible);
        assert!(result.upgrade_recommended);
    }
}
