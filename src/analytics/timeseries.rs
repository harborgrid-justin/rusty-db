/// Time-Series Analytics Engine
///
/// This module provides specialized time-series analysis capabilities:
/// - Gap filling and missing data interpolation
/// - Time bucketing and downsampling
/// - Moving averages and exponential smoothing
/// - Trend detection and decomposition
/// - Seasonality detection and patterns
/// - Forecasting with multiple methods
/// - Change point detection

use std::collections::BTreeMap;
use std::time::SystemTime;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::time::{Duration};

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

/// Time-series with metadata
#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub name: String,
    pub points: Vec<TimeSeriesPoint>,
    pub interval: Option<Duration>,
    pub metadata: TimeSeriesMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetadata {
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub point_count: usize,
    pub has_gaps: bool,
    pub is_regular: bool,
    pub detected_interval: Option<Duration>,
}

/// Time bucket specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBucket {
    pub duration: Duration,
    pub alignment: BucketAlignment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BucketAlignment {
    /// Align to epoch
    Epoch,
    /// Align to calendar boundaries
    Calendar,
    /// Align to first data point
    FirstPoint,
}

/// Interpolation method for gap filling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterpolationMethod {
    /// Previous value (step function)
    Previous,
    /// Next value
    Next,
    /// Linear interpolation
    Linear,
    /// Spline interpolation
    Spline,
    /// Average of neighbors
    Average,
    /// Fill with constant
    Constant(f64),
    /// Fill with null/NaN
    Null,
}

/// Aggregation method for downsampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationMethod {
    Mean,
    Sum,
    Min,
    Max,
    First,
    Last,
    Count,
    StdDev,
    Median,
}

/// Moving average type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MovingAverageType {
    /// Simple moving average
    Simple { window_size: usize },
    /// Exponential moving average
    Exponential { alpha: f64 },
    /// Weighted moving average
    Weighted { weights: Vec<f64> },
    /// Cumulative moving average
    Cumulative,
}

/// Time-series analyzer
pub struct TimeSeriesAnalyzer {
    series: TimeSeries,
}

impl TimeSeriesAnalyzer {
    pub fn new(series: TimeSeries) -> Self {
        Self { series }
    }

