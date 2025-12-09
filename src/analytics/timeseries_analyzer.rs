// Time Series Analysis
//
// This module provides time series analytics capabilities:
//
// - **Moving Averages**: Simple and exponential
// - **Trend Detection**: Identify upward/downward trends
// - **Seasonality Detection**: Detect periodic patterns
// - **Forecasting**: Simple time series forecasting
// - **Anomaly Detection**: Identify outliers

use serde::{Deserialize, Serialize};

// =============================================================================
// Time Series Analyzer
// =============================================================================

/// Time series analyzer for detecting patterns and forecasting.
pub struct TimeSeriesAnalyzer {
    /// Window size for moving average calculations
    window_size: usize,
}

impl TimeSeriesAnalyzer {
    /// Create a new analyzer with the given window size.
    pub fn new(window_size: usize) -> Self {
        Self { window_size }
    }

    /// Compute simple moving average.
    ///
    /// Returns a vector of moving averages, one per input value.
    pub fn moving_average(&self, data: &[f64]) -> Vec<f64> {
        let mut result = Vec::with_capacity(data.len());

        for i in 0..data.len() {
            let start = if i >= self.window_size {
                i - self.window_size + 1
            } else {
                0
            };
            let window = &data[start..=i];
            let avg = window.iter().sum::<f64>() / window.len() as f64;
            result.push(avg);
        }

        result
    }

    /// Compute exponential moving average.
    ///
    /// # Arguments
    /// * `data` - Input time series
    /// * `alpha` - Smoothing factor (0 < alpha <= 1)
    pub fn exponential_moving_average(&self, data: &[f64], alpha: f64) -> Vec<f64> {
        if data.is_empty() {
            return Vec::new();
        }

        let alpha = alpha.clamp(0.0, 1.0);
        let mut result = vec![data[0]];

        for i in 1..data.len() {
            let ema = alpha * data[i] + (1.0 - alpha) * result[i - 1];
            result.push(ema);
        }

        result
    }

    /// Compute weighted moving average.
    pub fn weighted_moving_average(&self, data: &[f64]) -> Vec<f64> {
        let mut result = Vec::with_capacity(data.len());

        for i in 0..data.len() {
            let start = if i >= self.window_size {
                i - self.window_size + 1
            } else {
                0
            };
            let window = &data[start..=i];

            // Weights increase linearly
            let weights: Vec<f64> = (1..=window.len()).map(|w| w as f64).collect();
            let weight_sum: f64 = weights.iter().sum();

            let weighted_sum: f64 = window
                .iter()
                .zip(weights.iter())
                .map(|(v, w)| v * w)
                .sum();

            result.push(weighted_sum / weight_sum);
        }

        result
    }

    /// Detect the overall trend in the data.
    pub fn detect_trend(&self, data: &[f64]) -> Trend {
        if data.len() < 2 {
            return Trend::Stable;
        }

        let mut increases = 0;
        let mut decreases = 0;

        for i in 1..data.len() {
            if data[i] > data[i - 1] {
                increases += 1;
            } else if data[i] < data[i - 1] {
                decreases += 1;
            }
        }

        if increases > decreases * 2 {
            Trend::Increasing
        } else if decreases > increases * 2 {
            Trend::Decreasing
        } else {
            Trend::Stable
        }
    }

    /// Calculate trend strength (0.0 to 1.0).
    pub fn trend_strength(&self, data: &[f64]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }

        let mut increases = 0;
        let mut decreases = 0;

        for i in 1..data.len() {
            if data[i] > data[i - 1] {
                increases += 1;
            } else if data[i] < data[i - 1] {
                decreases += 1;
            }
        }

        let max_changes = data.len() - 1;
        let dominant = increases.max(decreases);

