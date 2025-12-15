// Advanced Compression Algorithms for Columnar Storage
//
// Implements multiple compression techniques optimized for columnar data:
// - Dictionary encoding
// - Run-length encoding (RLE)
// - Bit-packing
// - Delta encoding
// - Frame-of-reference compression
// - Hybrid compression with automatic selection

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::inmemory::column_store::{ColumnDataType, ColumnStats};

// Compression algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionType {
    None,
    Dictionary,
    RunLength,
    BitPacking,
    Delta,
    FrameOfReference,
    Hybrid,
}

// Statistics about compression
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub compression_time_us: u64,
    pub decompression_time_us: u64,
    pub algorithm_used: CompressionType,
}

impl CompressionStats {
    pub fn new(original_size: usize, compressed_size: usize) -> Self {
        Self {
            original_size,
            compressed_size,
            compression_ratio: original_size as f64 / compressed_size as f64,
            compression_time_us: 0,
            decompression_time_us: 0,
            algorithm_used: CompressionType::None,
        }
    }
}

// Result of compression operation
pub struct CompressedData {
    pub compressed_data: Vec<u8>,
    pub compression_type: CompressionType,
    pub stats: CompressionStats,
    pub metadata: Vec<u8>,
}

// Trait for compression algorithms
pub trait CompressionAlgorithm: Send + Sync {
    fn compress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String>;
    fn decompress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String>;
    fn estimate_ratio(&self, data: &[u8], stats: &ColumnStats) -> f64;
}

// Dictionary encoding for low-cardinality columns
pub struct DictionaryEncoder {
    max_dictionary_size: usize,
}

impl DictionaryEncoder {
    pub fn new(max_dictionary_size: usize) -> Self {
        Self {
            max_dictionary_size,
        }
    }

    fn compress_int64(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() % 8 != 0 {
            return Err("Data length must be multiple of 8 for Int64".to_string());
        }

        let count = data.len() / 8;
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            values.push(i64::from_le_bytes(bytes));
        }

        // Build dictionary
        let mut dict_map: HashMap<i64, u32> = HashMap::new();
        let mut dict_values = Vec::new();

        for &val in &values {
            if !dict_map.contains_key(&val) {
                if dict_map.len() >= self.max_dictionary_size {
                    return Err("Dictionary size exceeded".to_string());
                }
                dict_map.insert(val, dict_values.len() as u32);
                dict_values.push(val);
            }
        }

        // Encode data
        let mut encoded = Vec::new();

        // Write dictionary size
        encoded.extend_from_slice(&(dict_values.len() as u32).to_le_bytes());

        // Write dictionary values
        for val in dict_values {
            encoded.extend_from_slice(&val.to_le_bytes());
        }

        // Write encoded indices (use minimal bit width)
        let bits_per_index = if dict_map.len() <= 256 {
            8
        } else if dict_map.len() <= 65536 {
            16
        } else {
            32
        };

        encoded.push(bits_per_index);

        for val in values {
            let index = dict_map[&val];
            match bits_per_index {
                8 => encoded.push(index as u8),
                16 => encoded.extend_from_slice(&(index as u16).to_le_bytes()),
                32 => encoded.extend_from_slice(&index.to_le_bytes()),
                _ => unreachable!(),
            }
        }

        Ok(encoded)
    }

    fn decompress_int64(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 5 {
            return Err("Invalid dictionary-encoded data".to_string());
        }

        let mut offset = 0;

        // Read dictionary size
        let dict_size = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        // Read dictionary values
        let mut dict_values = Vec::with_capacity(dict_size);
        for _ in 0..dict_size {
            if offset + 8 > data.len() {
                return Err("Truncated dictionary data".to_string());
            }
            let val = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            dict_values.push(val);
            offset += 8;
        }

        // Read bits per index
        if offset >= data.len() {
            return Err("Missing bits per index".to_string());
        }
        let bits_per_index = data[offset];
        offset += 1;

        // Decode indices
        let mut decoded = Vec::new();

        while offset < data.len() {
            let index = match bits_per_index {
                8 => {
                    let idx = data[offset] as u32;
                    offset += 1;
                    idx
                }
                16 => {
                    if offset + 2 > data.len() {
                        break;
                    }
                    let idx =
                        u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as u32;
                    offset += 2;
                    idx
                }
                32 => {
                    if offset + 4 > data.len() {
                        break;
                    }
                    let idx = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    idx
                }
                _ => return Err("Invalid bits per index".to_string()),
            };

            if (index as usize) >= dict_values.len() {
                return Err("Index out of bounds".to_string());
            }

            let val = dict_values[index as usize];
            decoded.extend_from_slice(&val.to_le_bytes());
        }

        Ok(decoded)
    }
}

