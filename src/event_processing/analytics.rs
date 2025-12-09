// Stream Analytics
//
// Implements real-time analytics on event streams including anomaly detection,
// trend analysis, predictive analytics, and alert generation with ML model serving.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use super::{Event, EventValue};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};

/// Analytics metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub timestamp: SystemTime,
    pub tags: HashMap<String, String>,
}

impl Metric {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            timestamp: SystemTime::now(),
            tags: HashMap::new(),
        }
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Dashboard data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardDataPoint {
    pub metric: String,
    pub value: f64,
    pub timestamp: SystemTime,
    pub dimensions: HashMap<String, String>,
}

/// Real-time dashboard
pub struct Dashboard {
    name: String,
    metrics: RwLock<HashMap<String, VecDeque<DashboardDataPoint>>>,
    retention_duration: Duration,
    max_points: usize,
}

impl Dashboard {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metrics: RwLock::new(HashMap::new()),
            retention_duration: Duration::from_secs(3600), // 1 hour
            max_points: 10000,
        }
    }

    pub fn with_retention(mut self, duration: Duration) -> Self {
        self.retention_duration = duration;
        self
    }

    pub fn add_data_point(&self, point: DashboardDataPoint) {
        let mut metrics = self.metrics.write().unwrap();
        let points = metrics
            .entry(point.metric.clone())
            .or_insert_with(VecDeque::new);

        points.push_back(point);

        // Enforce max points
        while points.len() > self.max_points {
            points.pop_front();
        }

        // Clean old data
        let cutoff = SystemTime::now() - self.retention_duration;
        while let Some(front) = points.front() {
            if front.timestamp < cutoff {
                points.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_metric_data(&self, metric: &str) -> Vec<DashboardDataPoint> {
        let metrics = self.metrics.read().unwrap();
        metrics
            .get(metric)
            .map(|points| points.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_latest(&self, metric: &str) -> Option<DashboardDataPoint> {
        let metrics = self.metrics.read().unwrap();
        metrics.get(metric)?.back().cloned()
    }

    pub fn get_all_metrics(&self) -> Vec<String> {
        let metrics = self.metrics.read().unwrap();
        metrics.keys().cloned().collect()
    }
}

/// Anomaly detection algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyDetectionAlgorithm {
    /// Z-score based detection
    ZScore { threshold: f64 },

    /// Interquartile range (IQR) method
    IQR { multiplier: f64 },

    /// Moving average deviation
    MovingAverageDeviation { window_size: usize, threshold: f64 },

    /// Exponential weighted moving average
    EWMA { alpha: f64, threshold: f64 },

    /// Machine learning model
    MLModel { model_name: String },
}

/// Anomaly detected event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub score: f64,
    pub timestamp: SystemTime,
    pub algorithm: String,
}

/// Anomaly detector
pub struct AnomalyDetector {
    algorithm: AnomalyDetectionAlgorithm,
    history: VecDeque<f64>,
    window_size: usize,
    stats: Statistics,
}

impl AnomalyDetector {
    pub fn new(algorithm: AnomalyDetectionAlgorithm) -> Self {
        let window_size = match &algorithm {
            AnomalyDetectionAlgorithm::MovingAverageDeviation { window_size, .. } => *window_size,
            _ => 100,
        };

        Self {
            algorithm,
            history: VecDeque::new(),
            window_size,
            stats: Statistics::default(),
        }
    }

    pub fn add_value(&mut self, value: f64) {
        self.history.push_back(value);

        while self.history.len() > self.window_size {
            self.history.pop_front();
        }

        self.stats.update(&self.history);
    }

    pub fn detect(&self, value: f64) -> Option<Anomaly> {
        if self.history.len() < 10 {
            return None; // Need more data
        }

        match &self.algorithm {
            AnomalyDetectionAlgorithm::ZScore { threshold } => {
                let z_score = (value - self.stats.mean) / self.stats.std_dev;

                if z_score.abs() > *threshold {
                    Some(Anomaly {
                        metric: "unknown".to_string(),
                        value,
                        expected_value: self.stats.mean,
                        deviation: value - self.stats.mean,
                        score: z_score.abs(),
                        timestamp: SystemTime::now(),
                        algorithm: "Z-Score".to_string(),
                    })
                } else {
                    None
                }
            }

            AnomalyDetectionAlgorithm::IQR { multiplier } => {
                let q1 = self.stats.quantile(0.25);
                let q3 = self.stats.quantile(0.75);
                let iqr = q3 - q1;
                let lower_bound = q1 - multiplier * iqr;
                let upper_bound = q3 + multiplier * iqr;

                if value < lower_bound || value > upper_bound {
                    Some(Anomaly {
                        metric: "unknown".to_string(),
                        value,
                        expected_value: self.stats.median,
                        deviation: value - self.stats.median,
                        score: ((value - self.stats.median).abs() / iqr).min(10.0),
                        timestamp: SystemTime::now(),
                        algorithm: "IQR".to_string(),
                    })
                } else {
                    None
                }
            }

            AnomalyDetectionAlgorithm::MovingAverageDeviation {
                window_size: _,
                threshold,
            } => {
                let deviation = (value - self.stats.mean).abs();

                if deviation > *threshold {
                    Some(Anomaly {
                        metric: "unknown".to_string(),
                        value,
                        expected_value: self.stats.mean,
                        deviation: value - self.stats.mean,
                        score: deviation / self.stats.std_dev,
                        timestamp: SystemTime::now(),
                        algorithm: "MovingAvgDeviation".to_string(),
                    })
                } else {
                    None
                }
            }

            AnomalyDetectionAlgorithm::EWMA { alpha: _, threshold } => {
                let deviation = (value - self.stats.mean).abs();

                if deviation > *threshold {
                    Some(Anomaly {
                        metric: "unknown".to_string(),
                        value,
                        expected_value: self.stats.mean,
                        deviation: value - self.stats.mean,
                        score: deviation,
                        timestamp: SystemTime::now(),
                        algorithm: "EWMA".to_string(),
                    })
                } else {
                    None
                }
            }

            AnomalyDetectionAlgorithm::MLModel { .. } => {
                // ML model inference would go here
                None
            }
        }
    }
}

/// Statistics calculator
#[derive(Debug, Clone, Default)]
struct Statistics {
    mean: f64,
    median: f64,
    std_dev: f64,
    min: f64,
    max: f64,
}

impl Statistics {
    fn update(&mut self, values: &VecDeque<f64>) {
        if values.is_empty() {
            return;
        }

        // Mean
        self.mean = values.iter().sum::<f64>() / values.len() as f64;

        // Median
        let mut sorted: Vec<f64> = values.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        self.median = if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        };

        // Standard deviation
        let variance = values
            .iter()
            .map(|v| (v - self.mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        self.std_dev = variance.sqrt();

        // Min/Max
        self.min = sorted[0];
        self.max = sorted[sorted.len() - 1];
    }

    fn quantile(&self, q: f64) -> f64 {
        // Simplified - in production use a proper quantile calculation
        self.mean + (q - 0.5) * self.std_dev * 2.0
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

/// Trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub metric: String,
    pub direction: TrendDirection,
    pub slope: f64,
    pub confidence: f64,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
}

/// Trend analyzer
pub struct TrendAnalyzer {
    window_size: usize,
    history: VecDeque<(SystemTime, f64)>,
    stability_threshold: f64,
}

impl TrendAnalyzer {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            history: VecDeque::new(),
            stability_threshold: 0.01,
        }
    }

    pub fn add_value(&mut self, value: f64, timestamp: SystemTime) {
        self.history.push_back((timestamp, value));

        while self.history.len() > self.window_size {
            self.history.pop_front();
        }
    }

    pub fn analyze(&self) -> Option<Trend> {
        if self.history.len() < 3 {
            return None;
        }

        // Simple linear regression
        let n = self.history.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        let start_time = self.history.front().unwrap().0;

        for (i, (timestamp, value)) in self.history.iter().enumerate() {
            let x = i;
            sum_x += x;
            sum_y += value;
            sum_xy += x * value;
            sum_x2 += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);

        let direction = if slope.abs() < self.stability_threshold {
            TrendDirection::Stable
        } else if slope > 0.0 {
            TrendDirection::Up
        } else {
            TrendDirection::Down
        };

        // Calculate R-squared for confidence
        let mean_y = sum_y / n;
        let mut ss_tot = 0.0;
        let mut ss_res = 0.0;

        for (i, (_, value)) in self.history.iter().enumerate() {
            let x = i;
            let predicted = (slope * x) + ((sum_y - slope * sum_x) / n);
            ss_res += (value - predicted).powi(2);
            ss_tot += (value - mean_y).powi(2);
        }

        let r_squared = if ss_tot > 0.0 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };

        Some(Trend {
            metric: "unknown".to_string(),
            direction,
            slope,
            confidence: r_squared,
            start_time,
            end_time: self.history.back().unwrap().0,
        })
    }
}

/// Predictive model type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictiveModel {
    /// Simple moving average
    SimpleMovingAverage { window: usize },

    /// Exponential smoothing
    ExponentialSmoothing { alpha: f64 },

    /// Linear regression
    LinearRegression,

    /// ARIMA model
    ARIMA { p: usize, d: usize, q: usize },

    /// Custom ML model
    CustomML { model_name: String },
}

/// Prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub metric: String,
    pub predicted_value: f64,
    pub confidence_interval: (f64, f64),
    pub forecast_time: SystemTime,
    pub model: String,
}

/// Predictive analytics engine
pub struct PredictiveAnalyzer {
    model: PredictiveModel,
    history: VecDeque<f64>,
}

impl PredictiveAnalyzer {
    pub fn new(model: PredictiveModel) -> Self {
        Self {
            model,
            history: VecDeque::new(),
        }
    }

    pub fn add_value(&mut self, value: f64) {
        self.history.push_back(value);

        let max_history = match &self.model {
            PredictiveModel::SimpleMovingAverage { window } => *window * 2,
            _ => 1000,
        };

        while self.history.len() > max_history {
            self.history.pop_front();
        }
    }

    pub fn predict(&self, steps_ahead: usize) -> Vec<Prediction> {
        match &self.model {
            PredictiveModel::SimpleMovingAverage { window } => {
                self.predict_sma(*window, steps_ahead)
            }

            PredictiveModel::ExponentialSmoothing { alpha } => {
                self.predict_exponential_smoothing(*alpha, steps_ahead)
            }

            PredictiveModel::LinearRegression => self.predict_linear_regression(steps_ahead),

            _ => vec![],
        }
    }

    fn predict_sma(&self, window: usize, steps_ahead: usize) -> Vec<Prediction> {
        if self.history.len() < window {
            return vec![];
        }

        let last_values: Vec<f64> = self.history.iter().rev().take(window).copied().collect();
        let predicted_value: f64 = last_values.iter().sum::<f64>() / window as f64;

        let std_dev = {
            let mean = predicted_value;
            let variance =
                last_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window as f64;
            variance.sqrt()
        };

        let mut predictions = Vec::new();
        for i in 0..steps_ahead {
            predictions.push(Prediction {
                metric: "unknown".to_string(),
                predicted_value,
                confidence_interval: (
                    predicted_value - 1.96 * std_dev,
                    predicted_value + 1.96 * std_dev,
                ),
                forecast_time: SystemTime::now() + Duration::from_secs((i + 1) as u64),
                model: "SMA".to_string(),
            });
        }

        predictions
    }

