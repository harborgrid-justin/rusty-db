// Approximate Query Processing (AQP) Engine
//
// This module provides probabilistic data structures and sampling techniques
// for fast approximate query answering with statistical guarantees:
// - HyperLogLog for cardinality estimation (distinct counts)
// - Count-Min Sketch for frequency estimation
// - Reservoir sampling for percentiles and quantiles
// - Confidence intervals and error bounds
// - Stratified sampling for better accuracy
// - Sample synopses for aggregation queries

use crate::error::{Result, DbError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// HyperLogLog for distinct count estimation
//
// HyperLogLog provides cardinality estimates with typical accuracy of ~2%
// using only a few KB of memory, regardless of the actual cardinality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperLogLog {
    // Number of registers (must be power of 2)
    num_registers: usize,
    // Precision parameter (4-16 typically)
    precision: u8,
    // Register array storing maximum leading zeros
    registers: Vec<u8>,
    // Alpha constant for bias correction
    alpha: f64,
}

impl HyperLogLog {
    // Create new HyperLogLog with specified precision
    // Precision of p gives 2^p registers and ~0.65/sqrt(2^p) standard error
    pub fn new(precision: u8) -> Result<Self> {
        if precision < 4 || precision > 16 {
            return Err(DbError::InvalidInput(
                "HyperLogLog precision must be between 4 and 16".to_string()
            ));
        }

        let num_registers = 1 << precision; // 2^precision
        let alpha = Self::calculate_alpha(num_registers);

        Ok(Self {
            num_registers,
            precision,
            registers: vec![0; num_registers],
            alpha,
        })
    }

    // Add an element to the HyperLogLog
    pub fn add<T: Hash>(&mut self, element: &T) {
        let mut hasher = DefaultHasher::new();
        element.hash(&mut hasher);
        let hash = hasher.finish();

        // Extract register index from first p bits
        let register_index = (hash & ((1 << self.precision) - 1)) as usize;

        // Count leading zeros in remaining bits plus 1
        let remaining_bits = hash >> self.precision;
        let leading_zeros = remaining_bits.leading_zeros() as u8 + 1;

        // Update register with maximum leading zeros seen
        if leading_zeros > self.registers[register_index] {
            self.registers[register_index] = leading_zeros;
        }
    }

    // Estimate cardinality
    pub fn cardinality(&self) -> u64 {
        // Raw estimate using harmonic mean
        let raw_estimate = self.alpha
            * (self.num_registers as f64).powi(2)
            / self.registers.iter()
                .map(|&r| 2.0_f64.powi(-(r as i32)))
                .sum::<f64>();

        // Apply bias correction for different ranges
        if raw_estimate <= 2.5 * self.num_registers as f64 {
            // Small range correction
            let zeros = self.registers.iter().filter(|&&r| r == 0).count();
            if zeros > 0 {
                return (self.num_registers as f64
                    * (self.num_registers as f64 / zeros as f64).ln()) as u64;
            }
        } else if raw_estimate <= (1u64 << 32) as f64 / 30.0 {
            // Intermediate range - no correction
            return raw_estimate as u64;
        } else {
            // Large range correction
            return (-( (1i64 << 32) as f64) * (1.0 - raw_estimate / (1u64 << 32) as f64).ln()) as u64;
        }

        raw_estimate as u64
    }

    // Get standard error for this HyperLogLog
    pub fn standard_error(&self) -> f64 {
        1.04 / (self.num_registers as f64).sqrt()
    }

    // Merge another HyperLogLog into this one
    pub fn merge(&mut self, other: &HyperLogLog) -> Result<()> {
        if self.precision != other.precision {
            return Err(DbError::InvalidInput(
                "Cannot merge HyperLogLogs with different precisions".to_string()
            ));
        }

        for i in 0..self.num_registers {
            self.registers[i] = self.registers[i].max(other.registers[i]);
        }

        Ok(())
    }

    fn calculate_alpha(num_registers: usize) -> f64 {
        match num_registers {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / num_registers as f64),
        }
    }
}

// Count-Min Sketch for frequency estimation
//
// Count-Min Sketch provides frequency estimates with probabilistic guarantees
// on error bounds. It never underestimates, but may overestimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountMinSketch {
    // Width of each row (controls accuracy)
    width: usize,
    // Depth (number of hash functions, controls probability)
    depth: usize,
    // Counter matrix
    counters: Vec<Vec<u64>>,
    // Hash seeds for each row
    seeds: Vec<u64>,
}

