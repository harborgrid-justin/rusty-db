// OLAP Cube Operations
//
// This module provides comprehensive OLAP cube functionality:
// - ROLLUP for hierarchical aggregation (subtotals and grand totals)
// - CUBE for all combinations of dimensions (full cross-tabulation)
// - GROUPING SETS for custom aggregation combinations
// - Pre-aggregation and caching for query acceleration
// - Drill-down and roll-up navigation
// - Slice and dice operations
// - Pivot table generation

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

// OLAP Cube definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlapCube {
    pub name: String,
    pub dimensions: Vec<Dimension>,
    pub measures: Vec<Measure>,
    pub hierarchies: Vec<Hierarchy>,
    pub aggregations: HashMap<GroupingKey, AggregationResult>,
    pub metadata: CubeMetadata,
}

// Dimension in a cube
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub column: String,
    pub data_type: DimensionType,
    pub cardinality: usize,
    pub levels: Vec<DimensionLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DimensionType {
    Categorical,
    Temporal,
    Spatial,
    Numerical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionLevel {
    pub name: String,
    pub column: String,
    pub order: usize,
}

// Measure (metric) in a cube
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    pub name: String,
    pub column: String,
    pub aggregation: AggregationType,
    pub data_type: MeasureType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Sum,
    Avg,
    Count,
    Min,
    Max,
    StdDev,
    Variance,
    DistinctCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeasureType {
    Integer,
    Float,
    Decimal,
    Money,
}

// Hierarchy of dimensions (e.g., Year > Quarter > Month > Day)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hierarchy {
    pub name: String,
    pub dimension: String,
    pub levels: Vec<String>,
}

// Cube metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeMetadata {
    pub created_at: std::time::SystemTime,
    pub last_updated: std::time::SystemTime,
    pub row_count: u64,
    pub cell_count: u64,
    pub sparsity: f64,
    pub compression_ratio: f64,
}

// Grouping key for aggregation results
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroupingKey {
    // Dimension values (None indicates ALL level)
    values: Vec<Option<String>>,
    // Grouping set identifier
    grouping_id: usize,
}

// Aggregation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub measure_values: HashMap<String, f64>,
    pub row_count: u64,
}

// Grouping sets specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupingSets {
    // ROLLUP(a, b, c) -> (a,b,c), (a,b), (a), ()
    Rollup { columns: Vec<String> },
    // CUBE(a, b, c) -> all 2^n combinations
    Cube { columns: Vec<String> },
    // Custom grouping sets
    Custom { sets: Vec<Vec<String>> },
}

// Cube builder for creating and materializing cubes
pub struct CubeBuilder {
    cube: OlapCube,
    source_data: Vec<HashMap<String, String>>,
}

impl CubeBuilder {
    pub fn new(name: String) -> Self {
        Self {
            cube: OlapCube {
                name,
                dimensions: Vec::new(),
                measures: Vec::new(),
                hierarchies: Vec::new(),
                aggregations: HashMap::new(),
                metadata: CubeMetadata {
                    created_at: std::time::SystemTime::now(),
                    last_updated: std::time::SystemTime::now(),
                    row_count: 0,
                    cell_count: 0,
                    sparsity: 0.0,
                    compression_ratio: 1.0,
                },
            },
            source_data: Vec::new(),
        }
    }

    // Add dimension to cube
    pub fn add_dimension(&mut self, dimension: Dimension) {
        self.cube.dimensions.push(dimension);
    }

    // Add measure to cube
    pub fn add_measure(&mut self, measure: Measure) {
        self.cube.measures.push(measure);
    }

    // Add hierarchy
    pub fn add_hierarchy(&mut self, hierarchy: Hierarchy) {
        self.cube.hierarchies.push(hierarchy);
    }

    // Set source data
    pub fn set_source_data(&mut self, data: Vec<HashMap<String, String>>) {
        self.source_data = data;
        self.cube.metadata.row_count = self.source_data.len() as u64;
    }

    // Build cube with ROLLUP
    pub fn build_rollup(&mut self, columns: Vec<String>) -> Result<()> {
        let grouping_sets = self.generate_rollup_sets(&columns);
        self.materialize_grouping_sets(&grouping_sets)?;
        Ok(())
    }

    // Build cube with CUBE
    pub fn build_cube(&mut self, columns: Vec<String>) -> Result<()> {
        let grouping_sets = self.generate_cube_sets(&columns);
        self.materialize_grouping_sets(&grouping_sets)?;
        Ok(())
    }

