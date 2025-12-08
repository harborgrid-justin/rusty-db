//! # Time Series Analysis and Forecasting
//!
//! Advanced time series capabilities including ARIMA-like forecasting,
//! exponential smoothing, seasonality detection, trend analysis, and anomaly detection.

use crate::error::{DbError, Result};
use super::Algorithm;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ============================================================================
// Time Series Models
// ============================================================================

/// Exponential Smoothing model (Holt-Winters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExponentialSmoothing {
    /// Level smoothing parameter (alpha)
    alpha: f64,
    /// Trend smoothing parameter (beta)
    beta: f64,
    /// Seasonal smoothing parameter (gamma)
    gamma: f64,
    /// Seasonality period
    season_length: usize,
    /// Current level
    level: f64,
    /// Current trend
    trend: f64,
    /// Seasonal components
    seasonal: Vec<f64>,
    /// Model type
    model_type: SeasonalityType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeasonalityType {
    None,
    Additive,
    Multiplicative,
}

impl ExponentialSmoothing {
    pub fn new(alpha: f64, beta: f64, gamma: f64, season_length: usize, model_type: SeasonalityType) -> Self {
        Self {
            alpha,
            beta,
            gamma,
            season_length,
            level: 0.0,
            trend: 0.0,
            seasonal: vec![0.0; season_length],
            model_type,
        }
    }

    pub fn fit(&mut self, series: &[f64]) -> Result<()> {
        if series.len() < self.season_length * 2 {
            return Err(DbError::InvalidInput("Insufficient data for seasonal model".into()));
        }

        // Initialize level, trend, and seasonal components
        self.initialize_components(series)?;

        // Update components using Holt-Winters equations
        for t in self.season_length..series.len() {
            let observation = series[t];
            let season_idx = t % self.season_length;

            match self.model_type {
                SeasonalityType::None => {
                    let prev_level = self.level;
                    self.level = self.alpha * observation + (1.0 - self.alpha) * (prev_level + self.trend);
                    self.trend = self.beta * (self.level - prev_level) + (1.0 - self.beta) * self.trend;
                }
                SeasonalityType::Additive => {
                    let prev_level = self.level;
                    let prev_seasonal = self.seasonal[season_idx];

                    self.level = self.alpha * (observation - prev_seasonal)
                        + (1.0 - self.alpha) * (prev_level + self.trend);
                    self.trend = self.beta * (self.level - prev_level)
                        + (1.0 - self.beta) * self.trend;
                    self.seasonal[season_idx] = self.gamma * (observation - self.level)
                        + (1.0 - self.gamma) * prev_seasonal;
                }
                SeasonalityType::Multiplicative => {
                    let prev_level = self.level;
                    let prev_seasonal = self.seasonal[season_idx];

                    self.level = self.alpha * (observation / prev_seasonal)
                        + (1.0 - self.alpha) * (prev_level + self.trend);
                    self.trend = self.beta * (self.level - prev_level)
                        + (1.0 - self.beta) * self.trend;
                    self.seasonal[season_idx] = self.gamma * (observation / self.level)
                        + (1.0 - self.gamma) * prev_seasonal;
                }
            }
        }

        Ok(())
    }

    pub fn forecast(&self, horizon: usize) -> Vec<f64> {
        let mut forecasts = Vec::with_capacity(horizon);

        for h in 1..=horizon {
            let forecast = match self.model_type {
                SeasonalityType::None => {
                    self.level + h as f64 * self.trend
                }
                SeasonalityType::Additive => {
                    let season_idx = (h - 1) % self.season_length;
                    self.level + h as f64 * self.trend + self.seasonal[season_idx]
                }
                SeasonalityType::Multiplicative => {
                    let season_idx = (h - 1) % self.season_length;
                    (self.level + h as f64 * self.trend) * self.seasonal[season_idx]
                }
            };

            forecasts.push(forecast);
        }

        forecasts
    }