    fn predict_exponential_smoothing(&self, alpha: f64, steps_ahead: usize) -> Vec<Prediction> {
        if self.history.is_empty() {
            return vec![];
        }

        let mut smoothed = self.history[0];
        for &value in self.history.iter().skip(1) {
            smoothed = alpha * value + (1.0 - alpha) * smoothed;
        }

        let mut predictions = Vec::new();
        for i in 0..steps_ahead {
            predictions.push(Prediction {
                metric: "unknown".to_string(),
                predicted_value: smoothed,
                confidence_interval: (smoothed * 0.9, smoothed * 1.1),
                forecast_time: SystemTime::now() + Duration::from_secs((i + 1) as u64),
                model: "ExponentialSmoothing".to_string(),
            });
        }

        predictions
    }

    fn predict_linear_regression(&self, steps_ahead: usize) -> Vec<Prediction> {
        if self.history.len() < 3 {
            return vec![];
        }

        let n = self.history.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, &value) in self.history.iter().enumerate() {
            let x = i;
            sum_x += x;
            sum_y += value;
            sum_xy += x * value;
            sum_x2 += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let mut predictions = Vec::new();
        for i in 0..steps_ahead {
            let x = (self.history.len() + i) as f64;
            let predicted_value = slope * x + intercept;

            predictions.push(Prediction {
                metric: "unknown".to_string(),
                predicted_value,
                confidence_interval: (predicted_value * 0.95, predicted_value * 1.05),
                forecast_time: SystemTime::now() + Duration::from_secs((i + 1) as u64),
                model: "LinearRegression".to_string(),
            });
        }

        predictions
    }
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: SystemTime,
    pub tags: HashMap<String, String>,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan { threshold: f64 },
    LessThan { threshold: f64 },
    Equals { value: f64 },
    NotEquals { value: f64 },
    RateOfChange { threshold: f64, window: Duration },
    Anomaly { detector: AnomalyDetectionAlgorithm },
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub metric: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub cooldown: Duration,
}

