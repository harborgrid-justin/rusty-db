// Column-Oriented Encodings - BitPacker, FOR, Delta, RLE

use crate::compression::{CompressionResult, CompressionStats, CompressionError};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// BitPacker - SIMD-ready bit packing
// ============================================================================

pub struct BitPacker {
    stats: Arc<Mutex<CompressionStats>>,
}

impl BitPacker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    pub fn bits_needed(max_value: u32) -> u8 {
        if max_value == 0 {
            1
        } else {
            32 - max_value.leading_zeros() as u8
        }
    }

    pub fn pack_u32(&self, values: &[u32], bit_width: u8) -> Vec<u8> {
        if values.is_empty() || bit_width == 0 || bit_width > 32 {
            return Vec::new();
        }

        let total_bits = values.len() * bit_width as usize;
        let total_bytes = (total_bits + 7) / 8;
        let mut packed = vec![0u8; total_bytes];

        let mut bit_pos = 0u64;
        for &value in values {
            let masked_value = value & ((1u32 << bit_width) - 1);

            for bit in 0..bit_width {
                let bit_val = (masked_value >> bit) & 1;
                if bit_val == 1 {
                    let byte_idx = (bit_pos / 8) as usize;
                    let bit_idx = (bit_pos % 8) as u8;
                    packed[byte_idx] |= 1 << bit_idx;
                }
                bit_pos += 1;
            }
        }

        packed
    }

    pub fn unpack_u32(&self, packed: &[u8], bit_width: u8, count: usize) -> Vec<u32> {
        if packed.is_empty() || bit_width == 0 || bit_width > 32 {
            return Vec::new();
        }

        let mut values = Vec::with_capacity(count);
        let mut bit_pos = 0u64;

        for _ in 0..count {
            let mut value = 0u32;

            for bit in 0..bit_width {
                let byte_idx = (bit_pos / 8) as usize;
                let bit_idx = (bit_pos % 8) as u8;

                if byte_idx >= packed.len() {
                    break;
                }

                if (packed[byte_idx] >> bit_idx) & 1 == 1 {
                    value |= 1 << bit;
                }
                bit_pos += 1;
            }

            values.push(value);
        }

        values
    }

    pub fn unpack_u32_fast(&self, packed: &[u8], bit_width: u8, count: usize) -> Vec<u32> {
        match bit_width {
            1 => self.unpack_1bit(packed, count),
            2 => self.unpack_2bit(packed, count),
            4 => self.unpack_4bit(packed, count),
            8 => self.unpack_8bit(packed, count),
            16 => self.unpack_16bit(packed, count),
            32 => self.unpack_32bit(packed, count),
            _ => self.unpack_u32(packed, bit_width, count),
        }
    }

    fn unpack_1bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        let mut values = Vec::with_capacity(count);
        for &byte in packed.iter() {
            for bit in 0..8 {
                if values.len() >= count {
                    break;
                }
                values.push(((byte >> bit) & 1) as u32);
            }
            if values.len() >= count {
                break;
            }
        }
        values
    }

    fn unpack_2bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        let mut values = Vec::with_capacity(count);
        for &byte in packed.iter() {
            for shift in [0, 2, 4, 6] {
                if values.len() >= count {
                    break;
                }
                values.push(((byte >> shift) & 0b11) as u32);
            }
            if values.len() >= count {
                break;
            }
        }
        values
    }

    fn unpack_4bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        let mut values = Vec::with_capacity(count);
        for &byte in packed.iter() {
            if values.len() < count {
                values.push((byte & 0x0F) as u32);
            }
            if values.len() < count {
                values.push(((byte >> 4) & 0x0F) as u32);
            }
            if values.len() >= count {
                break;
            }
        }
        values
    }

    fn unpack_8bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        packed.iter().take(count).map(|&b| b as u32).collect()
    }

    fn unpack_16bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        let mut values = Vec::with_capacity(count);
        for chunk in packed.chunks_exact(2).take(count) {
            values.push(u16::from_le_bytes([chunk[0], chunk[1]]) as u32);
        }
        values
    }

    fn unpack_32bit(&self, packed: &[u8], count: usize) -> Vec<u32> {
        let mut values = Vec::with_capacity(count);
        for chunk in packed.chunks_exact(4).take(count) {
            values.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        values
    }
}

impl Default for BitPacker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Frame-of-Reference (FOR) Encoder
// ============================================================================

pub struct FOREncoder {
    stats: Arc<Mutex<CompressionStats>>,
}

