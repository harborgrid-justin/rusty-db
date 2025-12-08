// Compression Algorithms - LZ4, Zstandard-like, Dictionary, Arithmetic, Huffman
// Implements various compression algorithms from scratch

use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MIN_MATCH_LENGTH: usize = 4;
const MAX_MATCH_LENGTH: usize = 255;
const HASH_TABLE_SIZE: usize = 1 << 16;

// ============================================================================
// LZ4 Compression Implementation
// ============================================================================

pub struct LZ4Compressor {
    level: CompressionLevel,
    stats: Arc<Mutex<CompressionStats>>,
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
        // Encode distance (2 bytes)
        output.extend_from_slice(&(distance as u16).to_le_bytes());
        // Encode length (1 byte, with extension for long matches)
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
                // Encode any pending literals
                if pos > literal_start {
                    self.encode_literal_run(input, literal_start, pos - literal_start, &mut compressed);
                }

                // Encode the match
                self.encode_match(distance, match_len, &mut compressed);

                pos += match_len;
                literal_start = pos;
            } else {
                pos += 1;
            }
        }

        // Encode remaining literals
        if literal_start < input.len() {
            self.encode_literal_run(input, literal_start, input.len() - literal_start, &mut compressed);
        }

        if compressed.len() > output.len() {
            return Err(CompressionError::BufferTooSmall(compressed.len(), output.len()));
        }

        output[..compressed.len()].copy_from_slice(&compressed);

        let mut stats = self.stats.lock().unwrap();
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
            // Read literal length
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
                // Read match distance and length
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

        let mut stats = self.stats.lock().unwrap();
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
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CompressionStats::new();
    }
}

// ============================================================================
// Zstandard-like Compression Implementation
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
            CompressionLevel::Fast => 1 << 16,  // 64KB
            CompressionLevel::Default => 1 << 20, // 1MB
            CompressionLevel::Maximum => 1 << 22, // 4MB
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

        // Store frequency table (simplified)
        for &f in freq_table.iter() {
            encoded.extend_from_slice(&f.to_le_bytes());
        }

        // LZ77-style encoding with entropy coding
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
                        // Encode match: marker + distance + length
                        encoded.push(0xFF); // Match marker
                        encoded.extend_from_slice(&(distance as u32).to_le_bytes());
                        encoded.push(length as u8);
                        pos += length;
                        continue;
                    }
                }

                hash_table.insert(sequence.to_vec(), pos);
            }

            // Encode literal
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

        // Skip frequency table
        let mut pos = 256 * 4;
        let mut decoded = Vec::new();

        while pos < data.len() {
            if data[pos] == 0xFF && pos + 5 < data.len() {
                // Decode match
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
                // Literal byte
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

        let mut stats = self.stats.lock().unwrap();
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

        let mut stats = self.stats.lock().unwrap();
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
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CompressionStats::new();
    }
}

// ============================================================================
// Dictionary-Based Compression
// ============================================================================

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

        // Initialize with single-byte entries
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

        let mut stats = self.stats.lock().unwrap();
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

        let mut stats = self.stats.lock().unwrap();
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
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CompressionStats::new();
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

        // Flush remaining bits
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

        // Store frequency table
        for &freq in &frequencies {
            compressed.extend_from_slice(&freq.to_le_bytes());
        }

        // Encode data
        let encoded = self.encode_with_huffman(input, &code_table);
        compressed.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        compressed.extend_from_slice(&encoded);

        if compressed.len() > output.len() {
            return Err(CompressionError::BufferTooSmall(compressed.len(), output.len()));
        }

        output[..compressed.len()].copy_from_slice(&compressed);

        let mut stats = self.stats.lock().unwrap();
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

        // Read frequency table
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

        let mut stats = self.stats.lock().unwrap();
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
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CompressionStats::new();
    }
}

// ============================================================================
// Adaptive Compression - Selects best algorithm dynamically
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
            // Small data - use LZ4 for speed
            0
        } else if compressibility < 1.5 {
            // Low compressibility - use fast algorithm
            0 // LZ4
        } else if compressibility < 2.5 {
            // Medium compressibility
            1 // Zstd
        } else if input.iter().collect::<std::collections::HashSet<_>>().len() < 64 {
            // Few unique bytes - dictionary works well
            2
        } else {
            // High compressibility - Huffman
            3
        }
    }
}

impl Compressor for AdaptiveCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        if input.is_empty() {
            return Ok(0);
        }

        let compressor_idx = self.select_best_compressor(input);

        // Store algorithm identifier
        output[0] = compressor_idx as u8;

        // Compress with selected algorithm
        let compressed_size = self.compressors[compressor_idx].compress(input, &mut output[1..])?;

        let mut stats = self.stats.lock().unwrap();
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

        let mut stats = self.stats.lock().unwrap();
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
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CompressionStats::new();
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
}
