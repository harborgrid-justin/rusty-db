//! Predictive Analytics for Database Operations
//!
//! Forecasting and prediction capabilities for storage growth, query performance,
//! resource exhaustion, and capacity planning.

use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::error::DbError;

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
}

/// Forecast result with confidence intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub timestamp: SystemTime,
    pub predicted_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence_level: f64,
}

/// Storage growth predictor
pub struct StorageGrowthPredictor {
    historical_data: Arc<RwLock<VecDeque<TimeSeriesDataPoint>>>,
    max_history_size: usize,
}

impl StorageGrowthPredictor {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            historical_data: Arc::new(RwLock::new(VecDeque::with_capacity(max_history_size))),
            max_history_size,
        }
    }

    pub fn record_storage_size(&self, size_gb: f64) {
        let mut data = self.historical_data.write();

        if data.len() >= self.max_history_size {
            data.pop_front();
        }

        data.push_back(TimeSeriesDataPoint {
            timestamp: SystemTime::now(),
            value: size_gb,
        });
    }

    pub fn predict_growth(&self, days_ahead: usize) -> Result<Vec<Forecast>> {
        let data = self.historical_data.read();

        if data.len() < 3 {
            return Err(DbError::Internal("Insufficient historical data for prediction".to_string()));
        }

        // Use linear regression with exponential smoothing
        let points: Vec<(f64, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.value))
            .collect();

        let (slope, intercept, std_error) = self.fit_linear_model(&points);

        let mut forecasts = Vec::new();
        let next_index = data.len();

        for _i in 0..days_ahead {
            let x = (next_index + i) as f64;
            let predicted = slope * x + intercept;

            // Calculate confidence interval (assuming normal distribution)
            let margin = 1.96 * std_error * (1.0 + 1.0 / data.len() as f64).sqrt();

            forecasts.push(Forecast {
                timestamp: SystemTime::now() + Duration::from_secs(86400 * (i + 1) as u64),
                predicted_value: predicted,
                lower_bound: (predicted - margin).max(0.0),
                upper_bound: predicted + margin,
                confidence_level: 0.95,
            });
        }

        Ok(forecasts)
    }

    fn fit_linear_model(&self, points: &[(f64, f64)]) -> (f64, f64, f64) {
        let n = points.len() as f64;
        let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
        let sum_x2: f64 = points.iter().map(|(x, _)| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate standard error
        let residuals: Vec<f64> = points
            .iter()
            .map(|(x, y)| {
                let predicted = slope * x + intercept;
                (y - predicted).powi(2)
            })
            .collect();

        let mse = residuals.iter().sum::<f64>() / n;
        let std_error = mse.sqrt();

        (slope, intercept, std_error)
    }

    pub fn estimate_days_until_full(&self, capacity_gb: f64) -> Option<usize> {
        let data = self.historical_data.read();

        if data.is_empty() {
            return None;
        }

        let current_size = data.back()?.value;

        if current_size >= capacity_gb {
            return Some(0);
        }

        // Predict when storage will reach capacity
        match self.predict_growth(365) {
            Ok(forecasts) => {
                for (days, forecast) in forecasts.iter().enumerate() {
                    if forecast.predicted_value >= capacity_gb {
                        return Some(days + 1);
                    }
                }
                None  // Won't fill in next year
            }
            Err(_) => None,
        }
    }
}

/// Query response time predictor
pub struct ResponseTimePredictor {
    query_history: Arc<RwLock<HashMap<u64<f64>>>>,  // query_hash -> execution times
    max_history_per_query: usize,
}

impl ResponseTimePredictor {
    pub fn new(max_history_per_query: usize) -> Self {
        Self {
            query_history: Arc::new(RwLock::new(HashMap::new())),
            max_history_per_query,
        }
    }

    pub fn record_execution(&self, query_hash: u64, execution_time_ms: f64) {
        let mut history = self.query_history.write();

        history
            .entry(query_hash)
            .or_insert_with(|| VecDeque::with_capacity(self.max_history_per_query))
            .push_back(execution_time_ms);

        // Limit history size
        if let Some(queue) = history.get_mut(&query_hash) {
            if queue.len() > self.max_history_per_query {
                queue.pop_front();
            }
        }
    }

