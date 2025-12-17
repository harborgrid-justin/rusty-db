// Bitmap Index Implementation
//
// This module provides bitmap indexing optimized for:
// - Low-cardinality columns (e.g., gender, status, category)
// - Fast AND/OR/NOT operations
// - Compressed storage using run-length encoding
// - Range-encoded bitmaps for numeric data
// - Efficient bitmap scans

use crate::Result;
use crate::error::DbError;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

// Maximum runs in compressed bitmap to prevent fragmentation
// If exceeded, bitmap will be rejected to prevent pathological fragmentation
// For truly fragmented workloads, consider using uncompressed bitmaps
const MAX_RUNS: usize = 10000;

// Bitmap Index
//
// Maintains a bitmap for each distinct value in the indexed column
pub struct BitmapIndex<T: Eq + std::hash::Hash + Clone> {
    // Map from value to compressed bitmap
    bitmaps: Arc<RwLock<HashMap<T, CompressedBitmap>>>,
    // Total number of rows indexed
    num_rows: Arc<RwLock<usize>>,
    // Bitmap compression enabled
    compression_enabled: bool,
}

impl<T: Eq + std::hash::Hash + Clone> Clone for BitmapIndex<T> {
    fn clone(&self) -> Self {
        Self {
            bitmaps: Arc::clone(&self.bitmaps),
            num_rows: Arc::clone(&self.num_rows),
            compression_enabled: self.compression_enabled,
        }
    }
}

impl<T: Eq + std::hash::Hash + Clone> BitmapIndex<T> {
    // Create a new bitmap index
    pub fn new() -> Self {
        Self {
            bitmaps: Arc::new(RwLock::new(HashMap::new())),
            num_rows: Arc::new(RwLock::new(0)),
            compression_enabled: true,
        }
    }

    // Create without compression
    pub fn new_uncompressed() -> Self {
        Self {
            bitmaps: Arc::new(RwLock::new(HashMap::new())),
            num_rows: Arc::new(RwLock::new(0)),
            compression_enabled: false,
        }
    }

    // Insert a value at a given row position
    pub fn insert(&self, value: T, row_id: usize) -> Result<()> {
        let mut bitmaps = self.bitmaps.write();
        let mut num_rows = self.num_rows.write();

        // Ensure we have enough capacity
        if row_id >= *num_rows {
            *num_rows = row_id + 1;
        }

        // Get or create bitmap for this value
        let bitmap = bitmaps
            .entry(value)
            .or_insert_with(|| CompressedBitmap::new(*num_rows));

        // Set the bit for this row
        bitmap.set(row_id, true)?;

        Ok(())
    }

    // Get all row IDs for a specific value
    pub fn get(&self, value: &T) -> Result<Vec<usize>> {
        let bitmaps = self.bitmaps.read();

        match bitmaps.get(value) {
            Some(bitmap) => Ok(bitmap.get_set_bits()),
            None => Ok(Vec::new()),
        }
    }

    // Perform AND operation between two values
    pub fn and(&self, value1: &T, value2: &T) -> Result<Vec<usize>> {
        let bitmaps = self.bitmaps.read();

        let bitmap1 = bitmaps.get(value1);
        let bitmap2 = bitmaps.get(value2);

        match (bitmap1, bitmap2) {
            (Some(b1), Some(b2)) => {
                let result = b1.and(b2);
                Ok(result.get_set_bits())
            }
            _ => Ok(Vec::new()),
        }
    }

    // Perform OR operation between two values
    pub fn or(&self, value1: &T, value2: &T) -> Result<Vec<usize>> {
        let bitmaps = self.bitmaps.read();

        let bitmap1 = bitmaps.get(value1);
        let bitmap2 = bitmaps.get(value2);

        match (bitmap1, bitmap2) {
            (Some(b1), Some(b2)) => {
                let result = b1.or(b2);
                Ok(result.get_set_bits())
            }
            (Some(b1), None) => Ok(b1.get_set_bits()),
            (None, Some(b2)) => Ok(b2.get_set_bits()),
            _ => Ok(Vec::new()),
        }
    }

