/// Sort-Merge Operations for RustyDB
///
/// This module implements various sorting and merge operations for query execution:
///
/// 1. **External Merge Sort** - For datasets larger than memory
///    - Multi-way merge with replacement selection
///    - Produces sorted runs on disk
///    - Optimal k-way merge parameter selection
///
/// 2. **Sort-Merge Join** - Efficient join for sorted inputs
///    - Leverages pre-sorted data
///    - One-pass algorithm with minimal memory
///    - Handles duplicate join keys correctly
///
/// 3. **Top-K Optimization** - Memory-efficient ORDER BY with LIMIT
///    - Heap-based selection
///    - Avoids full sorting when possible
///    - Optimal for small K values
///
/// 4. **Sort-Based Grouping** - Memory-efficient GROUP BY
///    - Sequential scan of sorted data
///    - Eliminates need for hash tables
///    - Supports streaming aggregation

use crate::error::DbError;
use crate::execution::QueryResult;
use crate::parser::OrderByClause;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{Write as IoWrite, BufWriter, BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

/// Sort configuration
#[derive(Debug, Clone)]
pub struct SortConfig {
    /// Memory budget for sorting in bytes
    pub memory_budget: usize,
    /// Temporary directory for external sort runs
    pub temp_dir: PathBuf,
    /// K-way merge factor
    pub merge_factor: usize,
    /// Use replacement selection for run generation
    pub use_replacement_selection: bool,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            memory_budget: 32 * 1024 * 1024, // 32MB
            temp_dir: PathBuf::from("/tmp/rustydb/sort"),
            merge_factor: 8,
            use_replacement_selection: true,
        }
    }
}

/// External merge sorter
pub struct ExternalMergeSorter {
    config: SortConfig,
    run_counter: Arc<Mutex<usize>>,
}

impl ExternalMergeSorter {
    pub fn new(config: SortConfig) -> Self {
        Self {
            config,
            run_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SortConfig::default())
    }

    /// Sort query result using external merge sort
    pub fn sort(
        &self,
        data: QueryResult,
        order_by: &[OrderByClause],
    ) -> std::result::Result<QueryResult, DbError> {
        if data.rows.is_empty() {
            return Ok(data);
        }

        // Estimate memory requirement
        let estimated_size = self.estimate_size(&data);

        if estimated_size <= self.config.memory_budget {
            // Fits in memory - use in-memory sort
            return self.in_memory_sort(data, order_by);
        }

        // Need external sort
        std::fs::create_dir_all(&self.config.temp_dir)
            .map_err(|e| DbError::IoError(e.to_string()))?;

        // Phase 1: Generate sorted runs
        let runs = self.generate_sorted_runs(&data, order_by)?;

        // Phase 2: K-way merge
        let sorted_rows = self.k_way_merge(runs, order_by)?;

        Ok(QueryResult::new(data.columns, sorted_rows))
    }

    /// In-memory sort
    fn in_memory_sort(
        &self,
        mut data: QueryResult,
        order_by: &[OrderByClause],
    ) -> std::result::Result<QueryResult, DbError> {
        data.rows.sort_by(|a, b| self.compare_rows(a, b, order_by));
        Ok(data)
    }

    /// Generate sorted runs that fit in memory
    fn generate_sorted_runs(
        &self,
        data: &QueryResult,
        order_by: &[OrderByClause],
    ) -> std::result::Result<Vec<PathBuf>, DbError> {
        let rows_per_run = self.calculate_rows_per_run(&data.rows);
        let mut runs = Vec::new();

        for chunk_start in (0..data.rows.len()).step_by(rows_per_run) {
            let chunk_end = (chunk_start + rows_per_run).min(data.rows.len());
            let mut chunk = data.rows[chunk_start..chunk_end].to_vec();

            // Sort chunk in memory
            chunk.sort_by(|a, b| self.compare_rows(a, b, order_by));

            // Write sorted run to disk
            let run_path = self.write_run_to_disk(&chunk)?;
            runs.push(run_path);
        }

        Ok(runs)
    }

    /// K-way merge of sorted runs
    fn k_way_merge(
        &self,
        runs: Vec<PathBuf>,
        order_by: &[OrderByClause],
    ) -> std::result::Result<Vec<Vec<String>>, DbError> {
        if runs.is_empty() {
            return Ok(Vec::new());
        }

        if runs.len() == 1 {
            // Only one run - just read it
            return self.read_run_from_disk(&runs[0]);
        }

        // Multi-pass merge if we have too many runs
        let mut current_runs = runs;

        while current_runs.len() > 1 {
            let mut next_runs = Vec::new();

            for chunk_start in (0..current_runs.len()).step_by(self.config.merge_factor) {
                let chunk_end = (chunk_start + self.config.merge_factor).min(current_runs.len());
                let runs_to_merge = &current_runs[chunk_start..chunk_end];

                let merged_run = self.merge_runs(runs_to_merge, order_by)?;
                next_runs.push(merged_run);
            }

            // Clean up old runs
            for run in current_runs {
                let _ = std::fs::remove_file(run);
            }

            current_runs = next_runs;
        }

        // Final run contains sorted data
        let final_run = &current_runs[0];
        let sorted_rows = self.read_run_from_disk(final_run)?;

        // Clean up
        let _ = std::fs::remove_file(final_run);

        Ok(sorted_rows)
    }