impl CountMinSketch {
    // Create new Count-Min Sketch
    //
    // - epsilon: error bound (e.g., 0.01 for 1% error)
    // - delta: failure probability (e.g., 0.01 for 99% confidence)
    //
    // Width = ceil(e/epsilon), Depth = ceil(ln(1/delta))
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(DbError::InvalidInput(
                "Epsilon must be between 0 and 1".to_string()
            ));
        }
        if delta <= 0.0 || delta >= 1.0 {
            return Err(DbError::InvalidInput(
                "Delta must be between 0 and 1".to_string()
            ));
        }

        let width = (std::f64::consts::E / epsilon).ceil() as usize;
        let depth = (1.0 / delta).ln().ceil() as usize;

        let counters = vec![vec![0; width]; depth];
        let seeds: Vec<u64> = (0..depth).map(|i| (i * 12345 + 67890) as u64).collect();

        Ok(Self {
            width,
            depth,
            counters,
            seeds,
        })
    }

    // Add element with count
    pub fn add<T: Hash>(&mut self, element: &T, count: u64) {
        for (i, seed) in self.seeds.iter().enumerate() {
            let hash = self.hash_with_seed(element, *seed);
            let index = (hash % self.width as u64) as usize;
            self.counters[i][index] += count;
        }
    }

    // Estimate frequency of element
    pub fn estimate<T: Hash>(&self, element: &T) -> u64 {
        let mut min_count = u64::MAX;

        for (i, seed) in self.seeds.iter().enumerate() {
            let hash = self.hash_with_seed(element, *seed);
            let index = (hash % self.width as u64) as usize;
            min_count = min_count.min(self.counters[i][index]);
        }

        min_count
    }

    // Merge another Count-Min Sketch
    pub fn merge(&mut self, other: &CountMinSketch) -> Result<()> {
        if self.width != other.width || self.depth != other.depth {
            return Err(DbError::InvalidInput(
                "Cannot merge sketches with different dimensions".to_string()
            ));
        }

        for i in 0..self.depth {
            for j in 0..self.width {
                self.counters[i][j] += other.counters[i][j];
            }
        }

        Ok(())
    }

    fn hash_with_seed<T: Hash>(&self, element: &T, seed: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        element.hash(&mut hasher);
        hasher.finish()
    }
}

// Reservoir Sampling for percentiles and quantiles
//
// Maintains a fixed-size random sample from a stream of unknown size
#[derive(Debug, Clone)]
pub struct ReservoirSampler<T> {
    // Reservoir of samples
    reservoir: Vec<T>,
    // Maximum reservoir size
    capacity: usize,
    // Number of items seen so far
    items_seen: u64,
}

impl<T: Clone> ReservoirSampler<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            reservoir: Vec::with_capacity(capacity),
            capacity,
            items_seen: 0,
        }
    }

    // Add item to reservoir
    pub fn add(&mut self, item: T) {
        self.items_seen += 1;

        if self.reservoir.len() < self.capacity {
            // Reservoir not full, just add
            self.reservoir.push(item);
        } else {
            // Reservoir full, randomly replace
            let random_index = self.random_u64() % self.items_seen;
            if random_index < self.capacity as u64 {
                self.reservoir[random_index as usize] = item;
            }
        }
    }

    // Get current sample
    pub fn sample(&self) -> &[T] {
        &self.reservoir
    }

    // Get sample size
    pub fn size(&self) -> usize {
        self.reservoir.len()
    }

    // Get items seen count
    pub fn items_seen(&self) -> u64 {
        self.items_seen
    }

    fn random_u64(&self) -> u64 {
        // Simple PRNG - in production use a proper RNG
        let mut hasher = DefaultHasher::new();
        self.items_seen.hash(&mut hasher);
        hasher.finish()
    }
}

// Stratified sampler for better accuracy on subgroups
#[derive(Debug, Clone)]
pub struct StratifiedSampler<K: Hash + Eq, V: Clone> {
    // Samplers for each stratum
    strata: HashMap<K, ReservoirSampler<V>>,
    // Sample size per stratum
    stratum_size: usize,
}

impl<K: Hash + Eq + Clone, V: Clone> StratifiedSampler<K, V> {
    pub fn new(stratum_size: usize) -> Self {
        Self {
            strata: HashMap::new(),
            stratum_size,
        }
    }

    // Add item to appropriate stratum
    pub fn add(&mut self, key: K, value: V) {
        let sampler = self.strata
            .entry(key)
            .or_insert_with(|| ReservoirSampler::new(self.stratum_size));
        sampler.add(value);
    }

    // Get sample from specific stratum
    pub fn get_stratum(&self, key: &K) -> Option<&[V]> {
        self.strata.get(key).map(|s| s.sample())
    }

    // Get all strata
    pub fn strata(&self) -> &HashMap<K, ReservoirSampler<V>> {
        &self.strata
    }
}

// Percentile estimator with confidence intervals
#[derive(Debug, Clone)]
pub struct PercentileEstimator {
    // Sorted sample values
    sample: Vec<f64>,
    // Sample size
    sample_size: usize,
    // Confidence level (e.g., 0.95 for 95%)
    confidence_level: f64,
}

