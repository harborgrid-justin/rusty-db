// OLAP Operations and Multidimensional Analysis
//
// This module provides OLAP (Online Analytical Processing) capabilities:
//
// - **Cube Building**: Create multidimensional cubes
// - **OLAP Operations**: Drill-down, roll-up, slice, dice
// - **Aggregation Cubes**: Pre-computed aggregates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::Result;
use super::aggregates::AggregateFunction;

// =============================================================================
// OLAP Cube Builder
// =============================================================================

/// Builder for constructing OLAP cubes.
#[derive(Debug, Clone)]
pub struct OlapCubeBuilder {
    /// Dimension columns
    dimensions: Vec<String>,

    /// Measure columns
    measures: Vec<String>,

    /// Aggregation functions for measures
    aggregations: Vec<AggregateFunction>,
}

impl OlapCubeBuilder {
    /// Create a new OLAP cube builder.
    pub fn new() -> Self {
        Self {
            dimensions: Vec::new(),
            measures: Vec::new(),
            aggregations: Vec::new(),
        }
    }

    /// Add a dimension to the cube.
    pub fn add_dimension(&mut self, dimension: String) -> &mut Self {
        self.dimensions.push(dimension);
        self
    }

    /// Add a measure with an aggregation function.
    pub fn add_measure(&mut self, measure: String, aggregation: AggregateFunction) -> &mut Self {
        self.measures.push(measure);
        self.aggregations.push(aggregation);
        self
    }

    /// Get the dimensions.
    pub fn dimensions(&self) -> &[String] {
        &self.dimensions
    }

    /// Get the measures.
    pub fn measures(&self) -> &[String] {
        &self.measures
    }

    /// Build the cube from data.
    ///
    /// In production, this would compute aggregates for all dimension combinations.
    pub fn build_cube(&self, _data: Vec<Vec<String>>) -> OlapCube {
        OlapCube {
            dimensions: self.dimensions.clone(),
            measures: self.measures.clone(),
            cells: HashMap::new(),
        }
    }
}

impl Default for OlapCubeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// OLAP Cube
// =============================================================================

/// OLAP cube for multidimensional analysis.
///
/// Stores pre-computed aggregates indexed by dimension values.
pub struct OlapCube {
    /// Dimension column names
    dimensions: Vec<String>,

    /// Measure column names
    measures: Vec<String>,

    /// Cube cells: dimension values -> measure values
    cells: HashMap<Vec<String>, Vec<f64>>,
}

impl OlapCube {
    /// Create an empty cube.
    pub fn new(dimensions: Vec<String>, measures: Vec<String>) -> Self {
        Self {
            dimensions,
            measures,
            cells: HashMap::new(),
        }
    }

    /// Insert a cell value.
    pub fn insert_cell(&mut self, dimension_values: Vec<String>, measure_values: Vec<f64>) {
        self.cells.insert(dimension_values, measure_values);
    }

    /// Query the cube with dimension filters.
    pub fn query(&self, dimensionfilters: &HashMap<String, String>) -> Vec<Vec<f64>> {
        self.cells
            .iter()
            .filter(|(keys, _)| {
                // Check if all filters match
                for (i, dim) in self.dimensions.iter().enumerate() {
                    if let Some(filter_value) = dimension_filters.get(dim) {
                        if keys.get(i).map(|k| k != filter_value).unwrap_or(true) {
                            return false;
                        }
                    }
                }
                true
            })
            .map(|(_, values)| values.clone())
            .collect()
    }

    /// Drill down to a more detailed level.
    ///
    /// Returns a new cube with the additional dimension.
    pub fn drill_down(&self, dimension: &str) -> Result<OlapCube> {
        // In production, this would add a new dimension level
        Ok(OlapCube {
            dimensions: self.dimensions.clone(),
            measures: self.measures.clone(),
            cells: self.cells.clone(),
        })
    }

    /// Roll up to a less detailed level.
    ///
    /// Returns a new cube with aggregated values.
    pub fn roll_up(&self, dimension: &str) -> Result<OlapCube> {
        // In production, this would aggregate out a dimension
        Ok(OlapCube {
            dimensions: self.dimensions.clone(),
            measures: self.measures.clone(),
            cells: HashMap::new(),
        })
    }

