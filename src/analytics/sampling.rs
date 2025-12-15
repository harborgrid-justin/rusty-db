// Query Sampling and Approximate Query Processing
//
// This module provides sampling-based techniques for approximate query
// processing, enabling fast answers to analytical queries with bounded
// error margins.
//
// # Architecture
//
// The sampling subsystem supports multiple sampling strategies:
// - Random sampling for uniform distribution
// - Stratified sampling for skewed data
// - Reservoir sampling for streaming data
// - Systematic sampling for ordered data
//
// # Example
//
// ```rust,ignore
// use crate::analytics::sampling::{QueryResultSampler, SamplingMethod};
//
// let sampler = QueryResultSampler::new(SamplingMethod::Random, 0.01);
// let sample = sampler.sample(&data);
// let estimate = sampler.estimate_aggregate(&sample, AggregateFunction::Sum);
// ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

// Sampling method for approximate query processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamplingMethod {
    // Simple random sampling with replacement
    Random,
    // Random sampling without replacement
    RandomWithoutReplacement,
    // Stratified sampling based on grouping column
    Stratified,
    // Reservoir sampling for streaming data
    Reservoir,
    // Systematic sampling (every Nth row)
    Systematic,
    // Bernoulli sampling (coin flip per row)
    Bernoulli,
    // Cluster-based sampling
    Cluster,
}

impl SamplingMethod {
    // Returns whether the method preserves distribution characteristics.
    pub fn preserves_distribution(&self) -> bool {
        matches!(self, SamplingMethod::Stratified | SamplingMethod::Cluster)
    }

    // Returns whether the method works with streaming data.
    pub fn supports_streaming(&self) -> bool {
        matches!(self, SamplingMethod::Reservoir | SamplingMethod::Bernoulli)
    }
}

// Configuration for sampling operations.
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    // Sampling method to use
    pub method: SamplingMethod,
    // Sample rate (0.0 to 1.0) or absolute size
    pub rate: f64,
    // Whether rate is a percentage or absolute count
    pub is_percentage: bool,
    // Random seed for reproducibility (None for random)
    pub seed: Option<u64>,
    // Stratification column for stratified sampling
    pub strata_column: Option<String>,
    // Minimum sample size per stratum
    pub min_per_stratum: usize,
    // Maximum iterations for complex sampling
    pub max_iterations: usize,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            method: SamplingMethod::Random,
            rate: 0.01,
            is_percentage: true,
            seed: None,
            strata_column: None,
            min_per_stratum: 10,
            max_iterations: 1000,
        }
    }
}

impl SamplingConfig {
    // Creates a new sampling configuration.
    pub fn new(method: SamplingMethod, rate: f64) -> Self {
        Self {
            method,
            rate,
            ..Default::default()
        }
    }

    // Sets the random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    // Configures for stratified sampling.
    pub fn stratified(mut self, column: impl Into<String>) -> Self {
        self.method = SamplingMethod::Stratified;
        self.strata_column = Some(column.into());
        self
    }
}

// A sampled result with metadata about the sampling.
#[derive(Debug, Clone)]
pub struct SampledResult<T> {
    // The sampled data
    pub data: Vec<T>,
    // Original population size
    pub population_size: usize,
    // Actual sample size
    pub sample_size: usize,
    // Sampling fraction achieved
    pub sampling_fraction: f64,
    // Method used for sampling
    pub method: SamplingMethod,
    // Statistical weight for each sample row
    pub weights: Option<Vec<f64>>,
}

impl<T> SampledResult<T> {
    // Returns the expansion factor for aggregate estimation.
    pub fn expansion_factor(&self) -> f64 {
        if self.sample_size == 0 {
            0.0
        } else {
            self.population_size as f64 / self.sample_size as f64
        }
    }

