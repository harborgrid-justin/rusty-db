/// Dictionary-Based Compression

use crate::compression::{Compressor, CompressionLevel, CompressionAlgorithm, CompressionResult, CompressionStats, CompressionError};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct DictionaryCompressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
    dictionary: Arc<HashMap<Vec<u8>, u16>>,
    reverse_dict: Arc<HashMap<u16, Vec<u8>>>,
    next_code: u16,
}

impl DictionaryCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        let mut dictionary = HashMap::new();
        let mut reverse_dict = HashMap::new();

        for i in 0..=255u16 {
            dictionary.insert(vec![i as u8], i);
            reverse_dict.insert(i, vec![i as u8]);
        }

        Self {
            level,
            stats: Arc::new(Mutex::new(CompressionStats::new())),
            dictionary: Arc::new(dictionary),
            reverse_dict: Arc::new(reverse_dict),
            next_code: 256,
        }
    }

    fn compress_lzw(&self, input: &[u8]) -> Vec<u8> {
        let mut dict = self.dictionary.as_ref().clone();
        let mut next_code = self.next_code;
        let mut result = Vec::new();
        let mut current = Vec::new();

        for &byte in input {
            let mut temp = current.clone();
            temp.push(byte);

            if dict.contains_key(&temp) {
                current = temp;
            } else {
                if let Some(&code) = dict.get(&current) {
                    result.extend_from_slice(&code.to_le_bytes());
                }

                if next_code < u16::MAX {
                    dict.insert(temp, next_code);
                    next_code += 1;
                }

                current = vec![byte];
            }
        }

        if !current.is_empty() {
            if let Some(&code) = dict.get(&current) {
                result.extend_from_slice(&code.to_le_bytes());
            }
        }

        result
    }

    fn decompress_lzw(&self, input: &[u8]) -> CompressionResult<Vec<u8>> {
        if input.len() % 2 != 0 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid compressed data length".to_string()
            ));
        }

        let mut dict = self.reverse_dict.as_ref().clone();
        let mut next_code = self.next_code;
        let mut result = Vec::new();
        let mut previous = Vec::new();

        for chunk in input.chunks_exact(2) {
            let code = u16::from_le_bytes([chunk[0], chunk[1]]);

            let entry = if let Some(sequence) = dict.get(&code) {
                sequence.clone()
            } else if code == next_code && !previous.is_empty() {
                let mut temp = previous.clone();
                temp.push(previous[0]);
                temp
            } else {
                return Err(CompressionError::DecompressionFailed(
                    format!("Invalid code: {}", code)
                ));
            };

            result.extend_from_slice(&entry);

            if !previous.is_empty() && next_code < u16::MAX {
                let mut new_entry = previous.clone();
                new_entry.push(entry[0]);
                dict.insert(next_code, new_entry);
                next_code += 1;
            }

            previous = entry;
        }

        Ok(result)
    }
}

impl Compressor for DictionaryCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        let start = Instant::now();

        if input.is_empty() {
            return Ok(0);
        }

        let compressed = self.compress_lzw(input);

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
        let decoded = self.decompress_lzw(input)?;

        if decoded.len() > output.len() {
            return Err(CompressionError::BufferTooSmall(decoded.len(), output.len()));
        }

        output[..decoded.len()].copy_from_slice(&decoded);

        let mut stats = self.stats.lock();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(decoded.len())
    }

    fn max_compressed_size(&self, input_size: usize) -> usize {
        input_size * 2 + 16
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Dictionary
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