        dominant as f64 / max_changes as f64
    }

    /// Detect seasonality using autocorrelation.
    ///
    /// Returns true if significant seasonality is detected at the given period.
    pub fn detect_seasonality(&self, data: &[f64], period: usize) -> bool {
        if data.len() < period * 2 {
            return false;
        }

        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;

        if variance == 0.0 {
            return false;
        }

        let n = data.len() - period;
        let mut autocorr = 0.0;

        for i in 0..n {
            autocorr += (data[i] - mean) * (data[i + period] - mean);
        }

        autocorr /= n as f64 * variance;

        autocorr > 0.5 // Threshold for seasonality detection
    }

    /// Find the dominant period in the data.
    pub fn find_period(&self, data: &[f64], max_period: usize) -> Option<usize> {
        let max_period = max_period.min(data.len() / 2);

        let mut best_period = None;
        let mut best_autocorr = 0.5; // Minimum threshold

        for period in 2..=max_period {
            if self.detect_seasonality(data, period) {
                let autocorr = self.autocorrelation(data, period);
                if autocorr > best_autocorr {
                    best_autocorr = autocorr;
                    best_period = Some(period);
                }
            }
        }

        best_period
    }

    /// Compute autocorrelation at a specific lag.
    fn autocorrelation(&self, data: &[f64], lag: usize) -> f64 {
        if data.len() <= lag {
            return 0.0;
        }

        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;

        if variance == 0.0 {
            return 0.0;
        }

        let n = data.len() - lag;
        let mut autocorr = 0.0;

        for i in 0..n {
            autocorr += (data[i] - mean) * (data[i + lag] - mean);
        }

        autocorr / (n as f64 * variance)
    }

    /// Simple linear forecasting.
    ///
    /// Extrapolates the trend into the future.
    pub fn forecast(&self, data: &[f64], periods: usize) -> Vec<f64> {
        if data.is_empty() {
            return vec![0.0; periods];
        }

        let n = data.len();
        if n < 2 {
            return vec![data[0]; periods];
        }

        // Calculate linear regression
        let x_mean = (n - 1) as f64 / 2.0;
        let y_mean = data.iter().sum::<f64>() / n as f64;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, y) in data.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        let slope = if denominator != 0.0 {
            numerator / denominator
        } else {
            0.0
        };
        let intercept = y_mean - slope * x_mean;

        // Generate forecasts
        (0..periods)
            .map(|i| intercept + slope * (n + i) as f64)
            .collect()
    }

    /// Calculate rate of change.
    pub fn rate_of_change(&self, data: &[f64], period: usize) -> Vec<f64> {
        if data.len() <= period {
            return vec![0.0; data.len()];
        }

        let mut result = vec![0.0; period];

        for i in period..data.len() {
            let roc = if data[i - period] != 0.0 {
                (data[i] - data[i - period]) / data[i - period] * 100.0
            } else {
                0.0
            };
            result.push(roc);
        }

        result
    }
}

impl Default for TimeSeriesAnalyzer {
    fn default() -> Self {
        Self::new(7)
    }
}

// =============================================================================
// Trend Types
// =============================================================================

/// Trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trend {
    /// Values are increasing
    Increasing,
    /// Values are decreasing
    Decreasing,
    /// Values are relatively stable
    Stable,
}

// =============================================================================
// Anomaly Detector
// =============================================================================

/// Anomaly detector for identifying outliers in data.
pub struct AnomalyDetector {
    /// Number of standard deviations for outlier threshold
    threshold_stddev: f64,
}

impl AnomalyDetector {
    /// Create a new anomaly detector.
    ///
    /// # Arguments
    /// * `threshold_stddev` - Number of standard deviations for outlier detection
    pub fn new(threshold_stddev: f64) -> Self {
        Self { threshold_stddev }
    }

    /// Detect outliers using z-score method.
    ///
    /// Returns indices of outlier values.
    pub fn detect_outliers(&self, data: &[f64]) -> Vec<usize> {
        if data.len() < 3 {
            return Vec::new();
        }

        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
        let stddev = variance.sqrt();

        if stddev == 0.0 {
            return Vec::new();
        }

        data.iter()
            .enumerate()
            .filter(|(_, &value)| (value - mean).abs() > self.threshold_stddev * stddev)
            .map(|(i, _)| i)
            .collect()
    }