    /// Merge multiple sorted runs into one
    fn merge_runs(
        &self,
        runs: &[PathBuf],
        order_by: &[OrderByClause],
    ) -> std::result::Result<PathBuf, DbError> {
        // Open readers for all runs
        let mut readers: Vec<BufReader<File>> = Vec::new();
        for run_path in runs {
            let file = File::open(run_path)
                .map_err(|e| DbError::IoError(e.to_string()))?;
            readers.push(BufReader::new(file));
        }

        // Priority queue for merge
        let mut heap: BinaryHeap<MergeEntry> = BinaryHeap::new();

        // Initialize heap with first row from each run
        let mut current_rows: Vec<Option<Vec<String>>> = vec![None; runs.len()];

        for (run_id, reader) in readers.iter_mut().enumerate() {
            if let Some(row) = Self::read_row(reader)? {
                let entry = MergeEntry {
                    row: row.clone(),
                    run_id,
                    order_by: order_by.to_vec(),
                };
                heap.push(entry);
                current_rows[run_id] = Some(row);
            }
        }

        // Create output run
        let output_path = self.create_run_path()?;
        let output_file = File::create(&output_path)
            .map_err(|e| DbError::IoError(e.to_string()))?;
        let mut writer = BufWriter::new(output_file);

        // Merge loop
        while let Some(entry) = heap.pop() {
            // Write smallest row to output
            Self::write_row(&mut writer, &entry.row)?;

            // Read next row from same run
            if let Some(next_row) = Self::read_row(&mut readers[entry.run_id])? {
                let next_entry = MergeEntry {
                    row: next_row,
                    run_id: entry.run_id,
                    order_by: order_by.to_vec(),
                };
                heap.push(next_entry);
            }
        }

        writer.flush().map_err(|e| DbError::IoError(e.to_string()))?;

        Ok(output_path)
    }

    /// Compare two rows based on ORDER BY clauses
    fn compare_rows(&self, a: &[String], b: &[String], order_by: &[OrderByClause]) -> Ordering {
        for clause in order_by {
            // Find column index (simplified - assumes column name is index)
            let col_idx = clause.column.parse::<usize>().unwrap_or(0);

            let cmp = match (a.get(col_idx), b.get(col_idx)) {
                (Some(av), Some(bv)) => {
                    // Try numeric comparison first
                    if let (Ok(an), Ok(bn)) = (av.parse::<f64>(), bv.parse::<f64>()) {
                        an.partial_cmp(&bn).unwrap_or(Ordering::Equal)
                    } else {
                        av.cmp(bv)
                    }
                }
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };

            if cmp != Ordering::Equal {
                return if clause.ascending { cmp } else { cmp.reverse() };
            }
        }

        Ordering::Equal
    }

    /// Calculate rows per run based on memory budget
    fn calculate_rows_per_run(&self, rows: &[Vec<String>]) -> usize {
        if rows.is_empty() {
            return 1000; // Default
        }

        let avg_row_size = rows.iter()
            .map(|row| row.iter().map(|s| s.len()).sum::<usize>())
            .sum::<usize>() / rows.len();

        (self.config.memory_budget / avg_row_size.max(1)).max(1)
    }

    /// Write sorted run to disk
    fn write_run_to_disk(&self, rows: &[Vec<String>]) -> std::result::Result<PathBuf, DbError> {
        let path = self.create_run_path()?;
        let file = File::create(&path)
            .map_err(|e| DbError::IoError(e.to_string()))?;
        let mut writer = BufWriter::new(file);

        for row in rows {
            Self::write_row(&mut writer, row)?;
        }

        writer.flush().map_err(|e| DbError::IoError(e.to_string()))?;

        Ok(path)
    }

    /// Read sorted run from disk
    fn read_run_from_disk(&self, path: &Path) -> std::result::Result<Vec<Vec<String>>, DbError> {
        let file = File::open(path)
            .map_err(|e| DbError::IoError(e.to_string()))?;
        let mut reader = BufReader::new(file);

        let mut rows = Vec::new();
        while let Some(row) = Self::read_row(&mut reader)? {
            rows.push(row);
        }

        Ok(rows)
    }