impl FOREncoder {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    pub fn encode(&self, values: &[u32]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if values.is_empty() {
            return Ok(Vec::new());
        }

        let min_val = *values.iter().min().unwrap();
        let max_val = *values.iter().max().unwrap();

        let range = max_val.saturating_sub(min_val);
        let bit_width = BitPacker::bits_needed(range);

        let mut encoded = Vec::with_capacity(9 + (values.len() * bit_width as usize + 7) / 8);
        encoded.extend_from_slice(&min_val.to_le_bytes());
        encoded.push(bit_width);
        encoded.extend_from_slice(&(values.len() as u32).to_le_bytes());

        let relative_values: Vec<u32> = values.iter()
            .map(|&v| v.saturating_sub(min_val))
            .collect();

        let packer = BitPacker::new();
        let packed = packer.pack_u32(&relative_values, bit_width);
        encoded.extend_from_slice(&packed);

        let mut stats = self.stats.lock();
        stats.uncompressed_size += values.len() * 4;
        stats.compressed_size += encoded.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(encoded)
    }

    pub fn decode(&self, encoded: &[u8]) -> CompressionResult<Vec<u32>> {
        let start = Instant::now();

        if encoded.len() < 9 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid FOR encoding".to_string()
            ));
        }

        let min_val = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        let bit_width = encoded[4];
        let count = u32::from_le_bytes([encoded[5], encoded[6], encoded[7], encoded[8]]) as usize;

        let packed = &encoded[9..];
        let packer = BitPacker::new();
        let relative_values = packer.unpack_u32_fast(packed, bit_width, count);

        let values: Vec<u32> = relative_values.iter()
            .map(|&v| v.saturating_add(min_val))
            .collect();

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(values)
    }
}

impl Default for FOREncoder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Delta Encoder
// ============================================================================

pub struct DeltaEncoder {
    stats: Arc<Mutex<CompressionStats>>,
}

impl DeltaEncoder {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    pub fn encode(&self, values: &[u32]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if values.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = Vec::with_capacity(8 + values.len() * 2);
        encoded.extend_from_slice(&values[0].to_le_bytes());
        encoded.extend_from_slice(&(values.len() as u32).to_le_bytes());

        if values.len() == 1 {
            return Ok(encoded);
        }

        let mut deltas = Vec::with_capacity(values.len() - 1);
        for i in 1..values.len() {
            let delta = values[i].wrapping_sub(values[i - 1]);
            deltas.push(delta);
        }

        let max_delta = *deltas.iter().max().unwrap_or(&0);
        let bit_width = BitPacker::bits_needed(max_delta);

        encoded.push(bit_width);

        let packer = BitPacker::new();
        let packed = packer.pack_u32(&deltas, bit_width);
        encoded.extend_from_slice(&packed);

        let mut stats = self.stats.lock();
        stats.uncompressed_size += values.len() * 4;
        stats.compressed_size += encoded.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(encoded)
    }

    pub fn decode(&self, encoded: &[u8]) -> CompressionResult<Vec<u32>> {
        let start = Instant::now();

        if encoded.len() < 8 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid delta encoding".to_string()
            ));
        }

        let base_value = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        let count = u32::from_le_bytes([encoded[4], encoded[5], encoded[6], encoded[7]]) as usize;

        if count == 1 {
            return Ok(vec![base_value]);
        }

        let bit_width = encoded[8];
        let packed = &encoded[9..];

        let packer = BitPacker::new();
        let deltas = packer.unpack_u32_fast(packed, bit_width, count - 1);

        let mut values = Vec::with_capacity(count);
        values.push(base_value);

        for delta in deltas {
            let prev = *values.last().unwrap();
            values.push(prev.wrapping_add(delta));
        }

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(values)
    }
}

impl Default for DeltaEncoder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Run-Length Encoder (RLE)
// ============================================================================

pub struct RLEEncoder {
    stats: Arc<Mutex<CompressionStats>>,
}

impl RLEEncoder {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    pub fn encode(&self, data: &[u8]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = Vec::with_capacity(data.len() / 2);
        let mut i = 0;

        while i < data.len() {
            let current = data[i];
            let mut run_length = 1usize;

            while i + run_length < data.len() &&
                  data[i + run_length] == current &&
                  run_length < 255 {
                run_length += 1;
            }

            encoded.push(current);
            encoded.push(run_length as u8);

            i += run_length;
        }

        let mut stats = self.stats.lock();
        stats.uncompressed_size += data.len();
        stats.compressed_size += encoded.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(encoded)
    }