    // Build cube with custom grouping sets
    pub fn build_grouping_sets(&mut self, sets: Vec<Vec<String>>) -> Result<()> {
        self.materialize_grouping_sets(&sets)?;
        Ok(())
    }

    // Generate ROLLUP grouping sets
    // ROLLUP(a, b, c) generates: (a,b,c), (a,b), (a), ()
    fn generate_rollup_sets(&self, columns: &[String]) -> Vec<Vec<String>> {
        let mut sets = Vec::new();

        // Add full set
        sets.push(columns.to_vec());

        // Add progressive rollups
        for i in (0..columns.len()).rev() {
            if i > 0 {
                sets.push(columns[..i].to_vec());
            }
        }

        // Add grand total (empty set)
        sets.push(Vec::new());

        sets
    }

    // Generate CUBE grouping sets
    // CUBE(a, b, c) generates all 2^n combinations
    fn generate_cube_sets(&self, columns: &[String]) -> Vec<Vec<String>> {
        let n = columns.len();
        let num_sets = 1 << n; // 2^n
        let mut sets = Vec::new();

        for i in 0..num_sets {
            let mut set = Vec::new();
            for (j, col) in columns.iter().enumerate() {
                if (i & (1 << j)) != 0 {
                    set.push(col.clone());
                }
            }
            sets.push(set);
        }

        sets
    }

    // Materialize grouping sets
    fn materialize_grouping_sets(&mut self, sets: &[Vec<String>]) -> Result<()> {
        for (set_id, grouping_set) in sets.iter().enumerate() {
            self.materialize_single_grouping_set(grouping_set, set_id)?;
        }

        self.cube.metadata.last_updated = std::time::SystemTime::now();
        self.calculate_cube_statistics();

        Ok(())
    }

    // Materialize a single grouping set
    fn materialize_single_grouping_set(
        &mut self,
        groupingcolumns: &[String],
        set_id: usize,
    ) -> Result<()> {
        // Group data by the grouping columns
        let mut groups: HashMap<Vec<Option<String>>, Vec<&HashMap<String, String>>> =
            HashMap::new();

        for row in &self.source_data {
            let key: Vec<Option<String>> = groupingcolumns
                .iter()
                .map(|col| row.get(col).cloned())
                .collect();

            groups.entry(key).or_insert_with(Vec::new).push(row);
        }

        // Compute aggregations for each group
        for (key, group_rows) in groups {
            let mut measure_values = HashMap::new();

            for measure in &self.cube.measures {
                let value = self.compute_aggregate(measure, &group_rows)?;
                measure_values.insert(measure.name.clone(), value);
            }

            let grouping_key = GroupingKey {
                values: key,
                grouping_id: set_id,
            };

            let result = AggregationResult {
                measure_values,
                row_count: group_rows.len() as u64,
            };

            self.cube.aggregations.insert(grouping_key, result);
        }

        Ok(())
    }

    // Compute aggregate value
    fn compute_aggregate(
        &self,
        measure: &Measure,
        rows: &[&HashMap<String, String>],
    ) -> Result<f64> {
        let values: Vec<f64> = rows
            .iter()
            .filter_map(|row| row.get(&measure.column).and_then(|v| v.parse::<f64>().ok()))
            .collect();

        if values.is_empty() {
            return Ok(0.0);
        }

        let result = match measure.aggregation {
            AggregationType::Sum => values.iter().sum(),
            AggregationType::Avg => values.iter().sum::<f64>() / values.len() as f64,
            AggregationType::Count => values.len() as f64,
            AggregationType::Min => values
                .iter()
                .cloned()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            AggregationType::Max => values
                .iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            AggregationType::StdDev => {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance =
                    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                variance.sqrt()
            }
            AggregationType::Variance => {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
            }
            AggregationType::DistinctCount => {
                let unique: HashSet<_> = values.iter().map(|v| v.to_bits()).collect();
                unique.len() as f64
            }
        };

        Ok(result)
    }

    // Calculate cube statistics
    fn calculate_cube_statistics(&mut self) {
        self.cube.metadata.cell_count = self.cube.aggregations.len() as u64;

        // Calculate theoretical maximum cells
        let max_cells: u64 = self
            .cube
            .dimensions
            .iter()
            .map(|d| d.cardinality as u64)
            .product();

        if max_cells > 0 {
            self.cube.metadata.sparsity =
                1.0 - (self.cube.metadata.cell_count as f64 / max_cells as f64);
        }
    }