    // Perform NOT operation (invert bitmap)
    pub fn not(&self, value: &T) -> Result<Vec<usize>> {
        let bitmaps = self.bitmaps.read();
        let num_rows = *self.num_rows.read();

        match bitmaps.get(value) {
            Some(bitmap) => {
                let result = bitmap.not(num_rows);
                Ok(result.get_set_bits())
            }
            None => {
                // All rows match
                Ok((0..num_rows).collect())
            }
        }
    }

    // Get statistics
    pub fn stats(&self) -> BitmapIndexStats {
        let bitmaps = self.bitmaps.read();
        let num_rows = *self.num_rows.read();

        let num_values = bitmaps.len();
        let total_bits = num_rows * num_values;
        let compressed_size: usize = bitmaps.values().map(|b| b.compressed_size()).sum();

        BitmapIndexStats {
            num_values,
            num_rows,
            total_bits,
            compressed_size,
            compression_ratio: total_bits as f64 / compressed_size.max(1) as f64,
            total_bytes: (),
            total_entries: (),
        }
    }
}

// Compressed Bitmap using run-length encoding
#[derive(Debug, Clone)]
pub struct CompressedBitmap {
    // Run-length encoded runs of 0s and 1s
    runs: Vec<Run>,
}

impl CompressedBitmap {
    // Create a new compressed bitmap
    pub fn new(size: usize) -> Self {
        Self {
            runs: vec![Run {
                value: false,
                length: size,
            }],
        }
    }

    // Set a bit at a position
    pub fn set(&mut self, position: usize, value: bool) -> Result<()> {
        let mut current_pos = 0;
        let mut run_idx = 0;

        // Find the run containing this position
        while run_idx < self.runs.len() {
            let run = &self.runs[run_idx];

            if current_pos + run.length > position {
                // Found the run
                let offset = position - current_pos;

                if run.value == value {
                    // Already has the correct value
                    return Ok(());
                }

                // Check if split would exceed MAX_RUNS
                if self.runs.len() >= MAX_RUNS {
                    return Err(DbError::ResourceExhausted(
                        format!(
                            "Bitmap fragmentation limit reached ({} runs). Consider using uncompressed bitmaps.",
                            MAX_RUNS
                        )
                    ));
                }

                // Need to split the run
                self.split_run(run_idx, offset, value);
                return Ok(());
            }

            current_pos += run.length;
            run_idx += 1;
        }

        Ok(())
    }

    // Split a run to set a different value
    fn split_run(&mut self, run_idx: usize, offset: usize, value: bool) {
        let run = self.runs[run_idx].clone();

        if offset == 0 {
            // Split at beginning
            if run.length == 1 {
                self.runs[run_idx].value = value;
            } else {
                self.runs[run_idx].length -= 1;
                self.runs.insert(run_idx, Run { value, length: 1 });
            }
        } else if offset == run.length - 1 {
            // Split at end
            self.runs[run_idx].length -= 1;
            self.runs.insert(run_idx + 1, Run { value, length: 1 });
        } else {
            // Split in middle
            let remaining = run.length - offset - 1;
            self.runs[run_idx].length = offset;

            self.runs.insert(run_idx + 1, Run { value, length: 1 });

            self.runs.insert(
                run_idx + 2,
                Run {
                    value: run.value,
                    length: remaining,
                },
            );
        }

        // Merge adjacent runs with same value
        self.merge_adjacent_runs();
    }

