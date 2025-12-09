/// Adaptive Compression - Intelligently selects best algorithm

use crate::compression::{Compressor, CompressionLevel, CompressionAlgorithm, CompressionResult, CompressionStats, CompressionError, utils};
use super::lz4_compression::LZ4Compressor;
use super::zstd_compression::{ZstdCompressor, HuffmanCompressor};
use super::dictionary_compression::DictionaryCompressor;
use super::column_encodings::*;
use parking_lot::Mutex;
use std::sync::Arc;

// ============================================================================
// Adaptive Compressor - Selects best algorithm dynamically
// ============================================================================

pub struct AdaptiveCompressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
    compressors: Vec<Box<dyn Compressor>>,
}

impl AdaptiveCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        let compressors: Vec<Box<dyn Compressor>> = vec![
            Box::new(LZ4Compressor::new(level)),
            Box::new(ZstdCompressor::new(level)),
            Box::new(DictionaryCompressor::new(level)),
            Box::new(HuffmanCompressor::new(level)),
        ];

        Self {
            level,
            stats: Arc::new(Mutex::new(CompressionStats::new())),
            compressors,
        }
    }

    fn select_best_compressor(&self, input: &[u8]) -> usize {
        let compressibility = utils::estimate_compressibility(input);

        if input.len() < 1024 {
            0 // LZ4 for small data
        } else if compressibility < 1.5 {
            0 // LZ4 for low compressibility
        } else if compressibility < 2.5 {
            1 // Zstd for medium compressibility
        } else if input.iter().collect::<std::collections::HashSet<_>>().len() < 64 {
            2 // Dictionary for few unique bytes
        } else {
            3 // Huffman for high compressibility
        }
    }
}

impl Compressor for AdaptiveCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        if input.is_empty() {
            return Ok(0);
        }

        let compressor_idx = self.select_best_compressor(input);
        output[0] = compressor_idx as u8;

        let compressed_size = self.compressors[compressor_idx].compress(input, &mut output[1..])?;

        let mut stats = self.stats.lock();
        stats.uncompressed_size += input.len();
        stats.compressed_size += compressed_size + 1;
        stats.blocks_compressed += 1;

        Ok(compressed_size + 1)
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        if input.is_empty() {
            return Ok(0);
        }

        let compressor_idx = input[0] as usize;

        if compressor_idx >= self.compressors.len() {
            return Err(CompressionError::UnsupportedAlgorithm(
                format!("Invalid algorithm index: {}", compressor_idx)
            ));
        }

        let decompressed_size = self.compressors[compressor_idx].decompress(&input[1..], output)?;

        let mut stats = self.stats.lock();
        stats.blocks_decompressed += 1;

        Ok(decompressed_size)
    }

    fn max_compressed_size(&self, input_size: usize) -> usize {
        input_size * 2 + 1024 + 1
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Adaptive
    }

    fn level(&self) -> CompressionLevel {
        self.level
    }

    fn stats(&self) -> CompressionStats {
        self.stats.lock().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock() = CompressionStats::new();
    }
}

// ============================================================================
// Cascaded Compressor - Intelligently selects best encoding for columns
// ============================================================================

pub struct CascadedCompressor {
    stats: Arc<Mutex<CompressionStats>>,
    for_encoder: FOREncoder,
    delta_encoder: DeltaEncoder,
    rle_encoder: RLEEncoder,
    lz4_compressor: LZ4Compressor,
}

