// LZ4 Compression Implementation

use crate::compression::{Compressor, CompressionLevel, CompressionAlgorithm, CompressionResult, CompressionStats, CompressionError};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

const MIN_MATCH_LENGTH: usize = 4;
const MAX_MATCH_LENGTH: usize = 255;
const HASH_TABLE_SIZE: usize = 1 << 16;

pub struct LZ4Compressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
    #[allow(dead_code)]
    hash_table: Vec<usize>,
}

impl LZ4Compressor {
    pub fn new(level: CompressionLevel) -> Self {
        Self {
            level,
            stats: Arc::new(Mutex::new(CompressionStats::new())),
            hash_table: vec![0; HASH_TABLE_SIZE],
        }
    }

    fn hash_sequence(&self, data: &[u8], pos: usize) -> usize {
        if pos + 4 > data.len() {
            return 0;
        }
        let v = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        ((v.wrapping_mul(2654435761)) >> 16) as usize & (HASH_TABLE_SIZE - 1)
    }

    fn find_match(&self, data: &[u8], pos: usize, hash_pos: usize) -> (usize, usize) {
        if hash_pos >= pos {
            return (0, 0);
        }

        let max_match_len = (data.len() - pos).min(MAX_MATCH_LENGTH);
        let mut match_len = 0;

        while match_len < max_match_len && data[hash_pos + match_len] == data[pos + match_len] {
            match_len += 1;
        }

        if match_len >= MIN_MATCH_LENGTH {
            (pos - hash_pos, match_len)
        } else {
            (0, 0)
        }
    }

    fn encode_literal_run(&self, data: &[u8], start: usize, len: usize, output: &mut Vec<u8>) {
        let mut remaining = len;
        let mut pos = start;

        while remaining > 0 {
            let chunk = remaining.min(255);
            output.push(chunk as u8);
            output.extend_from_slice(&data[pos..pos + chunk]);
            pos += chunk;
            remaining -= chunk;
        }
    }

    fn encode_match(&self, distance: usize, length: usize, output: &mut Vec<u8>) {
        output.extend_from_slice(&(distance as u16).to_le_bytes());
        if length < 255 {
            output.push(length as u8);
        } else {
            output.push(255);
            output.extend_from_slice(&(length - 255).to_le_bytes());
        }
    }
}

impl Compressor for LZ4Compressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        let start = Instant::now();

        if input.is_empty() {
            return Ok(0);
        }

        let mut compressed = Vec::with_capacity(input.len());
        let mut hash_table = vec![0usize; HASH_TABLE_SIZE];
        let mut pos = 0;
        let mut literal_start = 0;

        while pos < input.len() {
            if pos + MIN_MATCH_LENGTH > input.len() {
                break;
            }

            let hash = self.hash_sequence(input, pos);
            let hash_pos = hash_table[hash];
            hash_table[hash] = pos;

            let (distance, match_len) = self.find_match(input, pos, hash_pos);

            if match_len >= MIN_MATCH_LENGTH {
                if pos > literal_start {
                    self.encode_literal_run(input, literal_start, pos - literal_start, &mut compressed);
                }

                self.encode_match(distance, match_len, &mut compressed);

                pos += match_len;
                literal_start = pos;
            } else {
                pos += 1;
            }
        }

        if literal_start < input.len() {
            self.encode_literal_run(input, literal_start, input.len() - literal_start, &mut compressed);
        }

        if compressed.len() > output.len() {
            return Err(CompressionError::BufferTooSmall(compressed.len(), output.len()));
        }

        output[..compressed.len()].copy_from_slice(&compressed);

        let mut stats = self.stats.lock();
        stats.uncompressed_size += input.len();
        stats.compressed_size += compressed.len();
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        Ok(compressed.len())
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        let start = Instant::now();
        let mut in_pos = 0;
        let mut out_pos = 0;

        while in_pos < input.len() {
            if in_pos >= input.len() {
                break;
            }

            let literal_len = input[in_pos] as usize;
            in_pos += 1;

            if literal_len > 0 {
                if in_pos + literal_len > input.len() || out_pos + literal_len > output.len() {
                    return Err(CompressionError::DecompressionFailed(
                        "Invalid literal length".to_string()
                    ));
                }
                output[out_pos..out_pos + literal_len].copy_from_slice(&input[in_pos..in_pos + literal_len]);
                in_pos += literal_len;
                out_pos += literal_len;
            }

            if in_pos + 3 <= input.len() {
                let distance = u16::from_le_bytes([input[in_pos], input[in_pos + 1]]) as usize;
                in_pos += 2;

                let mut match_len = input[in_pos] as usize;
                in_pos += 1;

                if match_len == 255 && in_pos + 4 <= input.len() {
                    match_len = 255 + usize::from_le_bytes([
                        input[in_pos], input[in_pos + 1], input[in_pos + 2], input[in_pos + 3],
                        0, 0, 0, 0
                    ]);
                    in_pos += 4;
                }

                if match_len > 0 && distance > 0 && distance <= out_pos {
                    let match_start = out_pos - distance;
                    for i in 0..match_len {
                        if out_pos >= output.len() {
                            return Err(CompressionError::BufferTooSmall(out_pos + 1, output.len()));
                        }
                        output[out_pos] = output[match_start + i];
                        out_pos += 1;
                    }
                }
            }
        }

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(out_pos)
    }

    fn max_compressed_size(&self, input_size: usize) -> usize {
        input_size + (input_size / 255) + 16
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::LZ4
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_compression() {
        let compressor = LZ4Compressor::new(CompressionLevel::Default);
        let input = b"Hello, World! Hello, World! This is a test.";
        let mut compressed = vec![0u8; compressor.max_compressed_size(input.len())];
        let mut decompressed = vec![0u8; input.len()];

        let comp_size = compressor.compress(input, &mut compressed).unwrap();
        let decomp_size = compressor.decompress(&compressed[..comp_size], &mut decompressed).unwrap();

        assert_eq!(&decompressed[..decomp_size], input);
    }
}