impl PercentileEstimator {
    pub fn new(mut sample: Vec<f64>, confidence_level: f64) -> Self {
        sample.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sample_size = sample.len();

        Self {
            sample,
            sample_size,
            confidence_level,
        }
    }

    // Estimate percentile
    pub fn percentile(&self, p: f64) -> Result<f64> {
        if p < 0.0 || p > 100.0 {
            return Err(DbError::InvalidInput(
                "Percentile must be between 0 and 100".to_string()
            ));
        }

        if self.sample.is_empty() {
            return Err(DbError::InvalidInput("Empty sample".to_string()));
        }

        let index = (p / 100.0 * (self.sample_size - 1) as f64).floor() as usize;
        let index = index.min(self.sample_size - 1);

        Ok(self.sample[index])
    }

    // Get confidence interval for percentile
    pub fn confidence_interval(&self, p: f64) -> Result<(f64, f64)> {
        if self.sample_size < 30 {
            return Err(DbError::InvalidInput(
                "Sample size too small for confidence interval".to_string()
            ));
        }

        // Using normal approximation to binomial
        let z = Self::z_score(self.confidence_level);
        let n = self.sample_size as f64;
        let p_decimal = p / 100.0;

        let variance = p_decimal * (1.0 - p_decimal) / n;
        let std_err = variance.sqrt();

        let lower_p = (p_decimal - z * std_err).max(0.0);
        let upper_p = (p_decimal + z * std_err).min(1.0);

        let lower_val = self.percentile(lower_p * 100.0)?;
        let upper_val = self.percentile(upper_p * 100.0)?;

        Ok((lower_val, upper_val))
    }

    // Get median
    pub fn median(&self) -> Result<f64> {
        self.percentile(50.0)
    }

    // Get interquartile range
    pub fn iqr(&self) -> Result<f64> {
        let q1 = self.percentile(25.0)?;
        let q3 = self.percentile(75.0)?;
        Ok(q3 - q1)
    }

    fn z_score(confidence: f64) -> f64 {
        // Common z-scores for confidence levels
        match confidence {
            c if (c - 0.90).abs() < 0.001 => 1.645,
            c if (c - 0.95).abs() < 0.001 => 1.96,
            c if (c - 0.99).abs() < 0.001 => 2.576,
            _ => 1.96, // Default to 95%
        }
    }
}

// Approximate aggregate query executor
pub struct ApproximateQueryExecutor {
    // HyperLogLog for distinct counts
    distinct_estimators: HashMap<String, HyperLogLog>,
    // Count-Min Sketch for frequencies
    frequency_estimators: HashMap<String, CountMinSketch>,
    // Reservoir samplers for samples
    samplers: HashMap<String, ReservoirSampler<Vec<String>>>,
}

impl ApproximateQueryExecutor {
    pub fn new() -> Self {
        Self {
            distinct_estimators: HashMap::new(),
            frequency_estimators: HashMap::new(),
            samplers: HashMap::new(),
        }
    }

    // Create distinct count estimator for column
    pub fn create_distinct_estimator(
        &mut self,
        column: String,
        precision: u8,
    ) -> Result<()> {
        let hll = HyperLogLog::new(precision)?;
        self.distinct_estimators.insert(column, hll);
        Ok(())
    }

    // Add value to distinct estimator
    pub fn add_to_distinct(&mut self, column: &str, value: &str) -> Result<()> {
        let estimator = self.distinct_estimators.get_mut(column)
            .ok_or_else(|| DbError::NotFound(format!("Estimator for column: {}", column)))?;
        estimator.add(&value);
        Ok(())
    }

    // Estimate distinct count
    pub fn estimate_distinct(&self, column: &str) -> Result<ApproximateResult> {
        let estimator = self.distinct_estimators.get(column)
            .ok_or_else(|| DbError::NotFound(format!("Estimator for column: {}", column)))?;

        let estimate = estimator.cardinality();
        let error_bound = estimator.standard_error();

        Ok(ApproximateResult {
            value: estimate as f64,
            error_bound,
            confidence_level: 0.95,
            sample_size: None,
        })
    }

    // Create frequency estimator for column
    pub fn create_frequency_estimator(
        &mut self,
        column: String,
        epsilon: f64,
        delta: f64,
    ) -> Result<()> {
        let cms = CountMinSketch::new(epsilon, delta)?;
        self.frequency_estimators.insert(column, cms);
        Ok(())
    }

    // Add value to frequency estimator
    pub fn add_to_frequency(&mut self, column: &str, value: &str, count: u64) -> Result<()> {
        let estimator = self.frequency_estimators.get_mut(column)
            .ok_or_else(|| DbError::NotFound(format!("Frequency estimator for: {}", column)))?;
        estimator.add(&value, count);
        Ok(())
    }

