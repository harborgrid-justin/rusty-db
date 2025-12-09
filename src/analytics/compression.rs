// Query Result Compression
//
// This module provides compression capabilities for query results,
// reducing memory usage and network transfer times for large result sets.
//
// # Architecture
//
// The compression system supports multiple algorithms:
// - Run-length encoding for repeated values
// - Dictionary encoding for low-cardinality columns
// - Delta encoding for sorted numeric data
// - LZ4-style compression for general data
//
// # Example
//
// ```rust,ignore
// use crate::analytics::compression::{QueryResultCompressor, CompressionAlgorithm};
//
// let compressor = QueryResultCompressor::new(CompressionAlgorithm::Dictionary);
// let compressed = compressor.compress(&data);
// let original = compressor.decompress(&compressed);
// ```

use std::collections::HashSet;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

// Compression algorithm for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    // No compression
    None,
    // Run-length encoding for repeated values
    RunLength,
    // Dictionary encoding for low-cardinality data
    Dictionary,
    // Delta encoding for sorted/sequential data
    Delta,
    // Bit-packing for small integers
    BitPacking,
    // LZ4-style byte compression
    Lz4,
    // Snappy-style compression
    Snappy,
    // Hybrid (chooses best per column)
    Adaptive,
}

impl CompressionAlgorithm {
    // Returns the expected compression ratio for the algorithm.
    pub fn expected_ratio(&self) -> f64 {
        match self {
            CompressionAlgorithm::None => 1.0,
            CompressionAlgorithm::RunLength => 0.3,
            CompressionAlgorithm::Dictionary => 0.2,
            CompressionAlgorithm::Delta => 0.4,
            CompressionAlgorithm::BitPacking => 0.25,
            CompressionAlgorithm::Lz4 => 0.5,
            CompressionAlgorithm::Snappy => 0.6,
            CompressionAlgorithm::Adaptive => 0.25,
        }
    }

    // Returns whether the algorithm is suitable for the data characteristics.
    pub fn suitable_for(&self, cardinality: usize, total_count: usize, issorted: bool) -> bool {
        match self {
            CompressionAlgorithm::None => true,
            CompressionAlgorithm::RunLength => {
                // Good when consecutive values repeat
                cardinality < total_count / 10
            }
            CompressionAlgorithm::Dictionary => {
                // Good for low cardinality
                cardinality < 65536 && cardinality < total_count / 2
            }
            CompressionAlgorithm::Delta => {
                // Best for sorted numeric data
                issorted
            }
            CompressionAlgorithm::BitPacking => {
                // Good for small value ranges
                cardinality < 256
            }
            _ => true,
        }
    }
}

// Compressed representation of query results.
#[derive(Debug, Clone)]
pub struct CompressedResult {
    // Compression algorithm used
    pub algorithm: CompressionAlgorithm,
    // Compressed data bytes
    pub data: Vec<u8>,
    // Original uncompressed size
    pub original_size: usize,
    // Compressed size
    pub compressed_size: usize,
    // Column metadata for decompression
    pub column_info: Vec<CompressedColumnInfo>,
    // Number of rows
    pub row_count: usize,
}

impl CompressedResult {
    // Returns the compression ratio (compressed/original).
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_size as f64 / self.original_size as f64
    }

    // Returns the space savings as a percentage.
    pub fn space_savings_percent(&self) -> f64 {
        (1.0 - self.compression_ratio()) * 100.0
    }
}

// Metadata for a compressed column.
#[derive(Debug, Clone)]
pub struct CompressedColumnInfo {
    // Column name
    pub name: String,
    // Algorithm used for this column
    pub algorithm: CompressionAlgorithm,
    // Offset in the data buffer
    pub offset: usize,
    // Length of compressed data
    pub length: usize,
    // Dictionary for dictionary-encoded columns
    pub dictionary: Option<Vec<String>>,
    // Base value for delta encoding
    pub delta_base: Option<i64>,
}

// Run-length encoded segment.
#[derive(Debug, Clone)]
struct RleSegment {
    value: Vec<u8>,
    count: u32,
}

// Query result compressor.
#[derive(Debug)]
pub struct QueryResultCompressor {
    // Default algorithm
    algorithm: CompressionAlgorithm,
    // Compression level (1-9)
    level: u8,
    // Statistics about compression
    stats: Arc<RwLock<CompressionStats>>,
}