/// Alert engine
pub struct AlertEngine {
    rules: Vec<AlertRule>,
    last_alert_times: HashMap<String, SystemTime>,
    anomaly_detectors: HashMap<String, AnomalyDetector>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            last_alert_times: HashMap::new(),
            anomaly_detectors: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: AlertRule) {
        if let AlertCondition::Anomaly { ref detector } = rule.condition {
            self.anomaly_detectors
                .insert(rule.name.clone(), AnomalyDetector::new(detector.clone()));
        }

        self.rules.push(rule);
    }

    pub fn evaluate(&mut self, metric: &str, value: f64) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let now = SystemTime::now();

        for rule in &self.rules {
            if rule.metric != metric {
                continue;
            }

            // Check cooldown
            if let Some(&last_alert) = self.last_alert_times.get(&rule.name) {
                if let Ok(elapsed) = now.duration_since(last_alert) {
                    if elapsed < rule.cooldown {
                        continue;
                    }
                }
            }

            // Evaluate condition
            let triggered = match &rule.condition {
                AlertCondition::GreaterThan { threshold } => value > *threshold,
                AlertCondition::LessThan { threshold } => value < *threshold,
                AlertCondition::Equals { value: v } => (value - v).abs() < 0.0001,
                AlertCondition::NotEquals { value: v } => (value - v).abs() >= 0.0001,
                AlertCondition::RateOfChange { .. } => false, // Simplified
                AlertCondition::Anomaly { .. } => {
                    if let Some(detector) = self.anomaly_detectors.get_mut(&rule.name) {
                        detector.add_value(value);
                        detector.detect(value).is_some()
                    } else {
                        false
                    }
                }
            };

            if triggered {
                let threshold = match &rule.condition {
                    AlertCondition::GreaterThan { threshold } => *threshold,
                    AlertCondition::LessThan { threshold } => *threshold,
                    _ => 0.0,
                };

                let alert = Alert {
                    id: format!("{}_{}", rule.name, now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    severity: rule.severity,
                    message: format!("Alert: {} - value {} exceeded threshold", rule.name, value),
                    metric: metric.to_string(),
                    value,
                    threshold,
                    timestamp: now,
                    tags: HashMap::new(),
                };

                alerts.push(alert);
                self.last_alert_times.insert(rule.name.clone(), now);
            }
        }

        alerts
    }
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream analytics engine
pub struct StreamAnalyticsEngine {
    dashboard: Arc<Dashboard>,
    anomaly_detectors: RwLock<HashMap<String, AnomalyDetector>>,
    trend_analyzers: RwLock<HashMap<String, TrendAnalyzer>>,
    predictive_analyzers: RwLock<HashMap<String, PredictiveAnalyzer>>,
    alert_engine: RwLock<AlertEngine>,
}

impl StreamAnalyticsEngine {
    pub fn new(dashboard_name: impl Into<String>) -> Self {
        Self {
            dashboard: Arc::new(Dashboard::new(dashboard_name)),
            anomaly_detectors: RwLock::new(HashMap::new()),
            trend_analyzers: RwLock::new(HashMap::new()),
            predictive_analyzers: RwLock::new(HashMap::new()),
            alert_engine: RwLock::new(AlertEngine::new()),
        }
    }

    pub fn process_event(&self, event: &Event) -> Result<AnalyticsResult> {
        let mut result = AnalyticsResult::default();

        // Extract metrics from event
        for (key, value) in &event.payload {
            if let Some(numeric_value) = value.as_f64() {
                // Update dashboard
                let point = DashboardDataPoint {
                    metric: key.clone(),
                    value: numeric_value,
                    timestamp: event.event_time,
                    dimensions: HashMap::new(),
                };
                self.dashboard.add_data_point(point);

                // Anomaly detection
                let mut detectors = self.anomaly_detectors.write().unwrap();
                if let Some(detector) = detectors.get_mut(key) {
                    detector.add_value(numeric_value);
                    if let Some(mut anomaly) = detector.detect(numeric_value) {
                        anomaly.metric = key.clone();
                        result.anomalies.push(anomaly);
                    }
                }

                // Trend analysis
                let mut analyzers = self.trend_analyzers.write().unwrap();
                if let Some(analyzer) = analyzers.get_mut(key) {
                    analyzer.add_value(numeric_value, event.event_time);
                    if let Some(mut trend) = analyzer.analyze() {
                        trend.metric = key.clone();
                        result.trends.push(trend);
                    }
                }

                // Alert evaluation
                let mut alert_engine = self.alert_engine.write().unwrap();
                let alerts = alert_engine.evaluate(key, numeric_value);
                result.alerts.extend(alerts);
            }
        }

        Ok(result)
    }