impl CompressionAlgorithm for DictionaryEncoder {
    fn compress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        match data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 => self.compress_int64(data),
            _ => Err(format!(
                "Dictionary encoding not supported for {:?}",
                data_type
            )),
        }
    }

    fn decompress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        match data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 => self.decompress_int64(data),
            _ => Err(format!(
                "Dictionary encoding not supported for {:?}",
                data_type
            )),
        }
    }

    fn estimate_ratio(&self, data: &[u8], stats: &ColumnStats) -> f64 {
        if let Some(distinct_count) = stats.distinct_count {
            if distinct_count < 1000 && distinct_count < data.len() / 16 {
                return 4.0; // Good compression expected
            }
        }
        1.0 // No compression benefit
    }
}

// Run-length encoding for repeated values
pub struct RunLengthEncoder {
    min_run_length: usize,
}

impl RunLengthEncoder {
    pub fn new(min_run_length: usize) -> Self {
        Self { min_run_length }
    }

    fn encode_int64(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() % 8 != 0 {
            return Err("Data length must be multiple of 8".to_string());
        }

        let count = data.len() / 8;
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            values.push(i64::from_le_bytes(bytes));
        }

        let mut encoded = Vec::new();

        let mut i = 0;
        while i < values.len() {
            let value = values[i];
            let mut run_length = 1;

            while i + run_length < values.len() && values[i + run_length] == value {
                run_length += 1;
            }

            if run_length >= self.min_run_length {
                // Encode as run: [marker=0xFF][value][run_length]
                encoded.push(0xFF);
                encoded.extend_from_slice(&value.to_le_bytes());
                encoded.extend_from_slice(&(run_length as u32).to_le_bytes());
            } else {
                // Encode literal values: [marker=count][values...]
                let literal_count = run_length.min(254);
                encoded.push(literal_count as u8);
                for j in 0..literal_count {
                    encoded.extend_from_slice(&values[i + j].to_le_bytes());
                }
                run_length = literal_count;
            }

            i += run_length;
        }

        Ok(encoded)
    }

    fn decode_int64(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decoded = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            if offset >= data.len() {
                break;
            }

            let marker = data[offset];
            offset += 1;

            if marker == 0xFF {
                // Run-length encoded
                if offset + 12 > data.len() {
                    return Err("Truncated RLE data".to_string());
                }

                let value = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;

                let run_length =
                    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;

                for _ in 0..run_length {
                    decoded.extend_from_slice(&value.to_le_bytes());
                }
            } else {
                // Literal values
                let count = marker as usize;
                if offset + count * 8 > data.len() {
                    return Err("Truncated literal data".to_string());
                }

                for _ in 0..count {
                    decoded.extend_from_slice(&data[offset..offset + 8]);
                    offset += 8;
                }
            }
        }

        Ok(decoded)
    }
}

impl CompressionAlgorithm for RunLengthEncoder {
    fn compress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        match data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 => self.encode_int64(data),
            _ => Err(format!("RLE not supported for {:?}", data_type)),
        }
    }

    fn decompress(&self, data: &[u8], data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        match data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 => self.decode_int64(data),
            _ => Err(format!("RLE not supported for {:?}", data_type)),
        }
    }

    fn estimate_ratio(&self, _data: &[u8], stats: &ColumnStats) -> f64 {
        if let Some(distinct) = stats.distinct_count {
            if distinct < stats.row_count / 10 {
                return 3.0; // Good compression for low cardinality
            }
        }
        1.0
    }
}

// Bit-packing for low-cardinality integer columns
pub struct BitPacker {
    max_bits: u8,
}

impl BitPacker {
    pub fn new(max_bits: u8) -> Self {
        Self { max_bits }
    }

