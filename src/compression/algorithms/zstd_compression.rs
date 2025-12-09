/// Zstandard-like and Huffman Compression Implementation

use crate::compression::{Compressor, CompressionLevel, CompressionAlgorithm, CompressionResult, CompressionStats, CompressionError};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

const MIN_MATCH_LENGTH: usize = 4;
const MAX_MATCH_LENGTH: usize = 255;

// ============================================================================
// Zstandard-like Compression
// ============================================================================

pub struct ZstdCompressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
    window_size: usize,
    dictionary: Option<Arc<Vec<u8>>>,
}

impl ZstdCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        let window_size = match level {
            CompressionLevel::None => 0,
            CompressionLevel::Fast => 1 << 16,
            CompressionLevel::Default => 1 << 20,
            CompressionLevel::Maximum => 1 << 22,
        };

        Self {
            level,
            stats: Arc::new(Mutex::new(CompressionStats::new())),
            window_size,
            dictionary: None,
        }
    }

    pub fn with_dictionary(mut self, dict: Vec<u8>) -> Self {
        self.dictionary = Some(Arc::new(dict));
        self
    }

    fn build_frequency_table(&self, data: &[u8]) -> [u32; 256] {
        let mut freq = [0u32; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }
        freq
    }

    fn entropy_encode(&self, data: &[u8], freq_table: &[u32; 256]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(data.len());

        for &f in freq_table.iter() {
            encoded.extend_from_slice(&f.to_le_bytes());
        }

        let mut pos = 0;
        let mut hash_table = HashMap::new();

        while pos < data.len() {
            let end = (pos + 8).min(data.len());
            if end - pos >= 4 {
                let sequence = &data[pos..end];

                if let Some(&prev_pos) = hash_table.get(sequence) {
                    let distance = pos - prev_pos;
                    let length = self.find_extended_match(data, pos, prev_pos);

                    if length >= MIN_MATCH_LENGTH && distance < self.window_size {
                        encoded.push(0xFF);
                        encoded.extend_from_slice(&(distance as u32).to_le_bytes());
                        encoded.push(length as u8);
                        pos += length;
                        continue;
                    }
                }

                hash_table.insert(sequence.to_vec(), pos);
            }

            encoded.push(data[pos]);
            pos += 1;
        }

        encoded
    }

    fn find_extended_match(&self, data: &[u8], pos1: usize, pos2: usize) -> usize {
        let max_len = (data.len() - pos1).min(MAX_MATCH_LENGTH);
        let mut len = 0;

        while len < max_len && data[pos1 + len] == data[pos2 + len] {
            len += 1;
        }

        len
    }

    fn entropy_decode(&self, data: &[u8]) -> CompressionResult<Vec<u8>> {
        if data.len() < 256 * 4 {
            return Err(CompressionError::DecompressionFailed(
                "Invalid compressed data".to_string()
            ));
        }

        let mut pos = 256 * 4;
        let mut decoded = Vec::new();

        while pos < data.len() {
            if data[pos] == 0xFF && pos + 5 < data.len() {
                pos += 1;
                let distance = u32::from_le_bytes([
                    data[pos], data[pos + 1], data[pos + 2], data[pos + 3]
                ]) as usize;
                pos += 4;
                let length = data[pos] as usize;
                pos += 1;

                if distance > decoded.len() {
                    return Err(CompressionError::DecompressionFailed(
                        "Invalid match distance".to_string()
                    ));
                }

                let match_start = decoded.len() - distance;
                for i in 0..length {
                    decoded.push(decoded[match_start + i]);
                }
            } else {
                decoded.push(data[pos]);
                pos += 1;
            }
        }

        Ok(decoded)
    }
}

impl Compressor for ZstdCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        let start = Instant::now();

        if input.is_empty() {
            return Ok(0);
        }

        let freq_table = self.build_frequency_table(input);
        let compressed = self.entropy_encode(input, &freq_table);

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
        let decoded = self.entropy_decode(input)?;

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
        input_size + (input_size / 128) + 1024 + 256 * 4
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Zstandard
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
// Huffman Encoding
// ============================================================================

#[derive(Clone)]
struct HuffmanNode {
    symbol: Option<u8>,
    frequency: u32,
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
    fn new_leaf(symbol: u8, frequency: u32) -> Self {
        Self {
            symbol: Some(symbol),
            frequency,
            left: None,
            right: None,
        }
    }

    fn new_internal(left: HuffmanNode, right: HuffmanNode) -> Self {
        Self {
            symbol: None,
            frequency: left.frequency + right.frequency,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

pub struct HuffmanCompressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
}

impl HuffmanCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        Self {
            level,
            stats: Arc::new(Mutex::new(CompressionStats::new())),
        }
    }

    fn build_frequency_table(&self, data: &[u8]) -> [u32; 256] {
        let mut freq = [0u32; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }
        freq
    }

