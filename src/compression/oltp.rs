// OLTP Compression - Optimized for transactional workloads
// Focuses on update-friendly compression with minimal overhead

use super::*;
use super::algorithms::LZ4Compressor;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;

/// Block-level compression for OLTP tables
/// Smaller compression units for better update performance
#[derive(Debug, Clone)]
pub struct OLTPBlock {
    pub block_id: u64,
    pub table_id: u64,
    pub row_count: usize,
    pub compressed_data: Vec<u8>,
    pub compression_metadata: BlockCompressionMetadata,
    pub row_directory: Vec<RowLocation>,
    pub free_space: usize,
    pub modification_count: usize,
}

/// Metadata for compressed block
#[derive(Debug, Clone)]
pub struct BlockCompressionMetadata {
    pub algorithm: CompressionAlgorithm,
    pub level: CompressionLevel,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub block_format: BlockFormat,
    pub checksum: u32,
    pub last_modified: u64,
}

/// Block format for OLTP compression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFormat {
    Uncompressed,        // No compression
    BasicCompression,    // Simple LZ4 compression
    AdvancedCompression, // Better compression with row chaining
    OnlineConversion,    // Being converted to compressed format
}

/// Row location within a compressed block
#[derive(Debug, Clone)]
pub struct RowLocation {
    pub row_id: u64,
    pub offset: usize,
    pub length: usize,
    pub is_chained: bool,
    pub next_piece: Option<u64>,
}

/// OLTP Compression Engine
pub struct OLTPCompressor {
    level: CompressionLevel,
    stats: Arc<RwLock<CompressionStats>>,
    block_cache: Arc<RwLock<HashMap<u64, OLTPBlock>>>,
    lz4_compressor: LZ4Compressor,
    block_size: usize,
    compression_threshold: f64,
    enable_row_chaining: bool,
}