    /// Fill gaps in time series
    pub fn fill_gaps(
        &self,
        expected_interval: Duration,
        method: InterpolationMethod,
    ) -> Result<TimeSeries> {
        if self.series.points.is_empty() {
            return Ok(self.series.clone());
        }

        let mut filled_points = Vec::new();
        let mut current_time = self.series.points[0].timestamp;
        let end_time = self.series.points.last().unwrap().timestamp;

        let mut point_index = 0;

        while current_time <= end_time {
            // Check if we have a point at current_time
            if point_index < self.series.points.len() {
                let point = &self.series.points[point_index];
                let point_secs = point.timestamp.duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0)).as_secs();
                let current_secs = current_time.duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0)).as_secs();

                if point_secs == current_secs {
                    // Exact match
                    filled_points.push(point.clone());
                    point_index += 1;
                } else {
                    // Gap - interpolate
                    let _value = self.interpolate_value(current_time, &method)?;
                    filled_points.push(TimeSeriesPoint {
                        timestamp: current_time,
                        value,
                        metadata: HashMap::new(),
                    });
                }
            } else {
                // Beyond last point - extrapolate or use method
                let _value = self.interpolate_value(current_time, &method)?;
                filled_points.push(TimeSeriesPoint {
                    timestamp: current_time,
                    value,
                    metadata: HashMap::new(),
                });
            }

            current_time = current_time + expected_interval;
        }

        let metadata = self.calculate_metadata(&filled_points);
        Ok(TimeSeries {
            name: format!("{}_filled", self.series.name),
            points: filled_points,
            interval: Some(expected_interval),
            metadata,
        })
    }

    /// Interpolate value at given timestamp
    fn interpolate_value(
        &self,
        timestamp: SystemTime,
        method: &InterpolationMethod,
    ) -> Result<f64> {
        match method {
            InterpolationMethod::Previous => {
                // Find last point before timestamp
                let prev = self.series.points.iter()
                    .filter(|p| p.timestamp <= timestamp)
                    .last();
                Ok(prev.map(|p| p.value).unwrap_or(0.0))
            }
            InterpolationMethod::Next => {
                // Find first point after timestamp
                let next = self.series.points.iter()
                    .find(|p| p.timestamp >= timestamp);
                Ok(next.map(|p| p.value).unwrap_or(0.0))
            }
            InterpolationMethod::Linear => {
                // Linear interpolation between neighbors
                let ts_secs = timestamp.duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0)).as_secs() as f64;

                let prev = self.series.points.iter()
                    .filter(|p| p.timestamp <= timestamp)
                    .last();
                let next = self.series.points.iter()
                    .find(|p| p.timestamp >= timestamp);

                match (prev, next) {
                    (Some(p), Some(n)) => {
                        let p_secs = p.timestamp.duration_since(UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0)).as_secs() as f64;
                        let n_secs = n.timestamp.duration_since(UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0)).as_secs() as f64;

                        let ratio = (ts_secs - p_secs) / (n_secs - p_secs);
                        Ok(p.value + ratio * (n.value - p.value))
                    }
                    (Some(p), None) => Ok(p.value),
                    (None, Some(n)) => Ok(n.value),
                    (None, None) => Ok(0.0),
                }
            }
            InterpolationMethod::Average => {
                let prev = self.series.points.iter()
                    .filter(|p| p.timestamp <= timestamp)
                    .last();
                let next = self.series.points.iter()
                    .find(|p| p.timestamp >= timestamp);

                match (prev, next) {
                    (Some(p), Some(n)) => Ok((p.value + n.value) / 2.0),
                    (Some(p), None) => Ok(p.value),
                    (None, Some(n)) => Ok(n.value),
                    (None, None) => Ok(0.0),
                }
            }
            InterpolationMethod::Constant(val) => Ok(*val),
            InterpolationMethod::Null => Ok(f64::NAN),
            InterpolationMethod::Spline => {
                // Simplified cubic spline - in production would use proper implementation
                self.interpolate_value(timestamp, &InterpolationMethod::Linear)
            }
        }
    }

    /// Downsample time series into buckets
    pub fn downsample(
        &self,
        bucket: TimeBucket,
        aggregation: AggregationMethod,
    ) -> Result<TimeSeries> {
        let mut buckets: BTreeMap<u64, Vec<f64>> = BTreeMap::new();

        // Group points into buckets
        for point in &self.series.points {
            let bucket_key = self.get_bucket_key(point.timestamp, &bucket)?;
            buckets.entry(bucket_key)
                .or_insert_with(Vec::new)
                .push(point.value);
        }

        // Aggregate each bucket
        let mut downsampled_points = Vec::new();
        for (bucket_key, values) in buckets {
            let timestamp = UNIX_EPOCH + Duration::from_secs(bucket_key);
            let _value = self.aggregate_values(&values, &aggregation)?;

            downsampled_points.push(TimeSeriesPoint {
                timestamp,
                value,
                metadata: HashMap::new(),
            });
        }

        let metadata = self.calculate_metadata(&downsampled_points);
        Ok(TimeSeries {
            name: format!("{}_downsampled", self.series.name),
            points: downsampled_points,
            interval: Some(bucket.duration),
            metadata,
        })
    }

    /// Get bucket key for timestamp
    fn get_bucket_key(&self, timestamp: SystemTime, bucket: &TimeBucket) -> Result<u64> {
        let secs = timestamp.duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0)).as_secs();
        let bucket_secs = bucket.duration.as_secs();

        let bucket_key = match bucket.alignment {
            BucketAlignment::Epoch => (secs / bucket_secs) * bucket_secs,
            BucketAlignment::Calendar => {
                // Simplified - would align to calendar boundaries in production
                (secs / bucket_secs) * bucket_secs
            }
            BucketAlignment::FirstPoint => {
                let start_secs = self.series.points[0].timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0)).as_secs();
                let offset = secs - start_secs;
                start_secs + (offset / bucket_secs) * bucket_secs
            }
        };

        Ok(bucket_key)
    }

    /// Aggregate values
    fn aggregate_values(&self, values: &[f64], method: &AggregationMethod) -> Result<f64> {
        if values.is_empty() {
            return Ok(0.0);
        }

        let _result = match method {
            AggregationMethod::Mean => {
                values.iter().sum::<f64>() / values.len() as f64
            }
            AggregationMethod::Sum => values.iter().sum(),
            AggregationMethod::Min => values.iter()
                .cloned()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            AggregationMethod::Max => values.iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0),
            AggregationMethod::First => values[0],
            AggregationMethod::Last => values[values.len() - 1],
            AggregationMethod::Count => values.len() as f64,
            AggregationMethod::StdDev => {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance = values.iter()
                    .map(|v| (v - mean).powi(2))
                    .sum::<f64>() / values.len() as f64;
                variance.sqrt()
            }
            AggregationMethod::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[mid]
                }
            }
        };

        Ok(result)
    }

    /// Calculate moving average
    pub fn moving_average(&self, ma_type: MovingAverageType) -> Result<TimeSeries> {
        let mut ma_points = Vec::new();

        match ma_type {
            MovingAverageType::Simple { window_size } => {
                for _i in 0..self.series.points.len() {
                    let start = if i >= window_size { i - window_size + 1 } else { 0 };
                    let window = &self.series.points[start..=i];
                    let avg = window.iter().map(|p| p.value).sum::<f64>() / window.len() as f64;

                    ma_points.push(TimeSeriesPoint {
                        timestamp: self.series.points[i].timestamp,
                        value: avg,
                        metadata: HashMap::new(),
                    });
                }
            }
            MovingAverageType::Exponential { alpha } => {
                if alpha <= 0.0 || alpha > 1.0 {
                    return Err(DbError::InvalidInput(
                        "Alpha must be in (0, 1]".to_string()
                    ));
                }

                let mut ema = self.series.points[0].value;
                for point in &self.series.points {
                    ema = alpha * point.value + (1.0 - alpha) * ema;
                    ma_points.push(TimeSeriesPoint {
                        timestamp: point.timestamp,
                        value: ema,
                        metadata: HashMap::new(),
                    });
                }
            }
            MovingAverageType::Weighted { weights } => {
                for _i in 0..self.series.points.len() {
                    let start = if i >= weights.len() { i - weights.len() + 1 } else { 0 };
                    let window = &self.series.points[start..=i];

                    let weighted_sum: f64 = window.iter()
                        .zip(weights.iter())
                        .map(|(p, w)| p.value * w)
                        .sum();
                    let weight_sum: f64 = weights.iter().sum();
                    let wma = weighted_sum / weight_sum;

                    ma_points.push(TimeSeriesPoint {
                        timestamp: self.series.points[i].timestamp,
                        value: wma,
                        metadata: HashMap::new(),
                    });
                }
            }
            MovingAverageType::Cumulative => {
                let mut sum = 0.0;
                for (i, point) in self.series.points.iter().enumerate() {
                    sum += point.value;
                    let cma = sum / (i + 1) as f64;

                    ma_points.push(TimeSeriesPoint {
                        timestamp: point.timestamp,
                        value: cma,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        let metadata = self.calculate_metadata(&ma_points);

        Ok(TimeSeries {
            name: format!("{}_ma", self.series.name),
            points: ma_points,
            interval: self.series.interval,
            metadata,
        })
    }

    /// Detect seasonality in time series
    pub fn detect_seasonality(&self) -> Result<SeasonalityInfo> {
        if self.series.points.len() < 4 {
            return Ok(SeasonalityInfo {
                has_seasonality: false,
                period: None,
                strength: 0.0,
                patterns: Vec::new(),
            });
        }

        // Simple autocorrelation-based detection
        let values: Vec<f64> = self.series.points.iter().map(|p| p.value).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;

        // Try different periods
        let max_period = values.len() / 2;
        let mut best_period = 0;
        let mut best_correlation = 0.0;

        for period in 2..=max_period {
            let correlation = self.autocorrelation(&values, period, mean);
            if correlation > best_correlation {
                best_correlation = correlation;
                best_period = period;
            }
        }

        let has_seasonality = best_correlation > 0.5;

        Ok(SeasonalityInfo {
            has_seasonality,
            period: if has_seasonality { Some(best_period) } else { None },
            strength: best_correlation,
            patterns: Vec::new(),
        })
    }

    /// Calculate autocorrelation at lag
    fn autocorrelation(&self, values: &[f64], lag: usize, mean: f64) -> f64 {
        if lag >= values.len() {
            return 0.0;
        }

        let n = values.len() - lag;
        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for _i in 0..n {
            numerator += (values[i] - mean) * (values[i + lag] - mean);
        }

        for value in values {
            denominator += (value - mean).powi(2);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Decompose time series into trend, seasonal, and residual components
    pub fn decompose(&self) -> Result<TimeSeriesDecomposition> {
        let values: Vec<f64> = self.series.points.iter().map(|p| p.value).collect();

        // Calculate trend using moving average
        let window_size = (values.len() / 4).max(3);
        let trend = self.calculate_trend(&values, window_size);

        // Detrend to get seasonal + residual
        let detrended: Vec<f64> = values.iter()
            .zip(trend.iter())
            .map(|(v, t)| v - t)
            .collect();

        // Extract seasonal component (simplified)
        let seasonal_info = self.detect_seasonality()?;
        let seasonal = if let Some(period) = seasonal_info.period {
            self.calculate_seasonal(&detrended, period)
        } else {
            vec![0.0; values.len()]
        };

        // Residual = original - trend - seasonal
        let residual: Vec<f64> = values.iter()
            .zip(trend.iter())
            .zip(seasonal.iter())
            .map(|((v, t), s)| v - t - s)
            .collect();

        Ok(TimeSeriesDecomposition {
            trend,
            seasonal,
            residual,
        })
    }

    fn calculate_trend(&self, values: &[f64], window_size: usize) -> Vec<f64> {
        let mut trend = Vec::new();

        for _i in 0..values.len() {
            let start = if i >= window_size / 2 { i - window_size / 2 } else { 0 };
            let end = (i + window_size / 2).min(values.len());
            let window = &values[start..end];
            let avg = window.iter().sum::<f64>() / window.len() as f64;
            trend.push(avg);
        }

        trend
    }

    fn calculate_seasonal(&self, detrended: &[f64], period: usize) -> Vec<f64> {
        let mut seasonal = vec![0.0; detrended.len()];
        let mut period_averages = vec![0.0; period];

        // Calculate average for each position in period
        for _i in 0..period {
            let mut sum = 0.0;
            let mut count = 0;

            for j in (i..detrended.len()).step_by(period) {
                sum += detrended[j];
                count += 1;
            }

            period_averages[i] = if count > 0 { sum / count as f64 } else { 0.0 };
        }

        // Apply seasonal pattern
        for _i in 0..detrended.len() {
            seasonal[i] = period_averages[i % period];
        }

        seasonal
    }

    /// Detect change points in time series
    pub fn detect_change_points(&self, threshold: f64) -> Result<Vec<usize>> {
        let values: Vec<f64> = self.series.points.iter().map(|p| p.value).collect();
        let mut change_points = Vec::new();

        if values.len() < 3 {
            return Ok(change_points);
        }

        // Simple threshold-based detection
        for _i in 1..values.len() - 1 {
            let change = (values[i] - values[i - 1]).abs();
            if change > threshold {
                change_points.push(i);
            }
        }

        Ok(change_points)
    }

    fn calculate_metadata(&self, points: &[TimeSeriesPoint]) -> TimeSeriesMetadata {
        if points.is_empty() {
            return TimeSeriesMetadata {
                start_time: SystemTime::now(),
                end_time: SystemTime::now(),
                point_count: 0,
                has_gaps: false,
                is_regular: true,
                detected_interval: None,
            };
        }

        TimeSeriesMetadata {
            start_time: points[0].timestamp,
            end_time: points[points.len() - 1].timestamp,
            point_count: points.len(),
            has_gaps: false,
            is_regular: true,
            detected_interval: self.detect_interval(points),
        }
    }

    fn detect_interval(&self, points: &[TimeSeriesPoint]) -> Option<Duration> {
        if points.len() < 2 {
            return None;
        }

        // Calculate intervals between consecutive points
        let intervals: Vec<Duration> = points.windows(2)
            .filter_map(|w| w[1].timestamp.duration_since(w[0].timestamp).ok())
            .collect();

        if intervals.is_empty() {
            return None;
        }

        // Find most common interval (simplified - would use mode in production)
        let avg_interval_secs = intervals.iter()
            .map(|d| d.as_secs())
            .sum::<u64>() / intervals.len() as u64;

        Some(Duration::from_secs(avg_interval_secs))
    }
}

/// Seasonality information
#[derive(Debug, Clone)]
pub struct SeasonalityInfo {
    pub has_seasonality: bool,
    pub period: Option<usize>,
    pub strength: f64,
    pub patterns: Vec<SeasonalPattern>,
}

#[derive(Debug, Clone)]
pub struct SeasonalPattern {
    pub period: usize,
    pub amplitude: f64,
    pub phase: f64,
}

/// Time-series decomposition
#[derive(Debug, Clone)]
pub struct TimeSeriesDecomposition {
    pub trend: Vec<f64>,
    pub seasonal: Vec<f64>,
    pub residual: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_series() -> TimeSeries {
        let mut points = Vec::new();
        let start = UNIX_EPOCH + Duration::from_secs(1000000);

        for _i in 0..100 {
            points.push(TimeSeriesPoint {
                timestamp: start + Duration::from_secs(i * 3600),
                value: (i as f64).sin() * 10.0 + 50.0,
                metadata: HashMap::new(),
            });
        }

        TimeSeries {
            name: "test_series".to_string(),
            points,
            interval: Some(Duration::from_secs(3600)),
            metadata: TimeSeriesMetadata {
                start_time: start,
                end_time: start + Duration::from_secs(99 * 3600),
                point_count: 100,
                has_gaps: false,
                is_regular: true,
                detected_interval: Some(Duration::from_secs(3600)),
            },
        }
    }

    #[test]
    fn test_moving_average() {
        let series = create_test_series();
        let analyzer = TimeSeriesAnalyzer::new(series);

        let ma = analyzer.moving_average(MovingAverageType::Simple { window_size: 5 })
            .unwrap();

        assert_eq!(ma.points.len(), 100);
    }

    #[test]
    fn test_downsample() {
        let series = create_test_series();
        let analyzer = TimeSeriesAnalyzer::new(series);

        let bucket = TimeBucket {
            duration: Duration::from_secs(7200), // 2 hours
            alignment: BucketAlignment::Epoch,
        };

        let downsampled = analyzer.downsample(bucket, AggregationMethod::Mean).unwrap();

        assert!(downsampled.points.len() < 100);
    }

    #[test]
    fn test_interpolation() {
        let series = create_test_series();
        let analyzer = TimeSeriesAnalyzer::new(series);

        let filled = analyzer.fill_gaps(
            Duration::from_secs(3600),
            InterpolationMethod::Linear,
        ).unwrap();

        assert_eq!(filled.points.len(), 100);
    }
}