    // Merge adjacent runs with the same value
    fn merge_adjacent_runs(&mut self) {
        let mut i = 0;
        while i + 1 < self.runs.len() {
            if self.runs[i].value == self.runs[i + 1].value {
                self.runs[i].length += self.runs[i + 1].length;
                self.runs.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    // Get all positions where bit is set to true
    pub fn get_set_bits(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut position = 0;

        for run in &self.runs {
            if run.value {
                for i in 0..run.length {
                    result.push(position + i);
                }
            }
            position += run.length;
        }

        result
    }

    // AND operation
    pub fn and(&self, other: &CompressedBitmap) -> CompressedBitmap {
        let mut result = CompressedBitmap { runs: Vec::new() };

        let mut self_iter = RunIterator::new(&self.runs);
        let mut other_iter = RunIterator::new(&other.runs);

        while self_iter.has_next() && other_iter.has_next() {
            let self_value = self_iter.current_value();
            let other_value = other_iter.current_value();
            let and_value = self_value && other_value;

            let min_remaining = self_iter.remaining().min(other_iter.remaining());

            // Add run to result
            if let Some(last) = result.runs.last_mut() {
                if last.value == and_value {
                    last.length += min_remaining;
                } else {
                    result.runs.push(Run {
                        value: and_value,
                        length: min_remaining,
                    });
                }
            } else {
                result.runs.push(Run {
                    value: and_value,
                    length: min_remaining,
                });
            }

            self_iter.advance(min_remaining);
            other_iter.advance(min_remaining);
        }

        result
    }

    // OR operation
    pub fn or(&self, other: &CompressedBitmap) -> CompressedBitmap {
        let mut result = CompressedBitmap { runs: Vec::new() };

        let mut self_iter = RunIterator::new(&self.runs);
        let mut other_iter = RunIterator::new(&other.runs);

        while self_iter.has_next() && other_iter.has_next() {
            let self_value = self_iter.current_value();
            let other_value = other_iter.current_value();
            let or_value = self_value || other_value;

            let min_remaining = self_iter.remaining().min(other_iter.remaining());

            // Add run to result
            if let Some(last) = result.runs.last_mut() {
                if last.value == or_value {
                    last.length += min_remaining;
                } else {
                    result.runs.push(Run {
                        value: or_value,
                        length: min_remaining,
                    });
                }
            } else {
                result.runs.push(Run {
                    value: or_value,
                    length: min_remaining,
                });
            }

            self_iter.advance(min_remaining);
            other_iter.advance(min_remaining);
        }

        result
    }

    // NOT operation
    pub fn not(&self, _size: usize) -> CompressedBitmap {
        let mut result = CompressedBitmap { runs: Vec::new() };

        for run in &self.runs {
            result.runs.push(Run {
                value: !run.value,
                length: run.length,
            });
        }

        result
    }

    // Get compressed size in bytes (approximate)
    pub fn compressed_size(&self) -> usize {
        // Each run takes about 16 bytes (bool + usize)
        self.runs.len() * 16
    }
}

// A run of consecutive bits with the same value
#[derive(Debug, Clone)]
struct Run {
    value: bool,
    length: usize,
}

// Iterator over runs
struct RunIterator<'a> {
    runs: &'a [Run],
    run_idx: usize,
    offset_in_run: usize,
}

impl<'a> RunIterator<'a> {
    fn new(runs: &'a [Run]) -> Self {
        Self {
            runs,
            run_idx: 0,
            offset_in_run: 0,
        }
    }

    fn has_next(&self) -> bool {
        self.run_idx < self.runs.len()
    }

    fn current_value(&self) -> bool {
        if self.run_idx < self.runs.len() {
            self.runs[self.run_idx].value
        } else {
            false
        }
    }

    fn remaining(&self) -> usize {
        if self.run_idx < self.runs.len() {
            self.runs[self.run_idx].length - self.offset_in_run
        } else {
            0
        }
    }

    fn advance(&mut self, count: usize) {
        let mut remaining_count = count;

        while remaining_count > 0 && self.has_next() {
            let available = self.remaining();

            if remaining_count >= available {
                remaining_count -= available;
                self.run_idx += 1;
                self.offset_in_run = 0;
            } else {
                self.offset_in_run += remaining_count;
                remaining_count = 0;
            }
        }
    }
}

// Range-encoded bitmap for numeric ranges
pub struct RangeEncodedBitmap {
    // Bitmaps for different ranges
    range_bitmaps: HashMap<RangeBucket, CompressedBitmap>,
    // Min and max values
    min_value: i64,
    #[allow(dead_code)]
    max_value: i64,
    // Bucket size
    bucket_size: i64,
}

impl RangeEncodedBitmap {
    // Create a new range-encoded bitmap
    pub fn new(min_value: i64, max_value: i64, num_buckets: usize) -> Self {
        let bucket_size = ((max_value - min_value) / num_buckets as i64).max(1);

        Self {
            range_bitmaps: HashMap::new(),
            min_value,
            max_value,
            bucket_size,
        }
    }

    // Insert a value
    pub fn insert(&mut self, value: i64, row_id: usize, total_rows: usize) -> Result<()> {
        let bucket = self.get_bucket(value);

        let bitmap = self
            .range_bitmaps
            .entry(bucket)
            .or_insert_with(|| CompressedBitmap::new(total_rows));

        bitmap.set(row_id, true)?;
        Ok(())
    }

    // Query a range
    pub fn range_query(&self, start: i64, end: i64) -> Vec<usize> {
        let start_bucket = self.get_bucket(start);
        let end_bucket = self.get_bucket(end);

        let mut result = CompressedBitmap::new(0);
        let mut initialized = false;

        for bucket_val in start_bucket.0..=end_bucket.0 {
            let bucket = RangeBucket(bucket_val);
            if let Some(bitmap) = self.range_bitmaps.get(&bucket) {
                if !initialized {
                    result = bitmap.clone();
                    initialized = true;
                } else {
                    result = result.or(bitmap);
                }
            }
        }

        result.get_set_bits()
    }

    fn get_bucket(&self, value: i64) -> RangeBucket {
        let offset = value - self.min_value;
        let bucket = offset / self.bucket_size;
        RangeBucket(bucket)
    }
}

// Range bucket identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RangeBucket(i64);

// Bitmap index statistics
#[derive(Debug, Clone)]
pub struct BitmapIndexStats {
    pub num_values: usize,
    pub num_rows: usize,
    pub total_bits: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub total_bytes: (),
    pub total_entries: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_insert_get() {
        let index: BitmapIndex<String> = BitmapIndex::new();

        index.insert("active".to_string(), 0).unwrap();
        index.insert("active".to_string(), 2).unwrap();
        index.insert("inactive".to_string(), 1).unwrap();

        let active_rows = index.get(&"active".to_string()).unwrap();
        assert_eq!(active_rows, vec![0, 2]);

        let inactive_rows = index.get(&"inactive".to_string()).unwrap();
        assert_eq!(inactive_rows, vec![1]);
    }

    #[test]
    fn test_bitmap_and() {
        let index: BitmapIndex<String> = BitmapIndex::new();

        index.insert("tag1".to_string(), 0).unwrap();
        index.insert("tag1".to_string(), 1).unwrap();
        index.insert("tag2".to_string(), 1).unwrap();
        index.insert("tag2".to_string(), 2).unwrap();

        let result = index.and(&"tag1".to_string(), &"tag2".to_string()).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_bitmap_or() {
        let index: BitmapIndex<String> = BitmapIndex::new();

        index.insert("tag1".to_string(), 0).unwrap();
        index.insert("tag1".to_string(), 1).unwrap();
        index.insert("tag2".to_string(), 2).unwrap();

        let result = index.or(&"tag1".to_string(), &"tag2".to_string()).unwrap();
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
    }

    #[test]
    fn test_compressed_bitmap() {
        let mut bitmap = CompressedBitmap::new(100);

        bitmap.set(10, true).unwrap();
        bitmap.set(20, true).unwrap();
        bitmap.set(30, true).unwrap();

        let set_bits = bitmap.get_set_bits();
        assert_eq!(set_bits, vec![10, 20, 30]);

        // Check compression
        assert!(bitmap.runs.len() < 10); // Should be compressed
    }

    #[test]
    fn test_bitmap_not() {
        let index: BitmapIndex<String> = BitmapIndex::new();

        index.insert("active".to_string(), 0).unwrap();
        index.insert("active".to_string(), 2).unwrap();
        // Row 1 is implicitly inactive

        let inactive_rows = index.not(&"active".to_string()).unwrap();
        assert!(inactive_rows.contains(&1));
    }

    #[test]
    fn test_range_encoded_bitmap() {
        let mut range_bitmap = RangeEncodedBitmap::new(0, 100, 10);

        range_bitmap.insert(5, 0, 10);
        range_bitmap.insert(15, 1, 10);
        range_bitmap.insert(25, 2, 10);
        range_bitmap.insert(35, 3, 10);

        let results = range_bitmap.range_query(10, 30);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }
}