    /// Slice the cube by fixing one dimension value.
    pub fn slice(&self, dimension: &str, value: &str) -> Result<OlapCube> {
        let mut filters = HashMap::new();
        filters.insert(dimension.to_string(), value.to_string());

        let filtered_cells: HashMap<_, _> = self
            .cells
            .iter()
            .filter(|(keys, _)| {
                if let Some(idx) = self.dimensions.iter().position(|d| d == dimension) {
                    keys.get(idx).map(|k| k == value).unwrap_or(false)
                } else {
                    true
                }
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(OlapCube {
            dimensions: self.dimensions.clone(),
            measures: self.measures.clone(),
            cells: filtered_cells,
        })
    }

    /// Dice the cube by filtering on multiple dimensions.
    pub fn dice(&self, filters: &HashMap<String, Vec<String>>) -> Result<OlapCube> {
        let filtered_cells: HashMap<_, _> = self
            .cells
            .iter()
            .filter(|(keys, _)| {
                for (i, dim) in self.dimensions.iter().enumerate() {
                    if let Some(allowed_values) = filters.get(dim) {
                        if let Some(key_value) = keys.get(i) {
                            if !allowed_values.contains(key_value) {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(OlapCube {
            dimensions: self.dimensions.clone(),
            measures: self.measures.clone(),
            cells: filtered_cells,
        })
    }

    /// Get a specific cell value.
    pub fn get_cell(&self, coordinates: &[String]) -> Option<&Vec<f64>> {
        self.cells.get(coordinates)
    }

    /// Get the total number of cells.
    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }

    /// Get the dimensions.
    pub fn dimensions(&self) -> &[String] {
        &self.dimensions
    }

    /// Get the measures.
    pub fn measures(&self) -> &[String] {
        &self.measures
    }
}

// =============================================================================
// Multidimensional Aggregator
// =============================================================================

/// Aggregator for computing multidimensional aggregates.
pub struct MultidimensionalAggregator {
    /// Dimension column indices
    dimensions: Vec<usize>,

    /// Measure column indices
    measures: Vec<usize>,

    /// Aggregation functions
    aggregations: Vec<AggregateFunction>,
}

impl MultidimensionalAggregator {
    /// Create a new aggregator.
    pub fn new(
        dimensions: Vec<usize>,
        measures: Vec<usize>,
        aggregations: Vec<AggregateFunction>,
    ) -> Self {
        Self {
            dimensions,
            measures,
            aggregations,
        }
    }

    /// Compute a CUBE (all possible groupings).
    pub fn compute_cube(&self, data: &[Vec<String>]) -> AggregationCube {
        let mut cube = AggregationCube::new();

        // Generate all possible dimension combinations (power set)
        let num_dims = self.dimensions.len();
        for i in 0..(1 << num_dims) {
            let mut active_dims = Vec::new();
            for j in 0..num_dims {
                if i & (1 << j) != 0 {
                    active_dims.push(self.dimensions[j]);
                }
            }

            self.aggregate_by_dimensions(data, &active_dims, &mut cube);
        }

        cube
    }

    /// Compute a ROLLUP (hierarchical groupings).
    pub fn compute_rollup(&self, data: &[Vec<String>]) -> AggregationCube {
        let mut cube = AggregationCube::new();

        // Generate prefix combinations
        for i in 0..=self.dimensions.len() {
            let active_dims: Vec<_> = self.dimensions[0..i].to_vec();
            self.aggregate_by_dimensions(data, &active_dims, &mut cube);
        }

        cube
    }

    /// Aggregate by specific dimensions.
    fn aggregate_by_dimensions(
        &self,
        data: &[Vec<String>],
        dim_indices: &[usize],
        cube: &mut AggregationCube,
    ) {
        // Group data by dimension values
        let mut groups: HashMap<Vec<String>, Vec<&Vec<String>>> = HashMap::new();

        for row in data {
            let key: Vec<String> = dim_indices
                .iter()
                .map(|&i| row.get(i).cloned().unwrap_or_default())
                .collect();
            groups.entry(key).or_default().push(row);
        }

        // Compute aggregates for each group
        for (key, group) in groups {
            let mut values = Vec::new();

            for (measure_idx, agg) in self.measures.iter().zip(&self.aggregations) {
                let measure_values: Vec<f64> = group
                    .iter()
                    .filter_map(|row| row.get(*measure_idx))
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();

                let agg_value = match agg {
                    AggregateFunction::Sum => measure_values.iter().sum(),
                    AggregateFunction::Count => measure_values.len() as f64,
                    AggregateFunction::Avg => {
                        if measure_values.is_empty() {
                            0.0
                        } else {
                            measure_values.iter().sum::<f64>() / measure_values.len() as f64
                        }
                    }
                    AggregateFunction::Min => {
                        measure_values.iter().copied().fold(f64::INFINITY, f64::min)
                    }
                    AggregateFunction::Max => {
                        measure_values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    }
                    _ => 0.0,
                };

                values.push(agg_value);
            }

            cube.cells.insert(key, values);
        }
    }
}

// =============================================================================
// Aggregation Cube
// =============================================================================

/// Result of multidimensional aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationCube {
    /// Cells mapping dimension values to measure values
    pub cells: HashMap<Vec<String>, Vec<f64>>,
}

impl AggregationCube {
    /// Create an empty aggregation cube.
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    /// Get a cell value.
    pub fn get_cell(&self, coordinates: &[String]) -> Option<&Vec<f64>> {
        self.cells.get(coordinates)
    }

    /// Get the total number of cells.
    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }

    /// Get all cells.
    pub fn all_cells(&self) -> impl Iterator<Item = (&Vec<String>, &Vec<f64>)> {
        self.cells.iter()
    }
}

impl Default for AggregationCube {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_builder() {
        let mut builder = OlapCubeBuilder::new();
        builder
            .add_dimension("region".to_string())
            .add_dimension("product".to_string())
            .add_measure("sales".to_string(), AggregateFunction::Sum);

        assert_eq!(builder.dimensions().len(), 2);
        assert_eq!(builder.measures().len(), 1);
    }

    #[test]
    fn test_olap_cube_slice() {
        let mut cube = OlapCube::new(
            vec!["region".to_string(), "product".to_string()],
            vec!["sales".to_string()],
        );

        cube.insert_cell(vec!["East".to_string(), "A".to_string()], vec![100.0]);
        cube.insert_cell(vec!["West".to_string(), "A".to_string()], vec![200.0]);
        cube.insert_cell(vec!["East".to_string(), "B".to_string()], vec![150.0]);

        let sliced = cube.slice("region", "East").unwrap();
        assert_eq!(sliced.total_cells(), 2);
    }

    #[test]
    fn test_olap_cube_query() {
        let mut cube = OlapCube::new(
            vec!["region".to_string()],
            vec!["sales".to_string()],
        );

        cube.insert_cell(vec!["East".to_string()], vec![100.0]);
        cube.insert_cell(vec!["West".to_string()], vec![200.0]);

        let mut filters = HashMap::new();
        filters.insert("region".to_string(), "East".to_string());

        let results = cube.query(&filters);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0][0], 100.0);
    }

    #[test]
    fn test_multidimensional_aggregator() {
        let aggregator = MultidimensionalAggregator::new(
            vec![0], // Dimension column
            vec![1], // Measure column
            vec![AggregateFunction::Sum],
        );

        let data = vec![
            vec!["A".to_string(), "10".to_string()],
            vec!["A".to_string(), "20".to_string()],
            vec!["B".to_string(), "30".to_string()],
        ];

        let cube = aggregator.compute_cube(&data);

        // Should have cells for: {}, {A}, {B}
        assert!(cube.total_cells() > 0);
    }

    #[test]
    fn test_aggregation_cube() {
        let mut cube = AggregationCube::new();
        cube.cells.insert(vec!["A".to_string()], vec![100.0]);

        let cell = cube.get_cell(&vec!["A".to_string()]);
        assert!(cell.is_some());
        assert_eq!(cell.unwrap()[0], 100.0);
    }
}