    fn calculate_bits_needed(&self, data: &[u8]) -> u8 {
        if data.len() % 8 != 0 {
            return 64;
        }

        let count = data.len() / 8;
        let mut max_value = 0i64;

        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            let value = i64::from_le_bytes(bytes);
            max_value = max_value.max(value);
        }

        if max_value <= 0 {
            return 1;
        }

        (64 - max_value.leading_zeros()) as u8
    }

    fn pack(&self, data: &[u8], bits_per_value: u8) -> Result<Vec<u8>, String> {
        if data.len() % 8 != 0 {
            return Err("Data length must be multiple of 8".to_string());
        }

        let count = data.len() / 8;
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            values.push(i64::from_le_bytes(bytes) as u64);
        }

        let mut packed = Vec::new();

        // Write metadata
        packed.extend_from_slice(&(count as u32).to_le_bytes());
        packed.push(bits_per_value);

        // Pack values
        let mut bit_buffer = 0u64;
        let mut bits_in_buffer = 0u8;

        for value in values {
            bit_buffer |= value << bits_in_buffer;
            bits_in_buffer += bits_per_value;

            while bits_in_buffer >= 8 {
                packed.push((bit_buffer & 0xFF) as u8);
                bit_buffer >>= 8;
                bits_in_buffer -= 8;
            }
        }

        // Flush remaining bits
        if bits_in_buffer > 0 {
            packed.push((bit_buffer & 0xFF) as u8);
        }

        Ok(packed)
    }

    fn unpack(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 5 {
            return Err("Invalid bit-packed data".to_string());
        }

        let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let bits_per_value = data[4];

        let mut unpacked = Vec::new();
        let mut bit_buffer = 0u64;
        let mut bits_in_buffer = 0u8;
        let mut offset = 5;

        let value_mask = if bits_per_value >= 64 {
            u64::MAX
        } else {
            (1u64 << bits_per_value) - 1
        };

        for _ in 0..count {
            while bits_in_buffer < bits_per_value && offset < data.len() {
                bit_buffer |= (data[offset] as u64) << bits_in_buffer;
                bits_in_buffer += 8;
                offset += 1;
            }

            let value = (bit_buffer & value_mask) as i64;
            unpacked.extend_from_slice(&value.to_le_bytes());

            bit_buffer >>= bits_per_value;
            bits_in_buffer -= bits_per_value;
        }

        Ok(unpacked)
    }
}

impl CompressionAlgorithm for BitPacker {
    fn compress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        let bits_needed = self.calculate_bits_needed(data).min(self.max_bits);
        self.pack(data, bits_needed)
    }

    fn decompress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        self.unpack(data)
    }

    fn estimate_ratio(&self, data: &[u8], _stats: &ColumnStats) -> f64 {
        let bits_needed = self.calculate_bits_needed(data);
        if bits_needed < 32 {
            return 64.0 / bits_needed as f64;
        }
        1.0
    }
}

// Delta encoding for sorted or monotonic columns
pub struct DeltaEncoder;

impl DeltaEncoder {
    pub fn new() -> Self {
        Self
    }

    fn encode(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() % 8 != 0 {
            return Err("Data length must be multiple of 8".to_string());
        }

        let count = data.len() / 8;
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut values = Vec::with_capacity(count);
        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            values.push(i64::from_le_bytes(bytes));
        }

        let mut encoded = Vec::new();

        // Write base value
        encoded.extend_from_slice(&values[0].to_le_bytes());

        // Write deltas
        for i in 1..values.len() {
            let delta = values[i] - values[i - 1];
            encoded.extend_from_slice(&delta.to_le_bytes());
        }

        Ok(encoded)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 8 || data.len() % 8 != 0 {
            return Err("Invalid delta-encoded data".to_string());
        }

        let count = data.len() / 8;
        let mut decoded = Vec::new();

        // Read base value
        let base = i64::from_le_bytes(data[0..8].try_into().unwrap());
        decoded.extend_from_slice(&base.to_le_bytes());

        let mut current = base;

        // Read and apply deltas
        for i in 1..count {
            let offset = i * 8;
            let delta = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            current += delta;
            decoded.extend_from_slice(&current.to_le_bytes());
        }

