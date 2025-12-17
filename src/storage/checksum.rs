// Hardware-accelerated CRC32C checksum utilities
// Extracted from duplicated implementations in page.rs and disk.rs

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{_mm_crc32_u64, _mm_crc32_u8};

/// Hardware-accelerated CRC32C checksum (SSE4.2 on x86_64)
/// Falls back to software implementation when hardware support is unavailable
#[inline]
pub fn hardware_crc32c(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sse4.2") {
            return unsafe { hardware_crc32c_impl(data) };
        }
    }
    // Fallback to software CRC32
    software_crc32c(data)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
unsafe fn hardware_crc32c_impl(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    let mut ptr = data.as_ptr();
    let mut remaining = data.len();

    // Process 8 bytes at a time for maximum throughput
    while remaining >= 8 {
        let value = (ptr as *const u64).read_unaligned();
        crc = _mm_crc32_u64(crc as u64, value) as u32;
        ptr = ptr.add(8);
        remaining -= 8;
    }

    // Process remaining bytes
    while remaining > 0 {
        let value = *ptr;
        crc = _mm_crc32_u8(crc, value);
        ptr = ptr.add(1);
        remaining -= 1;
    }

    !crc
}

/// Software fallback CRC32C using lookup table
fn software_crc32c(data: &[u8]) -> u32 {
    const CRC32C_TABLE: [u32; 256] = generate_crc32c_table();
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[index];
    }
    !crc
}

/// Generate CRC32C lookup table at compile time
const fn generate_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let poly: u32 = 0x82F63B78; // CRC32C polynomial
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ poly;
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

/// Software-only CRC32C using crc32c crate (for compatibility)
#[allow(dead_code)]
pub fn software_crc32c_crate(data: &[u8]) -> u32 {
    crc32c::crc32c(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32c_consistency() {
        let test_data = b"Hello, RustyDB!";
        let hw_result = hardware_crc32c(test_data);
        let sw_result = software_crc32c(test_data);

        // Hardware and software should produce same results
        // (On systems without SSE4.2, both will use software path)
        assert_eq!(hw_result, sw_result);
    }

    #[test]
    fn test_crc32c_empty() {
        let empty: &[u8] = &[];
        let result = hardware_crc32c(empty);
        assert_eq!(result, !0xFFFFFFFF);
    }

    #[test]
    fn test_crc32c_known_values() {
        // Test with known values
        let data = b"123456789";
        let result = hardware_crc32c(data);
        // CRC32C of "123456789" is 0xe3069283
        assert_eq!(result, 0xe3069283);
    }
}