    // Estimate frequency of value
    pub fn estimate_frequency(&self, column: &str, value: &str) -> Result<u64> {
        let estimator = self.frequency_estimators.get(column)
            .ok_or_else(|| DbError::NotFound(format!("Frequency estimator for: {}", column)))?;
        Ok(estimator.estimate(&value))
    }

    // Create reservoir sampler
    pub fn create_sampler(&mut self, name: String, capacity: usize) {
        let sampler = ReservoirSampler::new(capacity);
        self.samplers.insert(name, sampler);
    }

    // Add row to sampler
    pub fn add_to_sample(&mut self, name: &str, row: Vec<String>) -> Result<()> {
        let sampler = self.samplers.get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Sampler: {}", name)))?;
        sampler.add(row);
        Ok(())
    }

    // Get sample
    pub fn get_sample(&self, name: &str) -> Result<&[Vec<String>]> {
        let sampler = self.samplers.get(name)
            .ok_or_else(|| DbError::NotFound(format!("Sampler: {}", name)))?;
        Ok(sampler.sample())
    }
}

// Result of an approximate query
#[derive(Debug, Clone)]
pub struct ApproximateResult {
    // Estimated value
    pub value: f64,
    // Error bound (e.g., 0.02 for 2% error)
    pub error_bound: f64,
    // Confidence level (e.g., 0.95 for 95% confidence)
    pub confidence_level: f64,
    // Sample size if applicable
    pub sample_size: Option<usize>,
}

impl ApproximateResult {
    // Get lower bound of confidence interval
    pub fn lower_bound(&self) -> f64 {
        self.value * (1.0 - self.error_bound)
    }

    // Get upper bound of confidence interval
    pub fn upper_bound(&self) -> f64 {
        self.value * (1.0 + self.error_bound)
    }

    // Get confidence interval
    pub fn confidence_interval(&self) -> (f64, f64) {
        (self.lower_bound(), self.upper_bound())
    }
}

// Online variance calculator using Welford's algorithm
#[derive(Debug, Clone)]
pub struct OnlineVariance {
    count: u64,
    mean: f64,
    m2: f64, // Sum of squared differences from mean
}

impl OnlineVariance {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    // Add a value
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    // Get mean
    pub fn mean(&self) -> f64 {
        self.mean
    }

    // Get variance
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    // Get standard deviation
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    // Get count
    pub fn count(&self) -> u64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperloglog() {
        let mut hll = HyperLogLog::new(10).unwrap();

        // Add 10,000 distinct elements
        for i in 0..10000 {
            hll.add(&i);
        }

        let estimate = hll.cardinality();
        let error = (estimate as f64 - 10000.0).abs() / 10000.0;

        // Should be within ~2% error
        assert!(error < 0.05, "Error: {}", error);
    }

    #[test]
    fn test_count_min_sketch() {
        let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

        // Add elements with different frequencies
        for i in 0..100 {
            cms.add(&"common", i);
        }
        cms.add(&"rare", 1);

        let common_freq = cms.estimate(&"common");
        let rare_freq = cms.estimate(&"rare");

        assert!(common_freq >= 100);
        assert!(rare_freq >= 1);
        assert!(common_freq > rare_freq);
    }

    #[test]
    fn test_reservoir_sampler() {
        let mut sampler = ReservoirSampler::new(100);

        // Add 1000 items
        for i in 0..1000 {
            sampler.add(i);
        }

        assert_eq!(sampler.size(), 100);
        assert_eq!(sampler.items_seen(), 1000);
    }

    #[test]
    fn test_percentile_estimator() {
        let sample: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let estimator = PercentileEstimator::new(sample, 0.95);

        let median = estimator.median().unwrap();
        assert!((median - 49.0).abs() < 1.0);

        let p95 = estimator.percentile(95.0).unwrap();
        assert!((p95 - 94.0).abs() < 1.0);
    }

    #[test]
    fn test_online_variance() {
        let mut variance = OnlineVariance::new();

        for i in 0..100 {
            variance.add(i as f64);
        }

        let mean = variance.mean();
        assert!((mean - 49.5).abs() < 0.1);

        let std_dev = variance.std_dev();
        assert!(std_dev > 0.0);
    }

    #[test]
    fn test_approximate_query_executor() {
        let mut executor = ApproximateQueryExecutor::new();

        executor.create_distinct_estimator("user_id".to_string(), 10).unwrap();

        for i in 0..1000 {
            executor.add_to_distinct("user_id", &i.to_string()).unwrap();
        }

        let result = executor.estimate_distinct("user_id").unwrap();
        assert!(result.value > 900.0);
        assert!(result.value < 1100.0);
    }
}