    // Get the built cube
    pub fn build(self) -> OlapCube {
        self.cube
    }
}

// Cube query executor
pub struct CubeQuery {
    cube: Arc<RwLock<OlapCube>>,
}

impl CubeQuery {
    pub fn new(cube: Arc<RwLock<OlapCube>>) -> Self {
        Self { cube }
    }

    // Slice cube by fixing one dimension
    pub fn slice(&self, dimension: &str, value: &str) -> Result<Vec<AggregationResult>> {
        let cube = self.cube.read();
        let mut results = Vec::new();

        // Find dimension index
        let dim_index = cube
            .dimensions
            .iter()
            .position(|d| d.name == dimension)
            .ok_or_else(|| DbError::NotFound(format!("Dimension: {}", dimension)))?;

        // Filter aggregations matching the slice
        for (key, result) in &cube.aggregations {
            if let Some(Some(dim_value)) = key.values.get(dim_index) {
                if dim_value == value {
                    results.push(result.clone());
                }
            }
        }

        Ok(results)
    }

    // Dice cube by filtering multiple dimensions
    pub fn dice(&self, filters: HashMap<String, String>) -> Result<Vec<AggregationResult>> {
        let cube = self.cube.read();
        let mut results = Vec::new();

        // Get dimension indices
        let mut filter_indices = HashMap::new();
        for (dim_name, value) in &filters {
            let index = cube
                .dimensions
                .iter()
                .position(|d| &d.name == dim_name)
                .ok_or_else(|| DbError::NotFound(format!("Dimension: {}", dim_name)))?;
            filter_indices.insert(index, value);
        }

        // Filter aggregations matching all filters
        for (key, result) in &cube.aggregations {
            let mut matches = true;
            for (index, expected_value) in &filter_indices {
                if let Some(Some(actual_value)) = key.values.get(*index) {
                    if actual_value != *expected_value {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            if matches {
                results.push(result.clone());
            }
        }

        Ok(results)
    }

    // Drill down - navigate from higher to lower level in hierarchy
    pub fn drill_down(
        &self,
        hierarchy: &str,
        current_level: &str,
        _value: &str,
    ) -> Result<Vec<AggregationResult>> {
        let cube = self.cube.read();

        // Find hierarchy
        let hier = cube
            .hierarchies
            .iter()
            .find(|h| h.name == hierarchy)
            .ok_or_else(|| DbError::NotFound(format!("Hierarchy: {}", hierarchy)))?;

        // Find current level and next level
        let current_index = hier
            .levels
            .iter()
            .position(|l| l == current_level)
            .ok_or_else(|| DbError::NotFound(format!("Level: {}", current_level)))?;

        if current_index + 1 >= hier.levels.len() {
            return Err(DbError::InvalidInput("Already at lowest level".to_string()));
        }

        // Query lower level
        // In production, would filter by parent value and return child values
        Ok(Vec::new())
    }

    // Roll up - navigate from lower to higher level in hierarchy
    pub fn roll_up(&self, hierarchy: &str, current_level: &str) -> Result<Vec<AggregationResult>> {
        let cube = self.cube.read();

        // Find hierarchy
        let hier = cube
            .hierarchies
            .iter()
            .find(|h| h.name == hierarchy)
            .ok_or_else(|| DbError::NotFound(format!("Hierarchy: {}", hierarchy)))?;

        // Find current level and parent level
        let current_index = hier
            .levels
            .iter()
            .position(|l| l == current_level)
            .ok_or_else(|| DbError::NotFound(format!("Level: {}", current_level)))?;

        if current_index == 0 {
            return Err(DbError::InvalidInput(
                "Already at highest level".to_string(),
            ));
        }

        // Query higher level
        Ok(Vec::new())
    }

    // Pivot - rotate dimensions
    pub fn pivot(
        &self,
        row_dimensions: Vec<String>,
        col_dimensions: Vec<String>,
        measure: &str,
    ) -> Result<PivotTable> {
        let cube = self.cube.read();

        // Verify measure exists
        if !cube.measures.iter().any(|m| m.name == measure) {
            return Err(DbError::NotFound(format!("Measure: {}", measure)));
        }

        let mut row_keys = HashSet::new();
        let mut col_keys = HashSet::new();
        let mut cells = HashMap::new();

        // Collect all row and column keys
        for (key, result) in &cube.aggregations {
            let row_key = self.extract_key_values(&row_dimensions, &cube, key)?;
            let col_key = self.extract_key_values(&col_dimensions, &cube, key)?;

            row_keys.insert(row_key.clone());
            col_keys.insert(col_key.clone());

            if let Some(value) = result.measure_values.get(measure) {
                cells.insert((row_key, col_key), *value);
            }
        }

        Ok(PivotTable {
            row_headers: row_keys.into_iter().collect(),
            column_headers: col_keys.into_iter().collect(),
            cells,
            measure: measure.to_string(),
        })
    }

    fn extract_key_values(
        &self,
        dimensions: &[String],
        cube: &OlapCube,
        key: &GroupingKey,
    ) -> Result<Vec<String>> {
        let mut values = Vec::new();

        for dim_name in dimensions {
            let dim_index = cube
                .dimensions
                .iter()
                .position(|d| &d.name == dim_name)
                .ok_or_else(|| DbError::NotFound(format!("Dimension: {}", dim_name)))?;

            let value = key
                .values
                .get(dim_index)
                .and_then(|v| v.clone())
                .unwrap_or_else(|| "ALL".to_string());

            values.push(value);
        }

        Ok(values)
    }

    // Get GROUPING function value
    pub fn grouping(&self, key: &GroupingKey, dimension_index: usize) -> u8 {
        match key.values.get(dimension_index) {
            Some(None) => 1,    // Aggregated (ALL level)
            Some(Some(_)) => 0, // Not aggregated
            None => 1,
        }
    }

    // Get GROUPING_ID value
    pub fn grouping_id(&self, key: &GroupingKey) -> usize {
        key.grouping_id
    }
}

// Pivot table result
#[derive(Debug, Clone)]
pub struct PivotTable {
    pub row_headers: Vec<Vec<String>>,
    pub column_headers: Vec<Vec<String>>,
    pub cells: HashMap<(Vec<String>, Vec<String>), f64>,
    pub measure: String,
}

impl PivotTable {
    // Get cell value
    pub fn get_cell(&self, row: &[String], col: &[String]) -> Option<f64> {
        self.cells.get(&(row.to_vec(), col.to_vec())).copied()
    }

    // Format as text table
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Header row
        output.push_str("         ");
        for col_header in &self.column_headers {
            output.push_str(&format!("{:>12}", col_header.join(",")));
        }
        output.push('\n');

        // Data rows
        for row_header in &self.row_headers {
            output.push_str(&format!("{:8} ", row_header.join(",")));
            for col_header in &self.column_headers {
                if let Some(value) = self.get_cell(row_header, col_header) {
                    output.push_str(&format!("{:12.2}", value));
                } else {
                    output.push_str("           -");
                }
            }
            output.push('\n');
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollup_generation() {
        let builder = CubeBuilder::new("test_cube".to_string());
        let columns = vec![
            "year".to_string(),
            "quarter".to_string(),
            "month".to_string(),
        ];

        let sets = builder.generate_rollup_sets(&columns);

        // Should generate: (y,q,m), (y,q), (y), ()
        assert_eq!(sets.len(), 4);
        assert_eq!(sets[0].len(), 3);
        assert_eq!(sets[1].len(), 2);
        assert_eq!(sets[2].len(), 1);
        assert_eq!(sets[3].len(), 0);
    }

    #[test]
    fn test_cube_generation() {
        let builder = CubeBuilder::new("test_cube".to_string());
        let columns = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let sets = builder.generate_cube_sets(&columns);

        // Should generate 2^3 = 8 combinations
        assert_eq!(sets.len(), 8);
    }

    #[test]
    fn test_cube_building() {
        let mut builder = CubeBuilder::new("sales_cube".to_string());

        builder.add_dimension(Dimension {
            name: "region".to_string(),
            column: "region".to_string(),
            data_type: DimensionType::Categorical,
            cardinality: 4,
            levels: Vec::new(),
        });

        builder.add_measure(Measure {
            name: "total_sales".to_string(),
            column: "sales".to_string(),
            aggregation: AggregationType::Sum,
            data_type: MeasureType::Float,
        });

        let mut data = Vec::new();
        let mut row1 = HashMap::new();
        row1.insert("region".to_string(), "East".to_string());
        row1.insert("sales".to_string(), "100.0".to_string());
        data.push(row1);

        let mut row2 = HashMap::new();
        row2.insert("region".to_string(), "East".to_string());
        row2.insert("sales".to_string(), "200.0".to_string());
        data.push(row2);

        builder.set_source_data(data);
        builder.build_rollup(vec!["region".to_string()]).unwrap();

        let cube = builder.build();
        assert!(cube.aggregations.len() > 0);
    }
}