    pub fn predict_execution_time(&self, query_hash: u64) -> Option<Forecast> {
        let history = self.query_history.read();
        let times = history.get(&query_hash)?;

        if times.is_empty() {
            return None;
        }

        // Use exponential moving average for prediction
        let alpha = 0.3;  // Smoothing factor
        let mut ema = times[0];

        for &time in times.iter().skip(1) {
            ema = alpha * time + (1.0 - alpha) * ema;
        }

        // Calculate standard deviation for confidence interval
        let mean: f64 = times.iter().sum::<f64>() / times.len() as f64;
        let variance: f64 = times.iter().map(|&t| (t - mean).powi(2)).sum::<f64>() / times.len() as f64;
        let std_dev = variance.sqrt();

        Some(Forecast {
            timestamp: SystemTime::now(),
            predicted_value: ema,
            lower_bound: (ema - 1.96 * std_dev).max(0.0),
            upper_bound: ema + 1.96 * std_dev,
            confidence_level: 0.95,
        })
    }

    pub fn detect_regression(&self, query_hash: u64, current_time_ms: f64) -> bool {
        if let Some(forecast) = self.predict_execution_time(query_hash) {
            // Regression if current time exceeds upper bound
            current_time_ms > forecast.upper_bound
        } else {
            false
        }
    }
}