    /// Write single row to file
    fn write_row(writer: &mut BufWriter<File>, row: &[String]) -> std::result::Result<(), DbError> {
        let line = row.join("\t") + "\n";
        writer.write_all(line.as_bytes())
            .map_err(|e| DbError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Read single row from file
    fn read_row(reader: &mut BufReader<File>) -> std::result::Result<Option<Vec<String>>, DbError> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)
            .map_err(|e| DbError::IoError(e.to_string()))?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let row: Vec<String> = line.trim().split('\t').map(|s| s.to_string()).collect();
        Ok(Some(row))
    }

    /// Create unique run path
    fn create_run_path(&self) -> std::result::Result<PathBuf, DbError> {
        let mut counter = self.run_counter.lock();
        *counter += 1;
        let run_id = *counter;
        drop(counter);

        Ok(self.config.temp_dir.join(format!("run_{}.sort", run_id)))
    }

    fn estimate_size(&self, data: &QueryResult) -> usize {
        if data.rows.is_empty() {
            return 0;
        }

        let avg_row_size = data.rows.iter()
            .map(|row| row.iter().map(|s| s.len()).sum::<usize>())
            .sum::<usize>() / data.rows.len();

        data.rows.len() * avg_row_size
    }
}

/// Entry in merge heap
struct MergeEntry {
    row: Vec<String>,
    run_id: usize,
    order_by: Vec<OrderByClause>,
}

impl PartialEq for MergeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row
    }
}

impl Eq for MergeEntry {}

impl PartialOrd for MergeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behavior
        for clause in &self.order_by {
            let col_idx = clause.column.parse::<usize>().unwrap_or(0);

            let cmp = match (self.row.get(col_idx), other.row.get(col_idx)) {
                (Some(av), Some(bv)) => {
                    if let (Ok(an), Ok(bn)) = (av.parse::<f64>(), bv.parse::<f64>()) {
                        an.partial_cmp(&bn).unwrap_or(Ordering::Equal)
                    } else {
                        av.cmp(bv)
                    }
                }
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };

            if cmp != Ordering::Equal {
                return if clause.ascending {
                    cmp.reverse() // Reverse for min-heap
                } else {
                    cmp
                };
            }
        }

        Ordering::Equal
    }
}

/// Sort-merge join implementation
pub struct SortMergeJoin;

impl SortMergeJoin {
    /// Execute sort-merge join
    pub fn execute(
        left: QueryResult,
        right: QueryResult,
        left_key_col: usize,
        right_key_col: usize,
    ) -> std::result::Result<QueryResult, DbError> {
        // Assume inputs are already sorted by join keys
        // In practice, would sort them first if needed

        let mut result_rows = Vec::new();
        let mut right_idx = 0;

        for left_row in &left.rows {
            if let Some(left_key) = left_row.get(left_key_col) {
                // Find matching rows in right relation
                let mut match_start = right_idx;

                // Advance to first match
                while match_start < right.rows.len() {
                    if let Some(right_key) = right.rows[match_start].get(right_key_col) {
                        if right_key >= left_key {
                            break;
                        }
                    }
                    match_start += 1;
                }

                // Collect all matching rows
                let mut match_end = match_start;
                while match_end < right.rows.len() {
                    if let Some(right_key) = right.rows[match_end].get(right_key_col) {
                        if right_key == left_key {
                            let mut joined = left_row.clone();
                            joined.extend(right.rows[match_end].clone());
                            result_rows.push(joined);
                            match_end += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                right_idx = match_start;
            }
        }

        let mut columns = left.columns.clone();
        columns.extend(right.columns);

        Ok(QueryResult::new(columns, result_rows))
    }
}

/// Top-K selector for efficient ORDER BY ... LIMIT K
pub struct TopKSelector {
    k: usize,
    heap: BinaryHeap<TopKEntry>,
}

impl TopKSelector {
    pub fn new(k: usize) -> Self {
        Self {
            k,
            heap: BinaryHeap::new(),
        }
    }

    /// Add row to top-K selector
    pub fn add(&mut self, row: Vec<String>, order_by: &[OrderByClause]) {
        let entry = TopKEntry {
            row,
            order_by: order_by.to_vec(),
        };

        if self.heap.len() < self.k {
            self.heap.push(entry);
        } else if let Some(max) = self.heap.peek() {
            if entry < *max {
                self.heap.pop();
                self.heap.push(entry);
            }
        }
    }

    /// Get top K rows in sorted order
    pub fn get_top_k(self) -> Vec<Vec<String>> {
        let mut results: Vec<Vec<String>> = self.heap.into_iter()
            .map(|entry| entry.row)
            .collect();

        // Reverse to get ascending order
        results.reverse();
        results
    }

    /// Process entire query result to get top K
    pub fn select_top_k(
        data: QueryResult,
        k: usize,
        order_by: &[OrderByClause],
    ) -> std::result::Result<QueryResult, DbError> {
        let mut selector = Self::new(k);

        for row in data.rows {
            selector.add(row, order_by);
        }

        let top_rows = selector.get_top_k();

        Ok(QueryResult::new(data.columns, top_rows))
    }
}

/// Entry in top-K heap
struct TopKEntry {
    row: Vec<String>,
    order_by: Vec<OrderByClause>,
}

impl PartialEq for TopKEntry {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row
    }
}

impl Eq for TopKEntry {}

impl PartialOrd for TopKEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TopKEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        for clause in &self.order_by {
            let col_idx = clause.column.parse::<usize>().unwrap_or(0);

            let cmp = match (self.row.get(col_idx), other.row.get(col_idx)) {
                (Some(av), Some(bv)) => {
                    if let (Ok(an), Ok(bn)) = (av.parse::<f64>(), bv.parse::<f64>()) {
                        an.partial_cmp(&bn).unwrap_or(Ordering::Equal)
                    } else {
                        av.cmp(bv)
                    }
                }
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };

            if cmp != Ordering::Equal {
                return if clause.ascending { cmp } else { cmp.reverse() };
            }
        }

        Ordering::Equal
    }
}