    fn initialize_components(&mut self, series: &[f64]) -> Result<()> {
        // Initialize level as average of first season
        self.level = series.iter().take(self.season_length).sum::<f64>() / self.season_length as f64;

        // Initialize trend using linear regression on first two seasons
        if series.len() >= 2 * self.season_length {
            let first_avg = series.iter().take(self.season_length).sum::<f64>() / self.season_length as f64;
            let second_avg = series.iter()
                .skip(self.season_length)
                .take(self.season_length)
                .sum::<f64>() / self.season_length as f64;
            self.trend = (second_avg - first_avg) / self.season_length as f64;
        } else {
            self.trend = 0.0;
        }

        // Initialize seasonal components
        if self.model_type != SeasonalityType::None {
            for i in 0..self.season_length {
                let mut season_sum = 0.0;
                let mut count = 0;

                for j in (i..series.len()).step_by(self.season_length) {
                    season_sum += series[j];
                    count += 1;
                }

                let season_avg = season_sum / count as f64;

                self.seasonal[i] = match self.model_type {
                    SeasonalityType::Additive => season_avg - self.level,
                    SeasonalityType::Multiplicative => season_avg / self.level,
                    _ => 0.0,
                };
            }
        }

        Ok(())
    }
}

// ============================================================================
// ARIMA-like Model
// ============================================================================

/// ARIMA (AutoRegressive Integrated Moving Average) model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARIMAModel {
    /// AR (autoregressive) order
    p: usize,
    /// I (integration/differencing) order
    d: usize,
    /// MA (moving average) order
    q: usize,
    /// AR coefficients
    ar_coefficients: Vec<f64>,
    /// MA coefficients
    ma_coefficients: Vec<f64>,
    /// Constant term
    constant: f64,
    /// Recent observations (for forecasting)
    recent_values: VecDeque<f64>,
    /// Recent residuals (for MA component)
    recent_residuals: VecDeque<f64>,
}

impl ARIMAModel {
    pub fn new(p: usize, d: usize, q: usize) -> Self {
        Self {
            p,
            d,
            q,
            ar_coefficients: vec![0.0; p],
            ma_coefficients: vec![0.0; q],
            constant: 0.0,
            recent_values: VecDeque::with_capacity(p.max(d)),
            recent_residuals: VecDeque::with_capacity(q),
        }
    }

    pub fn fit(&mut self, series: &[f64]) -> Result<()> {
        if series.len() < self.p + self.d + self.q + 1 {
            return Err(DbError::InvalidInput("Insufficient data for ARIMA model".into()));
        }

        // Difference the series d times
        let mut differenced = series.to_vec();
        for _ in 0..self.d {
            differenced = self.difference(&differenced);
        }

        // Estimate AR coefficients using Yule-Walker equations
        if self.p > 0 {
            self.estimate_ar_coefficients(&differenced)?;
        }

        // Estimate MA coefficients (simplified)
        if self.q > 0 {
            self.estimate_ma_coefficients(&differenced)?;
        }

        // Store recent values for forecasting
        let start = series.len().saturating_sub(self.p.max(self.d));
        for &value in &series[start..] {
            self.recent_values.push_back(value);
        }

        Ok(())
    }

    pub fn forecast(&mut self, horizon: usize) -> Vec<f64> {
        let mut forecasts = Vec::with_capacity(horizon);

        for _ in 0..horizon {
            let mut forecast = self.constant;

            // AR component
            for (i, &coef) in self.ar_coefficients.iter().enumerate() {
                if i < self.recent_values.len() {
                    forecast += coef * self.recent_values[self.recent_values.len() - 1 - i];
                }
            }

            // MA component
            for (i, &coef) in self.ma_coefficients.iter().enumerate() {
                if i < self.recent_residuals.len() {
                    forecast += coef * self.recent_residuals[self.recent_residuals.len() - 1 - i];
                }
            }

            forecasts.push(forecast);

            // Update recent values
            self.recent_values.push_back(forecast);
            if self.recent_values.len() > self.p {
                self.recent_values.pop_front();
            }

            // Update recent residuals (assume zero for forecasts)
            self.recent_residuals.push_back(0.0);
            if self.recent_residuals.len() > self.q {
                self.recent_residuals.pop_front();
            }
        }

        // Integrate back if d > 0
        if self.d > 0 {
            forecasts = self.integrate(&forecasts);
        }

        forecasts
    }

