//! Utility Functions

use super::common::*;

pub use crate::memory::types::AllocatorType;

/// Helper to format memory size
pub fn format_memory_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Helper to parse memory size string
pub fn parse_memory_size(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    let (num_str, multiplier) = if s.ends_with("TB") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024 * 1024)
    } else if s.ends_with("GB") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len()-2], 1024u64 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len()-2], 1024u64)
    } else if s.ends_with("B") {
        (&s[..s.len()-1], 1)
    } else {
        (s.as_str(), 1)
    };

    let num: f64 = num_str.trim().parse()
        .map_err(|e| DbError::InvalidArgument(format!("Invalid memory size: {}", e)))?;

    Ok((num * multiplier as f64) as u64)
}

/// Calculate optimal slab size for object size
pub fn calculate_optimal_slab_size(object_size: usize) -> usize {
    // Aim for ~2MB slabs aligned to huge pages
    let objects_per_slab = SLAB_SIZE / object_size.max(1);
    if objects_per_slab < 64 {
        // Too few objects, use smaller slab
        (object_size * 64).next_power_of_two()
    } else {
        SLAB_SIZE
    }
}

/// Check if size should use slab, system, or large object allocator
pub fn classify_allocation_size(size: usize) -> AllocatorType {
    crate::memory::types::classify_allocation_size(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memory_size() {
        assert_eq!(format_memory_size(1024), "1.00 KB");
        assert_eq!(format_memory_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_memory_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_parse_memory_size() {
        assert_eq!(parse_memory_size("1KB").unwrap(), 1024);
        assert_eq!(parse_memory_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_memory_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_classify_allocation_size() {
        assert_eq!(classify_allocation_size(1024), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(MAX_SLAB_SIZE), AllocatorType::Slab);
        assert_eq!(classify_allocation_size(MAX_SLAB_SIZE + 1), AllocatorType::System);
        assert_eq!(classify_allocation_size(LARGE_OBJECT_THRESHOLD), AllocatorType::LargeObject);
    }
}