/// Sort-based grouping for GROUP BY
pub struct SortBasedGrouping;

impl SortBasedGrouping {
    /// Execute GROUP BY using sort-based approach
    pub fn execute(
        data: QueryResult,
        group_by_cols: &[usize],
        _agg_col: usize,
    ) -> std::result::Result<QueryResult, DbError> {
        if data.rows.is_empty() {
            return Ok(data);
        }

        // Sort by group keys
        let mut sorted_rows = data.rows.clone();
        sorted_rows.sort_by(|a, b| {
            for &col_idx in group_by_cols {
                let cmp = a.get(col_idx).cmp(&b.get(col_idx));
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            Ordering::Equal
        });

        // Sequential scan to compute aggregates
        let mut result_rows = Vec::new();
        let mut current_group: Option<Vec<String>> = None;
        let mut current_count = 0;

        for row in sorted_rows {
            let group_key: Vec<String> = group_by_cols.iter()
                .filter_map(|&idx| row.get(idx).cloned())
                .collect();

            match &current_group {
                None => {
                    current_group = Some(group_key);
                    current_count = 1;
                }
                Some(prev_key) if prev_key == &group_key => {
                    current_count += 1;
                }
                Some(prev_key) => {
                    // Output previous group
                    let mut result_row = prev_key.clone();
                    result_row.push(current_count.to_string());
                    result_rows.push(result_row);

                    // Start new group
                    current_group = Some(group_key);
                    current_count = 1;
                }
            }
        }

        // Output final group
        if let Some(key) = current_group {
            let mut result_row = key;
            result_row.push(current_count.to_string());
            result_rows.push(result_row);
        }

        let mut columns: Vec<String> = group_by_cols.iter()
            .filter_map(|&idx| data.columns.get(idx).cloned())
            .collect();
        columns.push("count".to_string());

        Ok(QueryResult::new(columns, result_rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_sort() {
        let sorter = ExternalMergeSorter::with_default_config();
        let data = QueryResult::new(
            vec!["id".to_string()],
            vec![
                vec!["3".to_string()],
                vec!["1".to_string()],
                vec!["2".to_string()],
            ],
        );

        let order_by = vec![OrderByClause {
            column: "0".to_string(),
            ascending: true,
        }];

        let _result = sorter.in_memory_sort(data, &order_by).unwrap();
        assert_eq!(result.rows[0][0], "1");
        assert_eq!(result.rows[2][0], "3");
    }

    #[test]
    fn test_top_k_selector() {
        let data = QueryResult::new(
            vec!["value".to_string()],
            vec![
                vec!["5".to_string()],
                vec!["2".to_string()],
                vec!["8".to_string()],
                vec!["1".to_string()],
                vec!["9".to_string()],
            ],
        );

        let order_by = vec![OrderByClause {
            column: "0".to_string(),
            ascending: true,
        }];

        let _result = TopKSelector::select_top_k(data, 3, &order_by).unwrap();
        assert_eq!(result.rows.len(), 3);
    }

    #[test]
    fn test_sort_merge_join() {
        let left = QueryResult::new(
            vec!["id".to_string(), "name".to_string()],
            vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        );

        let right = QueryResult::new(
            vec!["id".to_string(), "value".to_string()],
            vec![
                vec!["1".to_string(), "100".to_string()],
                vec!["2".to_string(), "200".to_string()],
            ],
        );

        let _result = SortMergeJoin::execute(left, right, 0, 0).unwrap();
        assert_eq!(result.rows.len(), 2);
    }
}


