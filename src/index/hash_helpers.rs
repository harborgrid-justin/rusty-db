// Hash Index Helper Functions
//
// This module provides common utilities for hash index implementations,
// eliminating code duplication across ExtendibleHashIndex and LinearHashIndex.

use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hash a key using optimized hash functions
///
/// - For String keys: uses SIMD-accelerated xxHash3-AVX2 (10x faster)
/// - For other types: uses DefaultHasher
///
/// This helper is used by both ExtendibleHashIndex and LinearHashIndex
/// to avoid code duplication.
pub fn hash_key<K: Hash + 'static>(key: &K) -> u64 {
    // Fast path for string keys - use SIMD hash
    if TypeId::of::<K>() == TypeId::of::<String>() {
        let key_str = unsafe { &*(key as *const K as *const String) };
        return crate::simd::hash::hash_str(key_str);
    }

    // Fallback to DefaultHasher for other types
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_key_string() {
        let key = "test_string".to_string();
        let hash1 = hash_key(&key);
        let hash2 = hash_key(&key);

        // Should be deterministic
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, 0);
    }

    #[test]
    fn test_hash_key_i32() {
        let key = 42i32;
        let hash1 = hash_key(&key);
        let hash2 = hash_key(&key);

        // Should be deterministic
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, 0);
    }

    #[test]
    fn test_hash_key_different_values() {
        let key1 = 42i32;
        let key2 = 43i32;

        // Different keys should (usually) have different hashes
        assert_ne!(hash_key(&key1), hash_key(&key2));
    }
}