    // Returns the standard error multiplier for confidence intervals.
    pub fn standard_error_multiplier(&self) -> f64 {
        if self.sample_size == 0 {
            return 0.0;
        }

        // Finite population correction
        let fpc = if self.population_size > 0 {
            ((self.population_size - self.sample_size) as f64 / (self.population_size - 1) as f64)
                .sqrt()
        } else {
            1.0
        };

        fpc / (self.sample_size as f64).sqrt()
    }
}

// Sampler for query results with various sampling strategies.
#[derive(Debug)]
pub struct QueryResultSampler {
    // Sampling configuration
    config: SamplingConfig,
    // Pseudo-random state
    rng_state: u64,
}

impl QueryResultSampler {
    // Creates a new query result sampler.
    pub fn new(method: SamplingMethod, rate: f64) -> Self {
        Self {
            config: SamplingConfig::new(method, rate),
            rng_state: 12345,
        }
    }

    // Creates a sampler with full configuration.
    pub fn with_config(config: SamplingConfig) -> Self {
        let seed = config.seed.unwrap_or(12345);
        Self {
            config,
            rng_state: seed,
        }
    }

    // Simple pseudo-random number generator (xorshift).
    fn next_random(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }

    // Samples from a slice of data.
    pub fn sample<T: Clone>(&mut self, data: &[T]) -> SampledResult<T> {
        let sample_size = if self.config.is_percentage {
            ((data.len() as f64) * self.config.rate).ceil() as usize
        } else {
            self.config.rate as usize
        };

        let sample_size = sample_size.min(data.len()).max(1);

        let sampled = match self.config.method {
            SamplingMethod::Random | SamplingMethod::RandomWithoutReplacement => {
                self.random_sample(data, sample_size)
            }
            SamplingMethod::Systematic => self.systematic_sample(data, sample_size),
            SamplingMethod::Bernoulli => self.bernoulli_sample(data),
            SamplingMethod::Reservoir => self.reservoir_sample(data, sample_size),
            _ => self.random_sample(data, sample_size),
        };

        SampledResult {
            sample_size: sampled.len(),
            sampling_fraction: sampled.len() as f64 / data.len() as f64,
            data: sampled,
            population_size: data.len(),
            method: self.config.method.clone(),
            weights: None,
        }
    }

    // Random sampling with replacement.
    fn random_sample<T: Clone>(&mut self, data: &[T], n: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(n);

        for _ in 0..n {
            let idx = (self.next_random() * data.len() as f64) as usize;
            let idx = idx.min(data.len() - 1);
            result.push(data[idx].clone());
        }

        result
    }

    // Systematic sampling (every kth element).
    fn systematic_sample<T: Clone>(&mut self, data: &[T], n: usize) -> Vec<T> {
        if n >= data.len() {
            return data.to_vec();
        }

        let step = data.len() / n;
        let start = (self.next_random() * step as f64) as usize;

        data.iter()
            .skip(start)
            .step_by(step)
            .take(n)
            .cloned()
            .collect()
    }

    // Bernoulli sampling (coin flip per row).
    fn bernoulli_sample<T: Clone>(&mut self, data: &[T]) -> Vec<T> {
        data.iter()
            .filter(|_| self.next_random() < self.config.rate)
            .cloned()
            .collect()
    }

    // Reservoir sampling for streaming data.
    fn reservoir_sample<T: Clone>(&mut self, data: &[T], k: usize) -> Vec<T> {
        let mut reservoir: Vec<T> = data.iter().take(k).cloned().collect();

        for (i, item) in data.iter().enumerate().skip(k) {
            let j = (self.next_random() * (i + 1) as f64) as usize;
            if j < k {
                reservoir[j] = item.clone();
            }
        }

        reservoir
    }

    // Estimates a count based on sample.
    pub fn estimate_count<T>(&self, sample: &SampledResult<T>) -> f64 {
        sample.sample_size as f64 * sample.expansion_factor()
    }

    // Estimates sum from numeric sample.
    pub fn estimate_sum(&self, sample: &SampledResult<f64>) -> f64 {
        let sum: f64 = sample.data.iter().sum();
        sum * sample.expansion_factor()
    }