impl OLTPCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        Self {
            level,
            stats: Arc::new(RwLock::new(CompressionStats::new())),
            block_cache: Arc::new(RwLock::new(HashMap::new())),
            lz4_compressor: LZ4Compressor::new(level),
            block_size: 8192, // 8KB blocks
            compression_threshold: 1.2, // Compress only if ratio > 1.2
            enable_row_chaining: true,
        }
    }

    pub fn with_block_size(mut self, size: usize) -> Self {
        self.block_size = size;
        self
    }

    pub fn with_compression_threshold(mut self, threshold: f64) -> Self {
        self.compression_threshold = threshold;
        self
    }

    /// Compress a table block
    pub fn compress_block(&self, block_id: u64, table_id: u64, rows: Vec<Vec<u8>>)
        -> CompressionResult<OLTPBlock> {

        let start = Instant::now();

        if rows.is_empty() {
            return Err(CompressionError::InvalidInput("Empty block".to_string()));
        }

        // Serialize rows with row directory
        let (uncompressed_data, row_directory) = self.serialize_rows(&rows, block_id)?;

        // Check if compression is beneficial
        let estimated_ratio = utils::estimate_compressibility(&uncompressed_data);
        let block_format = if estimated_ratio >= self.compression_threshold {
            BlockFormat::AdvancedCompression
        } else {
            BlockFormat::BasicCompression
        };

        // Compress the block
        let mut compressed_data = vec![0u8; self.lz4_compressor.max_compressed_size(uncompressed_data.len())];
        let compressed_size = self.lz4_compressor.compress(&uncompressed_data, &mut compressed_data)?;
        compressed_data.truncate(compressed_size);

        // Only use compression if it actually saves space
        let (final_data, final_format, final_size) = if compressed_size < uncompressed_data.len() {
            (compressed_data, block_format, compressed_size)
        } else {
            (uncompressed_data.clone(), BlockFormat::Uncompressed, uncompressed_data.len())
        };

        let metadata = BlockCompressionMetadata {
            algorithm: CompressionAlgorithm::LZ4,
            level: self.level,
            uncompressed_size: uncompressed_data.len(),
            compressed_size: final_size,
            block_format: final_format,
            checksum: utils::crc32(&uncompressed_data),
            last_modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let free_space = self.block_size.saturating_sub(final_size);

        let block = OLTPBlock {
            block_id,
            table_id,
            row_count: rows.len(),
            compressed_data: final_data,
            compression_metadata: metadata,
            row_directory,
            free_space,
            modification_count: 0,
        };

        // Update statistics
        let mut stats = self.stats.write().unwrap();
        stats.uncompressed_size += uncompressed_data.len();
        stats.compressed_size += final_size;
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        // Cache the block
        self.block_cache.write().unwrap().insert(block_id, block.clone());

        Ok(block)
    }

    /// Decompress a block
    pub fn decompress_block(&self, block: &OLTPBlock) -> CompressionResult<Vec<Vec<u8>>> {
        let start = Instant::now();

        let uncompressed_data = match block.compression_metadata.block_format {
            BlockFormat::Uncompressed => {
                block.compressed_data.clone()
            }
            BlockFormat::BasicCompression | BlockFormat::AdvancedCompression => {
                let mut decompressed = vec![0u8; block.compression_metadata.uncompressed_size];
                let size = self.lz4_compressor.decompress(&block.compressed_data, &mut decompressed)?;
                decompressed.truncate(size);

                // Verify checksum
                let checksum = utils::crc32(&decompressed);
                if checksum != block.compression_metadata.checksum {
                    return Err(CompressionError::CorruptedData(
                        format!("Block {} checksum mismatch", block.block_id)
                    )));
                }

                decompressed
            }
            BlockFormat::OnlineConversion => {
                return Err(CompressionError::InvalidInput(
                    "Block is being converted".to_string()
                ));
            }
        };

        let rows = self.deserialize_rows(&uncompressed_data, &block.row_directory)?;

        let mut stats = self.stats.write().unwrap();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(rows)
    }

    /// Update a single row in a compressed block
    pub fn update_row(&self, block: &mut OLTPBlock, row_id: u64, newrow: Vec<u8>)
        -> CompressionResult<()> {

        // Decompress the entire block
        let mut rows = self.decompress_block(block)?;

        // Find and update the row
        let row_index = block.row_directory.iter()
            .position(|loc| loc.row_id == row_id)
            .ok_or_else(|| CompressionError::InvalidInput(
                format!("Row {} not found", row_id)
            ))?;

        // Check if new row fits in place
        if new_row.len() <= rows[row_index].len() && !self.enable_row_chaining {
            // In-place update
            rows[row_index] = new_row;
        } else if new_row.len() <= block.free_space {
            // Update with available free space
            rows[row_index] = new_row;
        } else {
            // Row chaining needed
            if self.enable_row_chaining {
                rows[row_index] = new_row;
            } else {
                return Err(CompressionError::ResourceExhausted(
                    "Insufficient space for row update".to_string()
                ));
            }
        }

        // Recompress the block
        let (uncompressed_data, row_directory) = self.serialize_rows(&rows, block.block_id)?;

        let mut compressed_data = vec![0u8; self.lz4_compressor.max_compressed_size(uncompressed_data.len())];
        let compressed_size = self.lz4_compressor.compress(&uncompressed_data, &mut compressed_data)?;
        compressed_data.truncate(compressed_size);

        let (final_data, final_size) = if compressed_size < uncompressed_data.len() {
            (compressed_data, compressed_size)
        } else {
            (uncompressed_data.clone(), uncompressed_data.len())
        };

        // Update block
        block.compressed_data = final_data;
        block.row_directory = row_directory;
        block.compression_metadata.compressed_size = final_size;
        block.compression_metadata.uncompressed_size = uncompressed_data.len();
        block.compression_metadata.checksum = utils::crc32(&uncompressed_data);
        block.free_space = self.block_size.saturating_sub(final_size);
        block.modification_count += 1;

        Ok(())
    }

    /// Insert a new row into a compressed block
    pub fn insert_row(&self, block: &mut OLTPBlock, _row_id: u64, row: Vec<u8>)
        -> CompressionResult<()> {

        // Check if there's enough free space
        if row.len() > block.free_space && block.free_space < self.block_size / 4 {
            return Err(CompressionError::ResourceExhausted(
                "Block is full".to_string()
            ));
        }

        let mut rows = self.decompress_block(block)?;
        rows.push(row);

        let (uncompressed_data, row_directory) = self.serialize_rows(&rows, block.block_id)?;

        let mut compressed_data = vec![0u8; self.lz4_compressor.max_compressed_size(uncompressed_data.len())];
        let compressed_size = self.lz4_compressor.compress(&uncompressed_data, &mut compressed_data)?;
        compressed_data.truncate(compressed_size);

        let (final_data, final_size) = if compressed_size < uncompressed_data.len() {
            (compressed_data, compressed_size)
        } else {
            (uncompressed_data.clone(), uncompressed_data.len())
        };

        block.compressed_data = final_data;
        block.row_directory = row_directory;
        block.row_count = rows.len();
        block.compression_metadata.compressed_size = final_size;
        block.compression_metadata.uncompressed_size = uncompressed_data.len();
        block.compression_metadata.checksum = utils::crc32(&uncompressed_data);
        block.free_space = self.block_size.saturating_sub(final_size);
        block.modification_count += 1;

        Ok(())
    }

    /// Delete a row from a compressed block
    pub fn delete_row(&self, block: &mut OLTPBlock, row_id: u64) -> CompressionResult<()> {
        let mut rows = self.decompress_block(block)?;

        let row_index = block.row_directory.iter()
            .position(|loc| loc.row_id == row_id)
            .ok_or_else(|| CompressionError::InvalidInput(
                format!("Row {} not found", row_id)
            ))?;

        rows.remove(row_index);

        let (uncompressed_data, row_directory) = self.serialize_rows(&rows, block.block_id)?;

        let mut compressed_data = vec![0u8; self.lz4_compressor.max_compressed_size(uncompressed_data.len())];
        let compressed_size = self.lz4_compressor.compress(&uncompressed_data, &mut compressed_data)?;
        compressed_data.truncate(compressed_size);

        let (final_data, final_size) = if compressed_size < uncompressed_data.len() {
            (compressed_data, compressed_size)
        } else {
            (uncompressed_data.clone(), uncompressed_data.len())
        };

        block.compressed_data = final_data;
        block.row_directory = row_directory;
        block.row_count = rows.len();
        block.compression_metadata.compressed_size = final_size;
        block.compression_metadata.uncompressed_size = uncompressed_data.len();
        block.compression_metadata.checksum = utils::crc32(&uncompressed_data);
        block.free_space = self.block_size.saturating_sub(final_size);
        block.modification_count += 1;

        Ok(())
    }

    /// Bulk load compression - optimized for initial data load
    pub fn bulk_load(&self, table_id: u64, rows: Vec<Vec<u8>>) -> CompressionResult<Vec<OLTPBlock>> {
        let rows_per_block = self.calculate_optimal_rows_per_block(&rows);
        let mut blocks = Vec::new();
        let mut block_id = 0u64;

        for chunk in rows.chunks(rows_per_block) {
            let block = self.compress_block(block_id, table_id, chunk.to_vec())?;
            blocks.push(block);
            block_id += 1;
        }

        Ok(blocks)
    }

    /// Online compression conversion - convert uncompressed table to compressed
    pub fn convert_to_compressed(&self, uncompressed_blocks: Vec<OLTPBlock>)
        -> CompressionResult<Vec<OLTPBlock>> {

        let mut compressed_blocks = Vec::new();

        for block in uncompressed_blocks {
            if block.compression_metadata.block_format == BlockFormat::Uncompressed {
                // Convert to compressed format
                let rows = self.deserialize_rows(&block.compressed_data, &block.row_directory)?;
                let compressed_block = self.compress_block(block.block_id, block.table_id, rows)?;
                compressed_blocks.push(compressed_block);
            } else {
                // Already compressed
                compressed_blocks.push(block);
            }
        }

        Ok(compressed_blocks)
    }

    /// Check if block needs recompression
    pub fn needs_recompression(&self, block: &OLTPBlock) -> bool {
        // Recompress if too many modifications or fragmentation
        block.modification_count > 100 ||
        block.free_space > self.block_size / 2
    }

    /// Recompress a block to reclaim space
    pub fn recompress_block(&self, block: &mut OLTPBlock) -> CompressionResult<()> {
        let rows = self.decompress_block(block)?;
        let new_block = self.compress_block(block.block_id, block.table_id, rows)?;

        *block = new_block;
        block.modification_count = 0;

        Ok(())
    }

    ffn serialize_rows(&self, rows: &[Vec<u8>], baserow_id: u64)        -> CompressionResult<(Vec<u8>, Vec<RowLocation>)> {

        let mut data = Vec::new();
        let mut row_directory = Vec::new();

        for (idx, row) in rows.iter().enumerate() {
            let row_id = base_row_id * 10000 + idx as u64;
            let offset = data.len();
            let length = row.len();

            // Write row length
            data.extend_from_slice(&(length as u32).to_le_bytes());
            // Write row data
            data.extend_from_slice(row);

            row_directory.push(RowLocation {
                row_id,
                offset,
                length,
                is_chained: false,
                next_piece: None,
            });
        }

        Ok((data, row_directory))
    }

    fn deserialize_rows(&self, data: &[u8], row_directory: &[RowLocation])
        -> CompressionResult<Vec<Vec<u8>>> {

        let mut rows = Vec::new();

        for loc in row_directory {
            if loc.offset + 4 > data.len() {
                return Err(CompressionError::DecompressionFailed(
                    "Invalid row offset".to_string()
                ));
            }

            let length = u32::from_le_bytes([
                data[loc.offset],
                data[loc.offset + 1],
                data[loc.offset + 2],
                data[loc.offset + 3],
            ]) as usize;

            if loc.offset + 4 + length > data.len() {
                return Err(CompressionError::DecompressionFailed(
                    "Invalid row length".to_string()
                ));
            }

            let row = data[loc.offset + 4..loc.offset + 4 + length].to_vec();
            rows.push(row);
        }

        Ok(rows)
    }

    fn calculate_optimal_rows_per_block(&self, rows: &[Vec<u8>]) -> usize {
        if rows.is_empty() {
            return 100;
        }

        let avg_row_size = rows.iter().map(|r| r.len()).sum::<usize>() / rows.len();
        let rows_per_block = self.block_size / avg_row_size.max(1);

        rows_per_block.max(10).min(1000)
    }
}