/// Resource exhaustion forecaster
pub struct ResourceExhaustionForecaster {
    cpu_history: VecDeque<TimeSeriesDataPoint>,
    memory_history: VecDeque<TimeSeriesDataPoint>,
    disk_io_history: VecDeque<TimeSeriesDataPoint>,
    network_history: VecDeque<TimeSeriesDataPoint>,
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceExhaustionAlert {
    pub resource_type: ResourceType,
    pub current_usage: f64,
    pub predicted_usage: f64,
    pub time_until_exhaustion: Duration,
    pub severity: AlertSeverity,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    CPU,
    Memory,
    DiskIO,
    Network,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ResourceExhaustionForecaster {
    pub fn new(max_history: usize) -> Self {
        Self {
            cpu_history: VecDeque::with_capacity(max_history),
            memory_history: VecDeque::with_capacity(max_history),
            disk_io_history: VecDeque::with_capacity(max_history),
            network_history: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    pub fn record_cpu_usage(&mut self, usage_percent: f64) {
        Self::add_to_history(&mut self.cpu_history, usage_percent, self.max_history);
    }

    pub fn record_memory_usage(&mut self, usage_percent: f64) {
        Self::add_to_history(&mut self.memory_history, usage_percent, self.max_history);
    }

    pub fn record_disk_io(&mut self, io_percent: f64) {
        Self::add_to_history(&mut self.disk_io_history, io_percent, self.max_history);
    }

    pub fn record_network(&mut self, network_percent: f64) {
        Self::add_to_history(&mut self.network_history, network_percent, self.max_history);
    }

    fn add_to_history(history: &mut VecDeque<TimeSeriesDataPoint>, value: f64, max_history: usize) {
        if history.len() >= max_history {
            history.pop_front();
        }

        history.push_back(TimeSeriesDataPoint {
            timestamp: SystemTime::now(),
            value,
        });
    }

    pub fn check_for_alerts(&self) -> Vec<ResourceExhaustionAlert> {
        let mut alerts = Vec::new();

        if let Some(alert) = self.check_resource(&self.cpu_history, ResourceType::CPU, 90.0) {
            alerts.push(alert);
        }

        if let Some(alert) = self.check_resource(&self.memory_history, ResourceType::Memory, 85.0) {
            alerts.push(alert);
        }

        if let Some(alert) = self.check_resource(&self.disk_io_history, ResourceType::DiskIO, 80.0) {
            alerts.push(alert);
        }

        if let Some(alert) = self.check_resource(&self.network_history, ResourceType::Network, 75.0) {
            alerts.push(alert);
        }

        alerts
    }

    fn check_resource(
        &self,
        history: &VecDeque<TimeSeriesDataPoint>,
        resource_type: ResourceType,
        threshold: f64,
    ) -> Option<ResourceExhaustionAlert> {
        if history.len() < 5 {
            return None;
        }

        let current = history.back()?.value;

        // Calculate trend
        let points: Vec<(f64, f64)> = history
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.value))
            .collect();

        let slope = self.calculate_trend_slope(&points);

        // Predict when resource will be exhausted
        if slope > 0.1 {  // Resource usage is increasing
            let steps_to_exhaustion = ((100.0 - current) / slope).ceil() as u64;
            let time_until_exhaustion = Duration::from_secs(steps_to_exhaustion * 60);  // Assuming 1-minute intervals

            if current > threshold || time_until_exhaustion < Duration::from_secs(3600) {
                let severity = if current > 95.0 {
                    AlertSeverity::Critical
                } else if current > 90.0 {
                    AlertSeverity::High
                } else if current > 80.0 {
                    AlertSeverity::Medium
                } else {
                    AlertSeverity::Low
                };

                let recommendation = match resource_type {
                    ResourceType::CPU => "Consider scaling up CPU resources or optimizing queries".to_string(),
                    ResourceType::Memory => "Increase memory allocation or reduce buffer pool size".to_string(),
                    ResourceType::DiskIO => "Optimize disk-intensive operations or add more storage devices".to_string(),
                    ResourceType::Network => "Check for network bottlenecks or reduce replication traffic".to_string(),
                    ResourceType::Storage => "Expand storage capacity or archive old data".to_string(),
                };

                return Some(ResourceExhaustionAlert {
                    resource_type,
                    current_usage: current,
                    predicted_usage: current + slope * 10.0,  // 10 steps ahead
                    time_until_exhaustion,
                    severity,
                    recommendation,
                });
            }
        }

        None
    }

    fn calculate_trend_slope(&self, points: &[(f64, f64)]) -> f64 {
        let n = points.len() as f64;
        let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
        let sum_x2: f64 = points.iter().map(|(x, _)| x * x).sum();

        (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x)
    }
}

/// Maintenance window optimizer
pub struct MaintenanceWindowOptimizer {
    workload_patterns: Arc<RwLock<HashMap<u8, WorkloadIntensity>>>,  // hour -> intensity
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkloadIntensity {
    pub hour: u8,
    pub avg_queries_per_second: f64,
    pub avg_cpu_usage: f64,
    pub avg_connections: usize,
}

impl MaintenanceWindowOptimizer {
    pub fn new() -> Self {
        Self {
            workload_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record_workload(&self, hour: u8, qps: f64, cpu_usage: f64, connections: usize) {
        let mut patterns = self.workload_patterns.write();

        patterns
            .entry(hour)
            .and_modify(|intensity| {
                // Update moving average
                intensity.avg_queries_per_second =
                    (intensity.avg_queries_per_second * 0.9) + (qps * 0.1);
                intensity.avg_cpu_usage =
                    (intensity.avg_cpu_usage * 0.9) + (cpu_usage * 0.1);
                intensity.avg_connections =
                    ((intensity.avg_connections as f64 * 0.9) + (connections as f64 * 0.1)) as usize;
            })
            .or_insert(WorkloadIntensity {
                hour,
                avg_queries_per_second: qps,
                avg_cpu_usage: cpu_usage,
                avg_connections: connections,
            });
    }

    pub fn recommend_maintenance_window(&self, duration_hours: usize) -> Option<MaintenanceWindow> {
        let patterns = self.workload_patterns.read();

        if patterns.is_empty() {
            return None;
        }

        let mut best_start_hour = 0u8;
        let mut min_impact = f64::INFINITY;

        // Try each possible start hour
        for start_hour in 0..24 {
            let end_hour = (start_hour + duration_hours as u8) % 24;

            let mut total_impact = 0.0;
            let mut hours_checked = 0;

            for offset in 0..duration_hours {
                let hour = (start_hour + offset as u8) % 24;

                if let Some(intensity) = patterns.get(&hour) {
                    // Impact is a combination of QPS and CPU usage
                    total_impact += intensity.avg_queries_per_second * 0.6 + intensity.avg_cpu_usage * 0.4;
                    hours_checked += 1;
                }
            }

            if hours_checked > 0 {
                let avg_impact = total_impact / hours_checked as f64;

                if avg_impact < min_impact {
                    min_impact = avg_impact;
                    best_start_hour = start_hour;
                }
            }
        }

        Some(MaintenanceWindow {
            start_hour: best_start_hour,
            duration_hours,
            expected_impact_score: min_impact,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub start_hour: u8,
    pub duration_hours: usize,
    pub expected_impact_score: f64,
}

/// Capacity planning recommendations
pub struct CapacityPlanner {
    storage_predictor: Arc<StorageGrowthPredictor>,
    resource_forecaster: Arc<RwLock<ResourceExhaustionForecaster>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityPlanningReport {
    pub storage_forecast: Option<Vec<Forecast>>,
    pub days_until_storage_full: Option<usize>,
    pub resource_alerts: Vec<ResourceExhaustionAlert>,
    pub recommended_actions: Vec<String>,
    pub estimated_cost_impact: f64,
}

impl CapacityPlanner {
    pub fn new() -> Self {
        Self {
            storage_predictor: Arc::new(StorageGrowthPredictor::new(365)),
            resource_forecaster: Arc::new(RwLock::new(ResourceExhaustionForecaster::new(1000))),
        }
    }

    pub fn generate_report(&self, current_capacity_gb: f64) -> Result<CapacityPlanningReport> {
        let storage_forecast = self.storage_predictor.predict_growth(90).ok();
        let days_until_full = self.storage_predictor.estimate_days_until_full(current_capacity_gb);
        let resource_alerts = self.resource_forecaster.read().check_for_alerts();

        let mut recommended_actions = Vec::new();

        if let Some(days) = days_until_full {
            if days < 30 {
                recommended_actions.push(format!(
                    "URGENT: Storage will be full in {} days. Expand capacity immediately.",
                    days
                ));
            } else if days < 90 {
                recommended_actions.push(format!(
                    "Plan storage expansion within {} days to avoid disruption.",
                    days
                ));
            }
        }

        for alert in &resource_alerts {
            if matches!(alert.severity, AlertSeverity::Critical | AlertSeverity::High) {
                recommended_actions.push(alert.recommendation.clone());
            }
        }

        let estimated_cost_impact = self.estimate_cost_impact(&resource_alerts, days_until_full);

        Ok(CapacityPlanningReport {
            storage_forecast,
            days_until_storage_full: days_until_full,
            resource_alerts,
            recommended_actions,
            estimated_cost_impact,
        })
    }

    fn estimate_cost_impact(&self, alerts: &[ResourceExhaustionAlert], days_until_full: Option<usize>) -> f64 {
        let mut cost = 0.0;

        // Storage expansion cost
        if let Some(days) = days_until_full {
            if days < 90 {
                cost += 1000.0;  // Estimated storage expansion cost
            }
        }

        // Resource upgrade costs
        for alert in alerts {
            cost += match alert.severity {
                AlertSeverity::Critical => 500.0,
                AlertSeverity::High => 300.0,
                AlertSeverity::Medium => 100.0,
                AlertSeverity::Low => 50.0,
            };
        }

        cost
    }

    pub fn record_storage(&self, size_gb: f64) {
        self.storage_predictor.record_storage_size(size_gb);
    }

    pub fn record_resource_usage(&self, cpu: f64, memory: f64, disk_io: f64, network: f64) {
        let mut forecaster = self.resource_forecaster.write();
        forecaster.record_cpu_usage(cpu);
        forecaster.record_memory_usage(memory);
        forecaster.record_disk_io(disk_io);
        forecaster.record_network(network);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_growth_prediction() {
        let predictor = StorageGrowthPredictor::new(100);

        // Record increasing storage
        for _i in 0..10 {
            predictor.record_storage_size(100.0 + i as f64 * 5.0);
        }

        let forecasts = predictor.predict_growth(7);
        assert!(forecasts.is_ok());

        let forecasts = forecasts.unwrap();
        assert_eq!(forecasts.len(), 7);
        assert!(forecasts[0].predicted_value > 145.0);
    }

    #[test]
    fn test_response_time_prediction() {
        let predictor = ResponseTimePredictor::new(100);

        let query_hash = 12345u64;

        for _i in 0..10 {
            predictor.record_execution(query_hash, 100.0 + i as f64);
        }

        let forecast = predictor.predict_execution_time(query_hash);
        assert!(forecast.is_some());

        let forecast = forecast.unwrap();
        assert!(forecast.predicted_value > 100.0);
    }

    #[test]
    fn test_resource_exhaustion_detection() {
        let mut forecaster = ResourceExhaustionForecaster::new(100);

        // Record increasing CPU usage
        for _i in 0..10 {
            forecaster.record_cpu_usage(70.0 + i as f64 * 2.0);
        }

        let alerts = forecaster.check_for_alerts();
        assert!(!alerts.is_empty());
    }

    #[test]
    fn test_maintenance_window_optimization() {
        let optimizer = MaintenanceWindowOptimizer::new();

        // Record low workload during night hours
        for hour in 0..24 {
            let qps = if hour >= 22 || hour <= 6 { 10.0 } else { 100.0 };
            optimizer.record_workload(hour, qps, 50.0, 20);
        }

        let window = optimizer.recommend_maintenance_window(4);
        assert!(window.is_some());

        let window = window.unwrap();
        // Should recommend night hours
        assert!(window.start_hour >= 22 || window.start_hour <= 2);
    }
}