impl CascadedCompressor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CompressionStats::new())),
            for_encoder: FOREncoder::new(),
            delta_encoder: DeltaEncoder::new(),
            rle_encoder: RLEEncoder::new(),
            lz4_compressor: LZ4Compressor::new(CompressionLevel::Fast),
        }
    }

    pub fn select_best_encoding_u32(&self, values: &[u32]) -> u8 {
        if values.is_empty() {
            return 0;
        }

        let unique_count = values.iter().collect::<std::collections::HashSet<_>>().len();
        if unique_count < values.len() / 4 {
            return 3; // RLE
        }

        let mut is_sorted = true;
        let mut is_monotonic = true;
        for i in 1..values.len() {
            if values[i] < values[i - 1] {
                is_sorted = false;
                break;
            }
            if values[i] != values[i - 1] && values[i] != values[i - 1] + 1 {
                is_monotonic = false;
            }
        }

        if is_monotonic {
            return 2; // Delta
        }

        if is_sorted {
            let min_val = *values.iter().min().unwrap();
            let max_val = *values.iter().max().unwrap();
            let range = max_val.saturating_sub(min_val);
            let bit_width = BitPacker::bits_needed(range);

            if bit_width <= 16 {
                return 1; // FOR
            }
        }

        4 // LZ4
    }

    pub fn compress_u32(&self, values: &[u32]) -> CompressionResult<Vec<u8>> {
        let encoding = self.select_best_encoding_u32(values);

        let mut result = Vec::new();
        result.push(encoding);

        let compressed = match encoding {
            1 => self.for_encoder.encode(values)?,
            2 => self.delta_encoder.encode(values)?,
            3 => self.rle_encoder.encode_u32(values)?,
            4 => {
                let mut bytes = Vec::with_capacity(values.len() * 4);
                for &v in values {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
                let mut output = vec![0u8; self.lz4_compressor.max_compressed_size(bytes.len())];
                let size = self.lz4_compressor.compress(&bytes, &mut output)?;
                output.truncate(size);
                output
            }
            _ => Vec::new(),
        };

        result.extend_from_slice(&compressed);

        let mut stats = self.stats.lock();
        stats.uncompressed_size += values.len() * 4;
        stats.compressed_size += result.len();
        stats.blocks_compressed += 1;

        Ok(result)
    }

    pub fn decompress_u32(&self, compressed: &[u8]) -> CompressionResult<Vec<u32>> {
        if compressed.is_empty() {
            return Ok(Vec::new());
        }

        let encoding = compressed[0];
        let data = &compressed[1..];

        let values = match encoding {
            1 => self.for_encoder.decode(data)?,
            2 => self.delta_encoder.decode(data)?,
            3 => self.rle_encoder.decode_u32(data)?,
            4 => {
                let estimated_size = data.len() * 4;
                let mut decompressed = vec![0u8; estimated_size];
                let size = self.lz4_compressor.decompress(data, &mut decompressed)?;
                decompressed.truncate(size);

                let mut values = Vec::new();
                for chunk in decompressed.chunks_exact(4) {
                    values.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
                values
            }
            _ => return Err(CompressionError::UnsupportedAlgorithm(
                format!("Unknown encoding: {}", encoding)
            )),
        };

        let mut stats = self.stats.lock();
        stats.blocks_decompressed += 1;

        Ok(values)
    }
}

impl Default for CascadedCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_compression() {
        let compressor = AdaptiveCompressor::new(CompressionLevel::Default);
        let input = b"The quick brown fox jumps over the lazy dog. The quick brown fox.";
        let mut compressed = vec![0u8; compressor.max_compressed_size(input.len())];
        let mut decompressed = vec![0u8; input.len()];

        let comp_size = compressor.compress(input, &mut compressed).unwrap();
        let decomp_size = compressor.decompress(&compressed[..comp_size], &mut decompressed).unwrap();

        assert_eq!(&decompressed[..decomp_size], input);
    }

    #[test]
    fn test_cascaded_compressor() {
        let compressor = CascadedCompressor::new();

        let sorted = vec![100, 101, 102, 103, 104, 105];
        let compressed = compressor.compress_u32(&sorted).unwrap();
        let decompressed = compressor.decompress_u32(&compressed).unwrap();
        assert_eq!(sorted, decompressed);
        assert_eq!(compressed[0], 1); // FOR encoding

        let monotonic = (0..100).collect::<Vec<u32>>();
        let compressed = compressor.compress_u32(&monotonic).unwrap();
        let decompressed = compressor.decompress_u32(&compressed).unwrap();
        assert_eq!(monotonic, decompressed);
        assert_eq!(compressed[0], 2); // Delta encoding

        let repetitive = vec![42; 50];
        let compressed = compressor.compress_u32(&repetitive).unwrap();
        let decompressed = compressor.decompress_u32(&compressed).unwrap();
        assert_eq!(repetitive, decompressed);
        assert_eq!(compressed[0], 3); // RLE encoding
    }
}