        Ok(decoded)
    }
}

impl CompressionAlgorithm for DeltaEncoder {
    fn compress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        self.encode(data)
    }

    fn decompress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        self.decode(data)
    }

    fn estimate_ratio(&self, _data: &[u8], _stats: &ColumnStats) -> f64 {
        1.5 // Moderate compression, better with bit-packing on deltas
    }
}

// Frame-of-reference compression
pub struct FrameOfReferenceEncoder {
    frame_size: usize,
}

impl FrameOfReferenceEncoder {
    pub fn new(frame_size: usize) -> Self {
        Self { frame_size }
    }

    fn encode(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() % 8 != 0 {
            return Err("Data length must be multiple of 8".to_string());
        }

        let count = data.len() / 8;
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 8;
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            values.push(i64::from_le_bytes(bytes));
        }

        let mut encoded = Vec::new();

        // Process in frames
        for chunk in values.chunks(self.frame_size) {
            if chunk.is_empty() {
                continue;
            }

            // Find min value in frame (reference)
            let min_val = *chunk.iter().min().unwrap();

            // Write frame header: [frame_size][reference_value]
            encoded.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            encoded.extend_from_slice(&min_val.to_le_bytes());

            // Write offsets from reference
            for &val in chunk {
                let offset = (val - min_val) as u64;
                encoded.extend_from_slice(&offset.to_le_bytes());
            }
        }

        Ok(encoded)
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decoded = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            if offset + 12 > data.len() {
                break;
            }

            // Read frame header
            let frame_size =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            let reference = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;

            // Read offsets
            for _ in 0..frame_size {
                if offset + 8 > data.len() {
                    return Err("Truncated frame data".to_string());
                }

                let frame_offset = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;

                let value = reference + frame_offset as i64;
                decoded.extend_from_slice(&value.to_le_bytes());
            }
        }

        Ok(decoded)
    }
}

impl CompressionAlgorithm for FrameOfReferenceEncoder {
    fn compress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        self.encode(data)
    }

    fn decompress(&self, data: &[u8], _data_type: ColumnDataType) -> Result<Vec<u8>, String> {
        self.decode(data)
    }

    fn estimate_ratio(&self, _data: &[u8], _stats: &ColumnStats) -> f64 {
        2.0 // Moderate compression
    }
}

// Hybrid compressor that selects best algorithm
pub struct HybridCompressor {
    dictionary: Arc<DictionaryEncoder>,
    rle: Arc<RunLengthEncoder>,
    bitpacker: Arc<BitPacker>,
    delta: Arc<DeltaEncoder>,
    for_encoder: Arc<FrameOfReferenceEncoder>,
    #[allow(dead_code)]
    cache: RwLock<HashMap<u64, CompressionType>>,
}