    fn build_huffman_tree(&self, frequencies: &[u32; 256]) -> Option<HuffmanNode> {
        let mut nodes: Vec<HuffmanNode> = frequencies
            .iter()
            .enumerate()
            .filter(|(_, &freq)| freq > 0)
            .map(|(symbol, &freq)| HuffmanNode::new_leaf(symbol as u8, freq))
            .collect();

        if nodes.is_empty() {
            return None;
        }

        while nodes.len() > 1 {
            nodes.sort_by(|a, b| b.frequency.cmp(&a.frequency));
            let right = nodes.pop().unwrap();
            let left = nodes.pop().unwrap();
            nodes.push(HuffmanNode::new_internal(left, right));
        }

        Some(nodes.pop().unwrap())
    }

    fn build_code_table(&self, root: &HuffmanNode) -> HashMap<u8, (u32, u8)> {
        let mut table = HashMap::new();
        self.build_codes_recursive(root, 0, 0, &mut table);
        table
    }

    fn build_codes_recursive(&self, node: &HuffmanNode, code: u32, bits: u8,
                             table: &mut HashMap<u8, (u32, u8)>) {
        if let Some(symbol) = node.symbol {
            table.insert(symbol, (code, bits));
            return;
        }

        if let Some(ref left) = node.left {
            self.build_codes_recursive(left, code << 1, bits + 1, table);
        }

        if let Some(ref right) = node.right {
            self.build_codes_recursive(right, (code << 1) | 1, bits + 1, table);
        }
    }

    fn encode_with_huffman(&self, data: &[u8], code_table: &HashMap<u8, (u32, u8)>) -> Vec<u8> {
        let mut bit_buffer = 0u64;
        let mut bits_in_buffer = 0u8;
        let mut result = Vec::new();

        for &byte in data {
            if let Some(&(code, code_bits)) = code_table.get(&byte) {
                bit_buffer = (bit_buffer << code_bits) | (code as u64);
                bits_in_buffer += code_bits;

                while bits_in_buffer >= 8 {
                    bits_in_buffer -= 8;
                    result.push((bit_buffer >> bits_in_buffer) as u8);
                    bit_buffer &= (1 << bits_in_buffer) - 1;
                }
            }
        }

        if bits_in_buffer > 0 {
            result.push((bit_buffer << (8 - bits_in_buffer)) as u8);
        }

        result
    }
}

impl Compressor for HuffmanCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        let start = Instant::now();

        if input.is_empty() {
            return Ok(0);
        }

        let frequencies = self.build_frequency_table(input);
        let tree = self.build_huffman_tree(&frequencies)
            .ok_or_else(|| CompressionError::CompressionFailed("Empty input".to_string()))?;
        let code_table = self.build_code_table(&tree);

        let mut compressed = Vec::new();

        for &freq in &frequencies {
            compressed.extend_from_slice(&freq.to_le_bytes());
        }

        let encoded = self.encode_with_huffman(input, &code_table);
        compressed.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        compressed.extend_from_slice(&encoded);

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

        if input.len() < 256 * 4 + 4 {
            return Err(CompressionError::DecompressionFailed("Invalid input".to_string()));
        }

        let mut frequencies = [0u32; 256];
        for i in 0..256 {
            frequencies[i] = u32::from_le_bytes([
                input[i * 4], input[i * 4 + 1], input[i * 4 + 2], input[i * 4 + 3]
            ]);
        }

        let tree = self.build_huffman_tree(&frequencies)
            .ok_or_else(|| CompressionError::DecompressionFailed("Invalid tree".to_string()))?;

        let encoded_len = u32::from_le_bytes([
            input[1024], input[1025], input[1026], input[1027]
        ]) as usize;

        let encoded_data = &input[1028..1028 + encoded_len];
        let mut decoded = Vec::new();
        let mut current_node = &tree;

        for &byte in encoded_data {
            for bit_pos in (0..8).rev() {
                let bit = (byte >> bit_pos) & 1;

                current_node = if bit == 0 {
                    current_node.left.as_ref().map(|b| b.as_ref())
                } else {
                    current_node.right.as_ref().map(|b| b.as_ref())
                }.ok_or_else(|| CompressionError::DecompressionFailed("Invalid bit".to_string()))?;

                if let Some(symbol) = current_node.symbol {
                    decoded.push(symbol);
                    current_node = &tree;
                }
            }
        }

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
        input_size + 1024 + 16
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Huffman
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
    fn test_huffman_compression() {
        let compressor = HuffmanCompressor::new(CompressionLevel::Default);
        let input = b"aaaaaabbbbcccdde";
        let mut compressed = vec![0u8; compressor.max_compressed_size(input.len())];
        let mut decompressed = vec![0u8; input.len() * 2];

        let comp_size = compressor.compress(input, &mut compressed).unwrap();
        let decomp_size = compressor.decompress(&compressed[..comp_size], &mut decompressed).unwrap();

        assert_eq!(&decompressed[..decomp_size], input);
    }
}