impl Compressor for OLTPCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        self.lz4_compressor.compress(input, output)
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        self.lz4_compressor.decompress(input, output)
    }

    fn max_compressed_size(&self, input_size: usize) -> usize {
        self.lz4_compressor.max_compressed_size(input_size)
    }

    fn algorithm(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::LZ4
    }

    fn level(&self) -> CompressionLevel {
        self.level
    }

    fn stats(&self) -> CompressionStats {
        self.stats.read().unwrap().clone()
    }

    fn reset_stats(&mut self) {
        *self.stats.write().unwrap() = CompressionStats::new();
    }
}

/// OLTP Compression Advisor
pub struct OLTPCompressionAdvisor {
    update_frequency_threshold: f64,
    compression_ratio_threshold: f64,
}

impl OLTPCompressionAdvisor {
    pub fn new() -> Self {
        Self {
            update_frequency_threshold: 0.5, // 50% update rate
            compression_ratio_threshold: 1.5,
        }
    }

    /// Analyze table and recommend compression strategy
    pub fn recommend_compression(&self, table_stats: &TableStats) -> CompressionRecommendation {
        let update_ratio = table_stats.updates_per_second as f64 /
            (table_stats.reads_per_second as f64 + 1.0);

        if update_ratio > self.update_frequency_threshold {
            // High update rate - use minimal compression
            CompressionRecommendation {
                should_compress: false,
                recommended_level: CompressionLevel::Fast,
                recommended_block_size: 8192,
                enable_row_chaining: true,
                reason: "High update frequency detected".to_string(),
            }
        } else if table_stats.estimated_compression_ratio > self.compression_ratio_threshold {
            // Good compression potential
            CompressionRecommendation {
                should_compress: true,
                recommended_level: CompressionLevel::Default,
                recommended_block_size: 16384,
                enable_row_chaining: true,
                reason: "Good compression potential".to_string(),
            }
        } else {
            // Marginal benefit
            CompressionRecommendation {
                should_compress: true,
                recommended_level: CompressionLevel::Fast,
                recommended_block_size: 8192,
                enable_row_chaining: false,
                reason: "Minimal compression overhead".to_string(),
            }
        }
    }