    pub fn decode(&self, encoded: &[u8]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if encoded.len() % 2 != 0 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid RLE encoding".to_string()
            ));
        }

        let mut decoded = Vec::new();

        for chunk in encoded.chunks_exact(2) {
            let value = chunk[0];
            let run_length = chunk[1] as usize;

            decoded.extend(std::iter::repeat(value).take(run_length));
        }

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(decoded)
    }

    pub fn encode_u32(&self, values: &[u32]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if values.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&(values.len() as u32).to_le_bytes());

        let mut i = 0;

        while i < values.len() {
            let current = values[i];
            let mut run_length = 1usize;

            while i + run_length < values.len() &&
                  values[i + run_length] == current &&
                  run_length < 65535 {
                run_length += 1;
            }

            encoded.extend_from_slice(&current.to_le_bytes());
            encoded.extend_from_slice(&(run_length as u16).to_le_bytes());

            i += run_length;
        }

        let mut stats = self.stats.lock();
        stats.uncompressed_size += values.len() * 4;
        stats.compressed_size += encoded.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(encoded)
    }

    pub fn decode_u32(&self, encoded: &[u8]) -> CompressionResult<Vec<u32>> {
        let start = Instant::now();

        if encoded.len() < 4 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid RLE encoding".to_string()
            ));
        }

        let count = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        let mut decoded = Vec::with_capacity(count);
        let mut pos = 4;

        while pos + 6 <= encoded.len() && decoded.len() < count {
            let value = u32::from_le_bytes([
                encoded[pos], encoded[pos + 1], encoded[pos + 2], encoded[pos + 3]
            ]);
            let run_length = u16::from_le_bytes([encoded[pos + 4], encoded[pos + 5]]) as usize;

            decoded.extend(std::iter::repeat(value).take(run_length.min(count - decoded.len())));
            pos += 6;
        }

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(decoded)
    }
}

impl Default for RLEEncoder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Enhanced Dictionary Encoder
// ============================================================================

pub struct EnhancedDictionaryEncoder {
    stats: Arc<Mutex<CompressionStats>>,
}

impl EnhancedDictionaryEncoder {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    pub fn encode(&self, data: &[Vec<u8>]) -> CompressionResult<Vec<u8>> {
        let start = Instant::now();

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut dictionary = HashMap::new();
        let mut dict_entries = Vec::new();

        for item in data {
            if !dictionary.contains_key(item) {
                let index = dict_entries.len();
                dictionary.insert(item.clone(), index);
                dict_entries.push(item.clone());
            }
        }

        let bit_width = BitPacker::bits_needed(dict_entries.len() as u32);

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&(data.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&(dict_entries.len() as u32).to_le_bytes());
        encoded.push(bit_width);

        for entry in &dict_entries {
            encoded.extend_from_slice(&(entry.len() as u32).to_le_bytes());
            encoded.extend_from_slice(entry);
        }

        let indices: Vec<u32> = data.iter()
            .map(|item| dictionary[item] as u32)
            .collect();

        let packer = BitPacker::new();
        let packed_indices = packer.pack_u32(&indices, bit_width);
        encoded.extend_from_slice(&packed_indices);

        let mut stats = self.stats.lock();
        stats.uncompressed_size += data.iter().map(|v| v.len()).sum::<usize>();
        stats.compressed_size += encoded.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(encoded)
    }

    pub fn decode(&self, encoded: &[u8]) -> CompressionResult<Vec<Vec<u8>>> {
        let start = Instant::now();

        if encoded.len() < 9 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid dictionary encoding".to_string()
            ));
        }

        let data_count = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        let dict_size = u32::from_le_bytes([encoded[4], encoded[5], encoded[6], encoded[7]]) as usize;
        let bit_width = encoded[8];

        let mut pos = 9;
        let mut dict_entries = Vec::with_capacity(dict_size);

        for _ in 0..dict_size {
            if pos + 4 > encoded.len() {
                return Err(CompressionError::DecompressionFailed(
                    "Truncated dictionary".to_string()
                ));
            }

            let entry_len = u32::from_le_bytes([
                encoded[pos], encoded[pos + 1], encoded[pos + 2], encoded[pos + 3]
            ]) as usize;
            pos += 4;

            if pos + entry_len > encoded.len() {
                return Err(CompressionError::DecompressionFailed(
                    "Truncated dictionary entry".to_string()
                ));
            }

            dict_entries.push(encoded[pos..pos + entry_len].to_vec());
            pos += entry_len;
        }

        let packed_indices = &encoded[pos..];
        let packer = BitPacker::new();
        let indices = packer.unpack_u32_fast(packed_indices, bit_width, data_count);

        let mut data = Vec::with_capacity(data_count);
        for index in indices {
            if index as usize >= dict_entries.len() {
                return Err(CompressionError::DecompressionFailed(
                    format!("Invalid dictionary index: {}", index)
                ));
            }
            data.push(dict_entries[index as usize].clone());
        }

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(data)
    }
}

impl Default for EnhancedDictionaryEncoder {
    fn default() -> Self {
        Self::new()
    }
}