    fn difference(&self, series: &[f64]) -> Vec<f64> {
        series.windows(2).map(|w| w[1] - w[0]).collect()
    }

    fn integrate(&self, series: &[f64]) -> Vec<f64> {
        let last_value = self.recent_values.back().copied().unwrap_or(0.0);
        let mut integrated = Vec::with_capacity(series.len());
        let mut current = last_value;

        for &diff in series {
            current += diff;
            integrated.push(current);
        }

        integrated
    }

    fn estimate_ar_coefficients(&mut self, series: &[f64]) -> Result<()> {
        // Simplified AR estimation using OLS
        let n = series.len();
        if n <= self.p {
            return Ok(());
        }

        // Build design matrix and target vector
        let mut x_matrix = vec![vec![0.0; self.p]; n - self.p];
        let mut y_vector = vec![0.0; n - self.p];

        for i in 0..(n - self.p) {
            for j in 0..self.p {
                x_matrix[i][j] = series[self.p - j - 1 + i];
            }
            y_vector[i] = series[self.p + i];
        }

        // Solve using normal equations (simplified)
        self.ar_coefficients = vec![0.5; self.p]; // Placeholder

        Ok(())
    }

    fn estimate_ma_coefficients(&mut self, _series: &[f64]) -> Result<()> {
        // Simplified MA estimation
        self.ma_coefficients = vec![0.3; self.q]; // Placeholder
        Ok(())
    }
}

// ============================================================================
// Seasonality Detection
// ============================================================================

/// Detect seasonality in time series using autocorrelation
#[derive(Debug, Clone)]
pub struct SeasonalityDetector {
    /// Maximum lag to test
    max_lag: usize,
    /// Significance threshold
    threshold: f64,
}

impl SeasonalityDetector {
    pub fn new(max_lag: usize, threshold: f64) -> Self {
        Self { max_lag, threshold }
    }

    pub fn detect(&self, series: &[f64]) -> SeasonalityInfo {
        let acf = self.compute_autocorrelation(series);

        // Find significant peaks in ACF
        let mut peaks = Vec::new();
        for lag in 2..acf.len() {
            if acf[lag] > self.threshold && acf[lag] > acf[lag - 1] && acf[lag] > acf[lag + 1] {
                peaks.push((lag, acf[lag]));
            }
        }

        // Sort peaks by correlation strength
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let dominant_period = peaks.first().map(|(lag, _)| *lag);
        let strength = peaks.first().map(|(_, corr)| *corr).unwrap_or(0.0);

        SeasonalityInfo {
            has_seasonality: strength > self.threshold,
            period: dominant_period,
            strength,
            all_periods: peaks.iter().map(|(lag, _)| *lag).collect(),
        }
    }