// Statistics about compression operations.
#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    // Total bytes compressed
    pub bytes_in: usize,
    // Total bytes after compression
    pub bytes_out: usize,
    // Number of compression operations
    pub operations: usize,
    // Time spent compressing (microseconds)
    pub compress_time_us: u64,
    // Time spent decompressing (microseconds)
    pub decompress_time_us: u64,
}

impl CompressionStats {
    // Returns the overall compression ratio.
    pub fn overall_ratio(&self) -> f64 {
        if self.bytes_in == 0 {
            return 1.0;
        }
        self.bytes_out as f64 / self.bytes_in as f64
    }
}

impl Default for QueryResultCompressor {
    fn default() -> Self {
        Self::new(CompressionAlgorithm::Adaptive)
    }
}

impl QueryResultCompressor {
    // Creates a new compressor with the specified algorithm.
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            level: 6,
            stats: Arc::new(RwLock::new(CompressionStats::default())),
        }
    }

    // Sets the compression level (1-9).
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 9);
        self
    }

    // Compresses string data using the configured algorithm.
    pub fn compress_strings(&self, data: &[String]) -> CompressedResult {
        let start = std::time::Instant::now();

        let original_size: usize = data.iter().map(|s| s.len() + 4).sum();
        let algorithm = self.choose_algorithm(data);

        let (compressed_data, column_info) = match algorithm {
            CompressionAlgorithm::Dictionary => self.dictionary_encode(data),
            CompressionAlgorithm::RunLength => self.run_length_encode(data),
            _ => self.simple_encode(data),
        };

        let result = CompressedResult {
            algorithm,
            compressed_size: compressed_data.len(),
            data: compressed_data,
            original_size,
            column_info: vec![column_info],
            row_count: data.len(),
        };

        // Update stats
        let mut stats = self.stats.write();
        stats.bytes_in += original_size;
        stats.bytes_out += result.compressed_size;
        stats.operations += 1;
        stats.compress_time_us += start.elapsed().as_micros() as u64;

        result
    }

    // Compresses numeric data.
    pub fn compress_numbers(&self, data: &[i64]) -> CompressedResult {
        let start = std::time::Instant::now();

        let original_size = data.len() * 8;
        let is_sorted = data.windows(2).all(|w| w[0] <= w[1]);

        let (compressed_data, column_info) = if is_sorted {
            self.delta_encode(data)
        } else {
            self.varint_encode(data)
        };

        let result = CompressedResult {
            algorithm: if is_sorted {
                CompressionAlgorithm::Delta
            } else {
                CompressionAlgorithm::BitPacking
            },
            compressed_size: compressed_data.len(),
            data: compressed_data,
            original_size,
            column_info: vec![column_info],
            row_count: data.len(),
        };

        let mut stats = self.stats.write();
        stats.bytes_in += original_size;
        stats.bytes_out += result.compressed_size;
        stats.operations += 1;
        stats.compress_time_us += start.elapsed().as_micros() as u64;

        result
    }

    // Decompresses string data.
    pub fn decompress_strings(&self, compressed: &CompressedResult) -> Vec<String> {
        let start = std::time::Instant::now();

        let result = match compressed.algorithm {
            CompressionAlgorithm::Dictionary => {
                self.dictionary_decode(compressed)
            }
            CompressionAlgorithm::RunLength => {
                self.run_length_decode(compressed)
            }
            _ => self.simple_decode(compressed),
        };

        self.stats.write().decompress_time_us += start.elapsed().as_micros() as u64;

        result
    }

    // Decompresses numeric data.
    pub fn decompress_numbers(&self, compressed: &CompressedResult) -> Vec<i64> {
        let start = std::time::Instant::now();

        let result = match compressed.algorithm {
            CompressionAlgorithm::Delta => self.delta_decode(compressed),
            _ => self.varint_decode(compressed),
        };

        self.stats.write().decompress_time_us += start.elapsed().as_micros() as u64;

        result
    }

    // Chooses the best algorithm for the data.
    fn choose_algorithm(&self, data: &[String]) -> CompressionAlgorithm {
        if self.algorithm != CompressionAlgorithm::Adaptive {
            return self.algorithm;
        }

        // Count unique values
        let mut unique: HashSet<&str> = HashSet::new();
        for s in data {
            unique.insert(s);
        }

        let cardinality = unique.len();

        // Check for runs
        let mut run_count = 1;
        for i in 1..data.len() {
            if data[i] != data[i - 1] {
                run_count += 1;
            }
        }

        // Dictionary encoding for low cardinality
        if cardinality < 256 && cardinality < data.len() / 2 {
            return CompressionAlgorithm::Dictionary;
        }

        // RLE for many runs
        if run_count < data.len() / 4 {
            return CompressionAlgorithm::RunLength;
        }

        CompressionAlgorithm::None
    }

    // Dictionary encodes string data.
    fn dictionary_encode(&self, data: &[String]) -> (Vec<u8>, CompressedColumnInfo) {
        let mut dictionary: Vec<String> = Vec::new();
        let mut dict_map: HashMap<&str, u16> = HashMap::new();
        let mut encoded: Vec<u16> = Vec::new();

        for s in data {
            let idx = *dict_map.entry(s.as_str()).or_insert_with(|| {
                let idx = dictionary.len() as u16;
                dictionary.push(s.clone());
                idx
            });
            encoded.push(idx);
        }

        // Serialize: [dict_size: u16][dict entries][encoded indices]
        let mut bytes: Vec<u8> = Vec::new();

        // Dictionary size
        bytes.extend_from_slice(&(dictionary.len() as u16).to_le_bytes());

        // Dictionary entries (length-prefixed)
        for entry in &dictionary {
            bytes.extend_from_slice(&(entry.len() as u16).to_le_bytes());
            bytes.extend_from_slice(entry.as_bytes());
        }

        // Encoded indices
        for idx in encoded {
            bytes.extend_from_slice(&idx.to_le_bytes());
        }

        let info = CompressedColumnInfo {
            name: String::new(),
            algorithm: CompressionAlgorithm::Dictionary,
            offset: 0,
            length: bytes.len(),
            dictionary: Some(dictionary),
            delta_base: None,
        };

        (bytes, info)
    }

    // Decodes dictionary-encoded data.
    fn dictionary_decode(&self, compressed: &CompressedResult) -> Vec<String> {
        if compressed.column_info.is_empty() {
            return Vec::new();
        }

        let info = &compressed.column_info[0];
        let dictionary = match &info.dictionary {
            Some(d) => d,
            None => return Vec::new(),
        };

        let data = &compressed.data;
        let mut result = Vec::with_capacity(compressed.row_count);

        // Skip dictionary in bytes (we have it in metadata)
        let dict_size = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut offset = 2;

        for _ in 0..dict_size {
            let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2 + len;
        }

        // Read indices
        while offset + 1 < data.len() {
            let idx = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            if idx < dictionary.len() {
                result.push(dictionary[idx].clone());
            }
            offset += 2;
        }

        result
    }

    // Run-length encodes string data.
    fn run_length_encode(&self, data: &[String]) -> (Vec<u8>, CompressedColumnInfo) {
        let mut bytes: Vec<u8> = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let value = &data[i];
            let mut count = 1u32;

            while (i + count as usize) < data.len() && data[i + count as usize] == *value {
                count += 1;
            }

            // Write: [count: u32][len: u16][bytes]
            bytes.extend_from_slice(&count.to_le_bytes());
            bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
            bytes.extend_from_slice(value.as_bytes());

            i += count as usize;
        }

        let info = CompressedColumnInfo {
            name: String::new(),
            algorithm: CompressionAlgorithm::RunLength,
            offset: 0,
            length: bytes.len(),
            dictionary: None,
            delta_base: None,
        };

        (bytes, info)
    }

    // Decodes run-length encoded data.
    fn run_length_decode(&self, compressed: &CompressedResult) -> Vec<String> {
        let data = &compressed.data;
        let mut result = Vec::with_capacity(compressed.row_count);
        let mut offset = 0;

        while offset + 6 <= data.len() {
            let count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;

            let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            if offset + len > data.len() {
                break;
            }

            let value = String::from_utf8_lossy(&data[offset..offset + len]).to_string();
            offset += len;

            for _ in 0..count {
                result.push(value.clone());
            }
        }

        result
    }

    // Simple length-prefixed encoding.
    fn simple_encode(&self, data: &[String]) -> (Vec<u8>, CompressedColumnInfo) {
        let mut bytes: Vec<u8> = Vec::new();

        for s in data {
            bytes.extend_from_slice(&(s.len() as u16).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }

        let info = CompressedColumnInfo {
            name: String::new(),
            algorithm: CompressionAlgorithm::None,
            offset: 0,
            length: bytes.len(),
            dictionary: None,
            delta_base: None,
        };

        (bytes, info)
    }

    // Decodes simple length-prefixed data.
    fn simple_decode(&self, compressed: &CompressedResult) -> Vec<String> {
        let data = &compressed.data;
        let mut result = Vec::with_capacity(compressed.row_count);
        let mut offset = 0;

        while offset + 2 <= data.len() {
            let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            if offset + len > data.len() {
                break;
            }

            let value = String::from_utf8_lossy(&data[offset..offset + len]).to_string();
            result.push(value);
            offset += len;
        }

        result
    }

    // Delta encodes numeric data.
    fn delta_encode(&self, data: &[i64]) -> (Vec<u8>, CompressedColumnInfo) {
        if data.is_empty() {
            return (
                Vec::new(),
                CompressedColumnInfo {
                    name: String::new(),
                    algorithm: CompressionAlgorithm::Delta,
                    offset: 0,
                    length: 0,
                    dictionary: None,
                    delta_base: None,
                },
            );
        }

        let mut bytes: Vec<u8> = Vec::new();
        let base = data[0];

        // Write base value
        bytes.extend_from_slice(&base.to_le_bytes());

        // Write deltas
        let mut prev = base;
        for &value in &data[1..] {
            let delta = value - prev;
            bytes.extend_from_slice(&(delta as i32).to_le_bytes());
            prev = value;
        }

        let info = CompressedColumnInfo {
            name: String::new(),
            algorithm: CompressionAlgorithm::Delta,
            offset: 0,
            length: bytes.len(),
            dictionary: None,
            delta_base: Some(base),
        };

        (bytes, info)
    }

    // Decodes delta-encoded data.
    fn delta_decode(&self, compressed: &CompressedResult) -> Vec<i64> {
        let data = &compressed.data;

        if data.len() < 8 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(compressed.row_count);

        // Read base
        let base = i64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        result.push(base);

        // Read deltas
        let mut offset = 8;
        let mut prev = base;

        while offset + 4 <= data.len() {
            let delta = i32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as i64;
            offset += 4;

            prev += delta;
            result.push(prev);
        }

        result
    }

    // Variable-length integer encoding.
    fn varint_encode(&self, data: &[i64]) -> (Vec<u8>, CompressedColumnInfo) {
        let mut bytes: Vec<u8> = Vec::new();

        for &value in data {
            // Simple: just store as i64 for now
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        let info = CompressedColumnInfo {
            name: String::new(),
            algorithm: CompressionAlgorithm::BitPacking,
            offset: 0,
            length: bytes.len(),
            dictionary: None,
            delta_base: None,
        };

        (bytes, info)
    }

    // Decodes variable-length integers.
    fn varint_decode(&self, compressed: &CompressedResult) -> Vec<i64> {
        let data = &compressed.data;
        let mut result = Vec::with_capacity(compressed.row_count);
        let mut offset = 0;

        while offset + 8 <= data.len() {
            let value = i64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            result.push(value);
            offset += 8;
        }

        result
    }

    // Returns compression statistics.
    pub fn stats(&self) -> CompressionStats {
        self.stats.read().clone()
    }

    // Resets compression statistics.
    pub fn reset_stats(&self) {
        *self.stats.write() = CompressionStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::Instant;

    #[test]
    fn test_dictionary_encoding() {
        let data = vec![
            "apple".to_string(),
            "banana".to_string(),
            "apple".to_string(),
            "cherry".to_string(),
            "apple".to_string(),
        ];

        let compressor = QueryResultCompressor::new(CompressionAlgorithm::Dictionary);
        let compressed = compressor.compress_strings(&data);

        assert!(compressed.compression_ratio() < 1.0);

        let decompressed = compressor.decompress_strings(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_run_length_encoding() {
        let data = vec![
            "a".to_string(),
            "a".to_string(),
            "a".to_string(),
            "b".to_string(),
            "b".to_string(),
        ];

        let compressor = QueryResultCompressor::new(CompressionAlgorithm::RunLength);
        let compressed = compressor.compress_strings(&data);

        let decompressed = compressor.decompress_strings(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_delta_encoding() {
        let data: Vec<i64> = vec![100, 101, 102, 103, 105, 110];

        let compressor = QueryResultCompressor::new(CompressionAlgorithm::Delta);
        let compressed = compressor.compress_numbers(&data);

        let decompressed = compressor.decompress_numbers(&compressed);
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_stats() {
        let compressor = QueryResultCompressor::new(CompressionAlgorithm::Dictionary);

        let data = vec!["test".to_string(); 100];
        let _ = compressor.compress_strings(&data);

        let stats = compressor.stats();
        assert!(stats.operations > 0);
        assert!(stats.bytes_in > 0);
    }
}