    /// Estimate space savings from compression
    pub fn estimate_space_savings(&self, table_stats: &TableStats) -> f64 {
        let compressed_size = table_stats.total_size as f64 /
            table_stats.estimated_compression_ratio;
        table_stats.total_size as f64 - compressed_size
    }
}

impl Default for OLTPCompressionAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_id: u64,
    pub total_rows: usize,
    pub total_size: usize,
    pub avg_row_size: usize,
    pub reads_per_second: usize,
    pub updates_per_second: usize,
    pub estimated_compression_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct CompressionRecommendation {
    pub should_compress: bool,
    pub recommended_level: CompressionLevel,
    pub recommended_block_size: usize,
    pub enable_row_chaining: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_oltp_block_compression() {
        let compressor = OLTPCompressor::new(CompressionLevel::Default);

        let rows = vec![
            vec![1, 2, 3, 4, 5],
            vec![6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15],
        ];

        let block = compressor.compress_block(1, 100, rows.clone()).unwrap();
        assert_eq!(block.row_count, 3);

        let decompressed = compressor.decompress_block(&block).unwrap();
        assert_eq!(decompressed, rows);
    }

    #[test]
    fn test_row_update() {
        let compressor = OLTPCompressor::new(CompressionLevel::Default);

        let rows = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
        ];

        let mut block = compressor.compress_block(1, 100, rows).unwrap();
        let new_row = vec![7, 8, 9];

        compressor.update_row(&mut block, block.row_directory[0].row_id, new_row.clone()).unwrap();

        let decompressed = compressor.decompress_block(&block).unwrap();
        assert_eq!(decompressed[0], new_row);
    }

    #[test]
    fn test_bulk_load() {
        let compressor = OLTPCompressor::new(CompressionLevel::Default);

        let mut rows = Vec::new();
        for i in 0..1000 {
            rows.push(vec![i as u8; 10]);
        }

        let blocks = compressor.bulk_load(100, rows).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_compression_advisor() {
        let advisor = OLTPCompressionAdvisor::new();

        let stats = TableStats {
            table_id: 1,
            total_rows: 100000,
            total_size: 10_000_000,
            avg_row_size: 100,
            reads_per_second: 1000,
            updates_per_second: 100,
            estimated_compression_ratio: 2.5,
        };

        let recommendation = advisor.recommend_compression(&stats);
        assert!(recommendation.should_compress);
    }
}