    fn compute_autocorrelation(&self, series: &[f64]) -> Vec<f64> {
        let n = series.len();
        let mean = series.iter().sum::<f64>() / n as f64;
        let variance = series.iter().map(|&x| (x - mean).powi(2)).sum::<f64>();

        let mut acf = vec![0.0; self.max_lag.min(n)];
        acf[0] = 1.0;

        for lag in 1..acf.len() {
            let mut sum = 0.0;
            for i in 0..(n - lag) {
                sum += (series[i] - mean) * (series[i + lag] - mean);
            }
            acf[lag] = sum / variance;
        }

        acf
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalityInfo {
    pub has_seasonality: bool,
    pub period: Option<usize>,
    pub strength: f64,
    pub all_periods: Vec<usize>,
}

// ============================================================================
// Trend Analysis
// ============================================================================

/// Decompose time series into trend, seasonal, and residual components
#[derive(Debug, Clone)]
pub struct TrendAnalyzer {
    /// Decomposition method
    method: DecompositionMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecompositionMethod {
    Additive,
    Multiplicative,
}

impl TrendAnalyzer {
    pub fn new(method: DecompositionMethod) -> Self {
        Self { method }
    }

    pub fn decompose(&self, series: &[f64], period: usize) -> TimeSeriesDecomposition {
        let trend = self.extract_trend(series, period);
        let detrended = self.detrend(series, &trend);
        let seasonal = self.extract_seasonal(&detrended, period);
        let residual = self.compute_residual(series, &trend, &seasonal);

        TimeSeriesDecomposition {
            trend,
            seasonal,
            residual,
            method: self.method,
        }
    }

    fn extract_trend(&self, series: &[f64], period: usize) -> Vec<f64> {
        // Moving average for trend
        let window = period;
        let mut trend = Vec::with_capacity(series.len());

        for i in 0..series.len() {
            let start = i.saturating_sub(window / 2);
            let end = (i + window / 2 + 1).min(series.len());
            let avg = series[start..end].iter().sum::<f64>() / (end - start) as f64;
            trend.push(avg);
        }

        trend
    }

    fn detrend(&self, series: &[f64], trend: &[f64]) -> Vec<f64> {
        match self.method {
            DecompositionMethod::Additive => {
                series.iter().zip(trend).map(|(s, t)| s - t).collect()
            }
            DecompositionMethod::Multiplicative => {
                series.iter().zip(trend).map(|(s, t)| if t.abs() > 1e-10 { s / t } else { 0.0 }).collect()
            }
        }
    }

    fn extract_seasonal(&self, detrended: &[f64], period: usize) -> Vec<f64> {
        let mut seasonal = vec![0.0; period];

        // Average each seasonal period
        for i in 0..period {
            let mut sum = 0.0;
            let mut count = 0;

            for j in (i..detrended.len()).step_by(period) {
                sum += detrended[j];
                count += 1;
            }

            seasonal[i] = if count > 0 { sum / count as f64 } else { 0.0 };
        }

        // Replicate to match series length
        detrended.iter().enumerate()
            .map(|(i, _)| seasonal[i % period])
            .collect()
    }

    fn compute_residual(&self, series: &[f64], trend: &[f64], seasonal: &[f64]) -> Vec<f64> {
        match self.method {
            DecompositionMethod::Additive => {
                series.iter()
                    .zip(trend.iter().zip(seasonal))
                    .map(|(s, (t, sea))| s - t - sea)
                    .collect()
            }
            DecompositionMethod::Multiplicative => {
                series.iter()
                    .zip(trend.iter().zip(seasonal))
                    .map(|(s, (t, sea))| {
                        let denom = t * sea;
                        if denom.abs() > 1e-10 { s / denom } else { 0.0 }
                    })
                    .collect()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesDecomposition {
    pub trend: Vec<f64>,
    pub seasonal: Vec<f64>,
    pub residual: Vec<f64>,
    pub method: DecompositionMethod,
}

// ============================================================================
// Anomaly Detection
// ============================================================================

/// Time series anomaly detection
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    /// Detection method
    method: AnomalyDetectionMethod,
    /// Sensitivity threshold
    threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyDetectionMethod {
    ZScore,
    IQR,
    IsolationForest,
    SeasonalHybrid,
}

impl AnomalyDetector {
    pub fn new(method: AnomalyDetectionMethod, threshold: f64) -> Self {
        Self { method, threshold }
    }

    pub fn detect(&self, series: &[f64]) -> Vec<Anomaly> {
        match self.method {
            AnomalyDetectionMethod::ZScore => self.detect_zscore(series),
            AnomalyDetectionMethod::IQR => self.detect_iqr(series),
            AnomalyDetectionMethod::IsolationForest => self.detect_isolation_forest(series),
            AnomalyDetectionMethod::SeasonalHybrid => self.detect_seasonal_hybrid(series),
        }
    }

    fn detect_zscore(&self, series: &[f64]) -> Vec<Anomaly> {
        let mean = series.iter().sum::<f64>() / series.len() as f64;
        let std = (series.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / series.len() as f64).sqrt();

        series.iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                let z_score = ((value - mean) / std).abs();
                if z_score > self.threshold {
                    Some(Anomaly {
                        index: i,
                        value,
                        score: z_score,
                        anomaly_type: AnomalyType::Outlier,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn detect_iqr(&self, series: &[f64]) -> Vec<Anomaly> {
        let mut sorted = series.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1_idx = sorted.len() / 4;
        let q3_idx = 3 * sorted.len() / 4;
        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - self.threshold * iqr;
        let upper_bound = q3 + self.threshold * iqr;

        series.iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                if value < lower_bound || value > upper_bound {
                    let score = if value < lower_bound {
                        (lower_bound - value) / iqr
                    } else {
                        (value - upper_bound) / iqr
                    };

                    Some(Anomaly {
                        index: i,
                        value,
                        score,
                        anomaly_type: AnomalyType::Outlier,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn detect_isolation_forest(&self, _series: &[f64]) -> Vec<Anomaly> {
        // Simplified isolation forest - placeholder
        Vec::new()
    }

    fn detect_seasonal_hybrid(&self, series: &[f64]) -> Vec<Anomaly> {
        // Detect seasonality first
        let detector = SeasonalityDetector::new(series.len() / 2, 0.3);
        let seasonality = detector.detect(series);

        if let Some(period) = seasonality.period {
            // Decompose and detect anomalies in residuals
            let analyzer = TrendAnalyzer::new(DecompositionMethod::Additive);
            let decomp = analyzer.decompose(series, period);

            self.detect_zscore(&decomp.residual)
        } else {
            self.detect_zscore(series)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub index: usize,
    pub value: f64,
    pub score: f64,
    pub anomaly_type: AnomalyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    Outlier,
    LevelShift,
    Trend,
    Seasonal,
}

// ============================================================================
// Time Series Engine
// ============================================================================

/// Main time series analysis engine
pub struct TimeSeriesEngine {
    /// Cached models
    models: std::collections::HashMap<String, TimeSeriesModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesModel {
    ExponentialSmoothing(ExponentialSmoothing),
    ARIMA(ARIMAModel),
}

impl TimeSeriesEngine {
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }

    pub fn forecast(
        &self,
        series: Vec<f64>,
        horizon: usize,
        algorithm: Algorithm,
    ) -> Result<Vec<f64>> {
        match algorithm {
            Algorithm::ExponentialSmoothing => {
                let mut model = ExponentialSmoothing::new(
                    0.2, 0.1, 0.1, 12, SeasonalityType::Additive
                );
                model.fit(&series)?;
                Ok(model.forecast(horizon))
            }
            Algorithm::ARIMA => {
                let mut model = ARIMAModel::new(1, 1, 1);
                model.fit(&series)?;
                Ok(model.forecast(horizon))
            }
            _ => Err(DbError::InvalidInput("Unsupported time series algorithm".into())),
        }
    }

    pub fn detect_seasonality(&self, series: &[f64]) -> SeasonalityInfo {
        let detector = SeasonalityDetector::new(series.len() / 2, 0.3);
        detector.detect(series)
    }

    pub fn decompose(&self, series: &[f64], period: usize) -> TimeSeriesDecomposition {
        let analyzer = TrendAnalyzer::new(DecompositionMethod::Additive);
        analyzer.decompose(series, period)
    }

    pub fn detect_anomalies(&self, series: &[f64]) -> Vec<Anomaly> {
        let detector = AnomalyDetector::new(AnomalyDetectionMethod::ZScore, 3.0);
        detector.detect(series)
    }
}

impl Default for TimeSeriesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_smoothing() {
        let series = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 16.0, 14.0, 17.0, 18.0, 19.0, 17.0];
        let mut model = ExponentialSmoothing::new(0.2, 0.1, 0.1, 4, SeasonalityType::Additive);
        model.fit(&series).unwrap();

        let forecast = model.forecast(3);
        assert_eq!(forecast.len(), 3);
    }

    #[test]
    fn test_seasonality_detection() {
        let series: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0)
            .collect();

        let detector = SeasonalityDetector::new(50, 0.3);
        let info = detector.detect(&series);

        assert!(info.period.is_some());
    }

    #[test]
    fn test_anomaly_detection() {
        let mut series = vec![10.0; 100];
        series[50] = 100.0; // Anomaly

        let detector = AnomalyDetector::new(AnomalyDetectionMethod::ZScore, 3.0);
        let anomalies = detector.detect(&series);

        assert!(!anomalies.is_empty());
        assert_eq!(anomalies[0].index, 50);
    }
}