    pub fn register_anomaly_detector(&self, metric: String, algorithm: AnomalyDetectionAlgorithm) {
        let mut detectors = self.anomaly_detectors.write().unwrap();
        detectors.insert(metric, AnomalyDetector::new(algorithm));
    }

    pub fn register_trend_analyzer(&self, metric: String, window_size: usize) {
        let mut analyzers = self.trend_analyzers.write().unwrap();
        analyzers.insert(metric, TrendAnalyzer::new(window_size));
    }

    pub fn register_alert_rule(&self, rule: AlertRule) {
        let mut alert_engine = self.alert_engine.write().unwrap();
        alert_engine.add_rule(rule);
    }

    pub fn get_dashboard(&self) -> Arc<Dashboard> {
        self.dashboard.clone()
    }
}

/// Analytics processing result
#[derive(Debug, Clone, Default)]
pub struct AnalyticsResult {
    pub anomalies: Vec<Anomaly>,
    pub trends: Vec<Trend>,
    pub alerts: Vec<Alert>,
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_dashboard() {
        let dashboard = Dashboard::new("test_dashboard");

        let point = DashboardDataPoint {
            metric: "cpu_usage".to_string(),
            value: 75.5,
            timestamp: SystemTime::now(),
            dimensions: HashMap::new(),
        };

        dashboard.add_data_point(point);

        let latest = dashboard.get_latest("cpu_usage").unwrap();
        assert_eq!(latest.value, 75.5);
    }

    #[test]
    fn test_anomaly_detector() {
        let mut detector = AnomalyDetector::new(AnomalyDetectionAlgorithm::ZScore { threshold: 2.0 });

        // Add normal values
        for i in 0..100 {
            detector.add_value(50.0 + (i as f64 % 10.0));
        }

        // Add anomaly
        let anomaly = detector.detect(150.0);
        assert!(anomaly.is_some());
    }

    #[test]
    fn test_trend_analyzer() {
        let mut analyzer = TrendAnalyzer::new(10);

        // Add increasing values
        for i in 0..20 {
            analyzer.add_value(i as f64, SystemTime::now());
        }

        let trend = analyzer.analyze().unwrap();
        assert_eq!(trend.direction, TrendDirection::Up);
        assert!(trend.slope > 0.0);
    }

    #[test]
    fn test_predictive_analyzer() {
        let mut analyzer = PredictiveAnalyzer::new(PredictiveModel::SimpleMovingAverage { window: 5 });

        for i in 0..10 {
            analyzer.add_value((i * 10) as f64);
        }

        let predictions = analyzer.predict(3);
        assert_eq!(predictions.len(), 3);
    }

    #[test]
    fn test_alert_engine() {
        let mut engine = AlertEngine::new();

        let rule = AlertRule {
            name: "high_cpu".to_string(),
            metric: "cpu_usage".to_string(),
            condition: AlertCondition::GreaterThan { threshold: 80.0 },
            severity: AlertSeverity::Warning,
            cooldown: Duration::from_secs(60),
        };

        engine.add_rule(rule);

        let alerts = engine.evaluate("cpu_usage", 85.0);
        assert!(!alerts.is_empty());

        // Should be in cooldown now
        let alerts = engine.evaluate("cpu_usage", 90.0);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_stream_analytics_engine() {
        let engine = StreamAnalyticsEngine::new("test");

        engine.register_anomaly_detector(
            "value".to_string(),
            AnomalyDetectionAlgorithm::ZScore { threshold: 2.0 },
        );

        engine.register_trend_analyzer("value".to_string(), 10);

        let rule = AlertRule {
            name: "high_value".to_string(),
            metric: "value".to_string(),
            condition: AlertCondition::GreaterThan { threshold: 100.0 },
            severity: AlertSeverity::Warning,
            cooldown: Duration::from_secs(60),
        };
        engine.register_alert_rule(rule);

        let event = Event::new("test").with_payload("value", 50.0);
        let result = engine.process_event(&event).unwrap();

        // Should have dashboard data
        let dashboard = engine.get_dashboard();
        assert!(dashboard.get_latest("value").is_some());
    }
}