    // Estimates average from numeric sample.
    pub fn estimate_avg(&self, sample: &SampledResult<f64>) -> f64 {
        if sample.data.is_empty() {
            return 0.0;
        }
        sample.data.iter().sum::<f64>() / sample.data.len() as f64
    }

    // Calculates confidence interval for sum estimate.
    pub fn confidence_interval_sum(
        &self,
        sample: &SampledResult<f64>,
        confidence_level: f64,
    ) -> (f64, f64) {
        let estimate = self.estimate_sum(sample);

        if sample.data.is_empty() {
            return (0.0, 0.0);
        }

        // Calculate sample variance
        let mean = sample.data.iter().sum::<f64>() / sample.data.len() as f64;
        let variance: f64 = sample.data.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
            / (sample.data.len() - 1).max(1) as f64;

        let std_dev = variance.sqrt();
        let z_score = self.z_score_for_confidence(confidence_level);

        let margin =
            z_score * std_dev * sample.expansion_factor() * sample.standard_error_multiplier();

        (estimate - margin, estimate + margin)
    }

    // Returns z-score for common confidence levels.
    fn z_score_for_confidence(&self, confidence: f64) -> f64 {
        match confidence {
            c if c >= 0.99 => 2.576,
            c if c >= 0.95 => 1.96,
            c if c >= 0.90 => 1.645,
            c if c >= 0.80 => 1.28,
            _ => 1.96,
        }
    }
}

// Approximate query processor for fast analytical queries.
#[derive(Debug)]
pub struct ApproximateQueryProcessor {
    // Sample cache by table/query
    sample_cache: Arc<RwLock<HashMap<String, SampledData>>>,
    // Default sample rate
    _default_sample_rate: f64,
    // Maximum cache size
    max_cache_entries: usize,
}

// Cached sample data for a table.
#[derive(Debug, Clone)]
pub struct SampledData {
    // Column values (column name -> values)
    pub columns: HashMap<String, Vec<f64>>,
    // Original table size
    pub original_size: usize,
    // Sample creation time
    pub created_at: std::time::Instant,
    // Sampling method used
    pub method: SamplingMethod,
    // Sample rate
    pub rate: f64,
}

impl ApproximateQueryProcessor {
    // Creates a new approximate query processor.
    pub fn new(default_sample_rate: f64) -> Self {
        Self {
            sample_cache: Arc::new(RwLock::new(HashMap::new())),
            _default_sample_rate: default_sample_rate,
            max_cache_entries: 100,
        }
    }

    // Registers a pre-computed sample for a table.
    pub fn register_sample(&self, table_name: &str, sample: SampledData) {
        let mut cache = self.sample_cache.write();

        // Evict oldest if at capacity
        if cache.len() >= self.max_cache_entries {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, s)| s.created_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }

        cache.insert(table_name.to_string(), sample);
    }

    // Retrieves a cached sample for a table.
    pub fn get_sample(&self, table_name: &str) -> Option<SampledData> {
        self.sample_cache.read().get(table_name).cloned()
    }

    // Estimates count with error bounds.
    pub fn estimate_count_with_bounds(
        &self,
        table_name: &str,
        predicate: impl Fn(&HashMap<String, f64>) -> bool,
    ) -> Option<ApproximateResult> {
        let cache = self.sample_cache.read();
        let sample = cache.get(table_name)?;

        // Count matching rows
        let mut matches = 0usize;
        let num_rows = sample.columns.values().next().map(|v| v.len()).unwrap_or(0);

        for i in 0..num_rows {
            let mut row: HashMap<String, f64> = HashMap::new();
            for (col, values) in &sample.columns {
                if let Some(val) = values.get(i) {
                    row.insert(col.clone(), *val);
                }
            }

            if predicate(&row) {
                matches += 1;
            }
        }

        let expansion = sample.original_size as f64 / num_rows as f64;
        let estimate = matches as f64 * expansion;

        // Calculate error bounds (using binomial proportion confidence interval)
        let p = matches as f64 / num_rows as f64;
        let se = (p * (1.0 - p) / num_rows as f64).sqrt();
        let margin = 1.96 * se * sample.original_size as f64;

        Some(ApproximateResult {
            estimate,
            lower_bound: (estimate - margin).max(0.0),
            upper_bound: estimate + margin,
            confidence: 0.95,
            sample_size: num_rows,
        })
    }

    // Estimates sum with error bounds.
    pub fn estimate_sum_with_bounds(
        &self,
        table_name: &str,
        column: &str,
    ) -> Option<ApproximateResult> {
        let cache = self.sample_cache.read();
        let sample = cache.get(table_name)?;
        let values = sample.columns.get(column)?;

        let sum: f64 = values.iter().sum();
        let expansion = sample.original_size as f64 / values.len() as f64;
        let estimate = sum * expansion;

        // Calculate variance for confidence interval
        let mean = sum / values.len() as f64;
        let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
            / (values.len() - 1).max(1) as f64;

        let std_err = (variance / values.len() as f64).sqrt() * expansion;
        let margin = 1.96 * std_err;

        Some(ApproximateResult {
            estimate,
            lower_bound: estimate - margin,
            upper_bound: estimate + margin,
            confidence: 0.95,
            sample_size: values.len(),
        })
    }

    // Clears the sample cache.
    pub fn clear_cache(&self) {
        self.sample_cache.write().clear();
    }

    // Returns cache statistics.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.sample_cache.read();
        (cache.len(), self.max_cache_entries)
    }
}

// Result of an approximate query with confidence bounds.
#[derive(Debug, Clone)]
pub struct ApproximateResult {
    // Point estimate
    pub estimate: f64,
    // Lower bound of confidence interval
    pub lower_bound: f64,
    // Upper bound of confidence interval
    pub upper_bound: f64,
    // Confidence level (e.g., 0.95 for 95%)
    pub confidence: f64,
    // Sample size used
    pub sample_size: usize,
}

impl ApproximateResult {
    // Returns the error margin as a percentage.
    pub fn error_percentage(&self) -> f64 {
        if self.estimate == 0.0 {
            return 0.0;
        }
        ((self.upper_bound - self.lower_bound) / 2.0 / self.estimate.abs()) * 100.0
    }

    // Returns whether the result is precise enough.
    pub fn is_acceptable(&self, max_error_percent: f64) -> bool {
        self.error_percentage() <= max_error_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_sampling() {
        let data: Vec<f64> = (0..1000).map(|x| x as f64).collect();
        let mut sampler = QueryResultSampler::new(SamplingMethod::Random, 0.1);

        let result = sampler.sample(&data);

        assert!(result.sample_size > 50 && result.sample_size < 200);
        assert_eq!(result.population_size, 1000);
    }

    #[test]
    fn test_systematic_sampling() {
        let data: Vec<i32> = (0..100).collect();
        let mut sampler = QueryResultSampler::new(SamplingMethod::Systematic, 0.1);

        let result = sampler.sample(&data);

        assert_eq!(result.sample_size, 10);
    }

    #[test]
    fn test_estimate_sum() {
        let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let mut sampler = QueryResultSampler::new(SamplingMethod::Random, 1.0);

        let sample = sampler.sample(&data);
        let estimate = sampler.estimate_sum(&sample);

        // With 100% sample, estimate should be close to actual
        assert!((estimate - 150.0).abs() < 1.0);
    }

    #[test]
    fn test_approximate_result() {
        let result = ApproximateResult {
            estimate: 1000.0,
            lower_bound: 900.0,
            upper_bound: 1100.0,
            confidence: 0.95,
            sample_size: 100,
        };

        assert_eq!(result.error_percentage(), 10.0);
        assert!(result.is_acceptable(15.0));
        assert!(!result.is_acceptable(5.0));
    }
}