    /// Detect anomalies using IQR (Interquartile Range) method.
    ///
    /// More robust to extreme outliers than z-score.
    pub fn detect_anomalies_iqr(&self, data: &[f64]) -> Vec<usize> {
        if data.len() < 4 {
            return Vec::new();
        }

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1_idx = sorted.len() / 4;
        let q3_idx = 3 * sorted.len() / 4;

        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        data.iter()
            .enumerate()
            .filter(|(_, &value)| value < lower_bound || value > upper_bound)
            .map(|(i, _)| i)
            .collect()
    }

    /// Detect anomalies using modified z-score (MAD-based).
    ///
    /// Uses Median Absolute Deviation, robust to outliers.
    pub fn detect_anomalies_mad(&self, data: &[f64]) -> Vec<usize> {
        if data.len() < 3 {
            return Vec::new();
        }

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let median = sorted[sorted.len() / 2];

        let mut deviations: Vec<f64> = data.iter().map(|x| (x - median).abs()).collect();
        deviations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mad = deviations[deviations.len() / 2];

        if mad == 0.0 {
            return Vec::new();
        }

        // Modified z-score threshold (commonly 3.5)
        let threshold = 3.5;

        data.iter()
            .enumerate()
            .filter(|(_, &value)| {
                let modified_z = 0.6745 * (value - median) / mad;
                modified_z.abs() > threshold
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Get outlier values.
    pub fn get_outlier_values(&self, data: &[f64]) -> Vec<f64> {
        let indices = self.detect_outliers(data);
        indices.iter().map(|&i| data[i]).collect()
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new(2.0)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average() {
        let analyzer = TimeSeriesAnalyzer::new(3);
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = analyzer.moving_average(&data);

        assert_eq!(ma.len(), data.len());
        assert!((ma[0] - 1.0).abs() < 0.001);
        assert!((ma[4] - 4.0).abs() < 0.001); // Average of [3, 4, 5]
    }

    #[test]
    fn test_exponential_moving_average() {
        let analyzer = TimeSeriesAnalyzer::new(3);
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ema = analyzer.exponential_moving_average(&data, 0.5);

        assert_eq!(ema.len(), data.len());
        assert_eq!(ema[0], 1.0);
        assert!(ema[4] > ema[0]); // Should be increasing
    }

    #[test]
    fn test_trend_detection() {
        let analyzer = TimeSeriesAnalyzer::new(3);

        let increasing = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(analyzer.detect_trend(&increasing), Trend::Increasing);

        let decreasing = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        assert_eq!(analyzer.detect_trend(&decreasing), Trend::Decreasing);

        let stable = vec![3.0, 3.1, 2.9, 3.0, 3.1];
        assert_eq!(analyzer.detect_trend(&stable), Trend::Stable);
    }

    #[test]
    fn test_forecast() {
        let analyzer = TimeSeriesAnalyzer::new(3);
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let forecast = analyzer.forecast(&data, 3);

        assert_eq!(forecast.len(), 3);
        // Should continue the trend
        assert!(forecast[0] > 5.0);
        assert!(forecast[1] > forecast[0]);
    }

    #[test]
    fn test_anomaly_detection_zscore() {
        let detector = AnomalyDetector::new(2.0);
        let data = vec![1.0, 2.0, 2.5, 2.0, 100.0, 2.5]; // 100.0 is an outlier
        let outliers = detector.detect_outliers(&data);

        assert!(outliers.contains(&4)); // Index of 100.0
    }

    #[test]
    fn test_anomaly_detection_iqr() {
        let detector = AnomalyDetector::new(2.0);
        let data = vec![1.0, 2.0, 2.5, 2.0, 100.0, 2.5];
        let outliers = detector.detect_anomalies_iqr(&data);

        assert!(outliers.contains(&4));
    }

    #[test]
    fn test_rate_of_change() {
        let analyzer = TimeSeriesAnalyzer::new(3);
        let data = vec![100.0, 110.0, 121.0]; // 10% increase each
        let roc = analyzer.rate_of_change(&data, 1);

        assert!((roc[1] - 10.0).abs() < 0.001);
        assert!((roc[2] - 10.0).abs() < 0.001);
    }
}