impl HybridCompressor {
    pub fn new() -> Self {
        Self {
            dictionary: Arc::new(DictionaryEncoder::new(65536)),
            rle: Arc::new(RunLengthEncoder::new(3)),
            bitpacker: Arc::new(BitPacker::new(32)),
            delta: Arc::new(DeltaEncoder::new()),
            for_encoder: Arc::new(FrameOfReferenceEncoder::new(128)),
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn select_algorithm(&self, data: &[u8], stats: &ColumnStats) -> CompressionType {
        let mut best_type = CompressionType::None;
        let mut best_ratio = 1.0;

        let algorithms: Vec<(CompressionType, &dyn CompressionAlgorithm)> = vec![
            (
                CompressionType::Dictionary,
                &*self.dictionary as &dyn CompressionAlgorithm,
            ),
            (
                CompressionType::RunLength,
                &*self.rle as &dyn CompressionAlgorithm,
            ),
            (
                CompressionType::BitPacking,
                &*self.bitpacker as &dyn CompressionAlgorithm,
            ),
            (
                CompressionType::Delta,
                &*self.delta as &dyn CompressionAlgorithm,
            ),
            (
                CompressionType::FrameOfReference,
                &*self.for_encoder as &dyn CompressionAlgorithm,
            ),
        ];

        for (comp_type, algorithm) in algorithms {
            let ratio = algorithm.estimate_ratio(data, stats);
            if ratio > best_ratio {
                best_ratio = ratio;
                best_type = comp_type;
            }
        }

        best_type
    }

    pub fn compress(
        &self,
        data: &[u8],
        data_type: ColumnDataType,
        stats: &ColumnStats,
    ) -> Result<CompressedData, String> {
        let start = std::time::Instant::now();
        let compression_type = self.select_algorithm(data, stats);

        let compressed = match compression_type {
            CompressionType::Dictionary => self.dictionary.compress(data, data_type)?,
            CompressionType::RunLength => self.rle.compress(data, data_type)?,
            CompressionType::BitPacking => self.bitpacker.compress(data, data_type)?,
            CompressionType::Delta => self.delta.compress(data, data_type)?,
            CompressionType::FrameOfReference => self.for_encoder.compress(data, data_type)?,
            CompressionType::None | CompressionType::Hybrid => data.to_vec(),
        };

        let elapsed = start.elapsed();

        let mut comp_stats = CompressionStats::new(data.len(), compressed.len());
        comp_stats.compression_time_us = elapsed.as_micros() as u64;
        comp_stats.algorithm_used = compression_type;

        Ok(CompressedData {
            compressed_data: compressed,
            compression_type,
            stats: comp_stats,
            metadata: Vec::new(),
        })
    }

    pub fn decompress(
        &self,
        data: &[u8],
        compression_type: CompressionType,
        data_type: ColumnDataType,
    ) -> Result<Vec<u8>, String> {
        let start = std::time::Instant::now();

        let decompressed = match compression_type {
            CompressionType::Dictionary => self.dictionary.decompress(data, data_type)?,
            CompressionType::RunLength => self.rle.decompress(data, data_type)?,
            CompressionType::BitPacking => self.bitpacker.decompress(data, data_type)?,
            CompressionType::Delta => self.delta.decompress(data, data_type)?,
            CompressionType::FrameOfReference => self.for_encoder.decompress(data, data_type)?,
            CompressionType::None | CompressionType::Hybrid => data.to_vec(),
        };

        let _elapsed = start.elapsed();

        Ok(decompressed)
    }
}

impl Default for HybridCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_encoding() {
        let encoder = DictionaryEncoder::new(1000);

        let mut data = Vec::new();
        for _ in 0..100 {
            data.extend_from_slice(&42i64.to_le_bytes());
        }
        for _ in 0..100 {
            data.extend_from_slice(&99i64.to_le_bytes());
        }

        let compressed = encoder.compress(&data, ColumnDataType::Int64).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = encoder
            .decompress(&compressed, ColumnDataType::Int64)
            .unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_run_length_encoding() {
        let encoder = RunLengthEncoder::new(3);

        let mut data = Vec::new();
        for _ in 0..100 {
            data.extend_from_slice(&5i64.to_le_bytes());
        }

        let compressed = encoder.compress(&data, ColumnDataType::Int64).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = encoder
            .decompress(&compressed, ColumnDataType::Int64)
            .unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_bit_packing() {
        let packer = BitPacker::new(32);

        let mut data = Vec::new();
        for i in 0..100 {
            data.extend_from_slice(&(i as i64).to_le_bytes());
        }

        let compressed = packer.compress(&data, ColumnDataType::Int64).unwrap();
        let decompressed = packer
            .decompress(&compressed, ColumnDataType::Int64)
            .unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_delta_encoding() {
        let encoder = DeltaEncoder::new();

        let mut data = Vec::new();
        for i in 0i64..100 {
            data.extend_from_slice(&(i * 10).to_le_bytes());
        }

        let compressed = encoder.compress(&data, ColumnDataType::Int64).unwrap();
        let decompressed = encoder
            .decompress(&compressed, ColumnDataType::Int64)
            .unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_frame_of_reference() {
        let encoder = FrameOfReferenceEncoder::new(10);

        let mut data = Vec::new();
        for i in 100i64..200 {
            data.extend_from_slice(&i.to_le_bytes());
        }

        let compressed = encoder.compress(&data, ColumnDataType::Int64).unwrap();
        let decompressed = encoder
            .decompress(&compressed, ColumnDataType::Int64)
            .unwrap();
        assert_eq!(data, decompressed);
    }
}
