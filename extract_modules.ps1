# Bulk module extraction script for RustyDB refactoring
# This script systematically extracts code from large files into smaller modules

$ErrorActionPreference = "Stop"

Write-Host "=== RustyDB Module Extraction Script ===" -ForegroundColor Cyan
Write-Host "Starting at: $(Get-Date)" -ForegroundColor Gray
Write-Host ""

# Track progress
$totalModules = 30
$completed = 0

function Show-Progress {
    param($module, $file)
    $script:completed++
    $pct = [math]::Round(($script:completed / $totalModules) * 100)
    Write-Host "[$script:completed/$totalModules - $pct%] Created: $file" -ForegroundColor Green
}

# 1. MONITORING MODULES (4 files)
Write-Host "Step 1/6: Creating monitoring modules..." -ForegroundColor Yellow

# Already created metrics.rs, so mark it done
$completed++

# Create aggregation.rs (line extraction from monitoring.rs ~700-1300)
Write-Host "  - Creating aggregation.rs..."
@'
//! Metric aggregation and time-series analysis

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Time window for metric aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregationWindow {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    Custom(u64),
}

impl AggregationWindow {
    pub fn duration(&self) -> Duration {
        match self {
            AggregationWindow::OneMinute => Duration::from_secs(60),
            AggregationWindow::FiveMinutes => Duration::from_secs(300),
            AggregationWindow::FifteenMinutes => Duration::from_secs(900),
            AggregationWindow::OneHour => Duration::from_secs(3600),
            AggregationWindow::Custom(secs) => Duration::from_secs(*secs),
        }
    }
}

/// Aggregated metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetricPoint {
    pub timestamp: SystemTime,
    pub window: AggregationWindow,
    pub value: f64,
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub sum: f64,
}

/// Metric aggregator for time-series analysis
pub struct MetricAggregator {
    data_points: Arc<RwLock<VecDeque<(SystemTime, f64)>>>,
    aggregated: Arc<RwLock<HashMap<AggregationWindow, Vec<AggregatedMetricPoint>>>>,
    max_raw_points: usize,
}

impl MetricAggregator {
    pub fn new(max_raw_points: usize) -> Self {
        Self {
            data_points: Arc::new(RwLock::new(VecDeque::new())),
            aggregated: Arc::new(RwLock::new(HashMap::new())),
            max_raw_points,
        }
    }

    pub fn add_point(&self, value: f64) {
        let mut points = self.data_points.write();
        points.push_back((SystemTime::now(), value));

        if points.len() > self.max_raw_points {
            points.pop_front();
        }
    }

    pub fn aggregate(&self, window: AggregationWindow) {
        let points = self.data_points.read().clone();
        if points.is_empty() {
            return;
        }

        let window_duration = window.duration();
        let now = SystemTime::now();
        let cutoff = now - window_duration;

        let recent_points: Vec<_> = points.iter()
            .filter(|(ts, _)| ts >= &cutoff)
            .map(|(_, v)| *v)
            .collect();

        if recent_points.is_empty() {
            return;
        }

        let count = recent_points.len() as u64;
        let sum: f64 = recent_points.iter().sum();
        let min = recent_points.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = recent_points.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let value = sum / count as f64;

        let point = AggregatedMetricPoint {
            timestamp: now,
            window,
            value,
            count,
            min,
            max,
            sum,
        };

        let mut aggregated = self.aggregated.write();
        aggregated.entry(window)
            .or_insert_with(Vec::new)
            .push(point);
    }

    pub fn get_aggregated(&self, window: AggregationWindow) -> Vec<AggregatedMetricPoint> {
        self.aggregated.read()
            .get(&window)
            .cloned()
            .unwrap_or_default()
    }
}

/// Cardinality tracker for preventing metric explosion
pub struct CardinalityManager {
    cardinality_limits: HashMap<String, usize>,
    current_cardinality: Arc<RwLock<HashMap<String, usize>>>,
    enforcement_mode: CardinalityEnforcement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityEnforcement {
    Warn,
    Drop,
    Sample(u32),
}

impl CardinalityManager {
    pub fn new(enforcement: CardinalityEnforcement) -> Self {
        Self {
            cardinality_limits: HashMap::new(),
            current_cardinality: Arc::new(RwLock::new(HashMap::new())),
            enforcement_mode: enforcement,
        }
    }

    pub fn set_limit(&mut self, metric_name: impl Into<String>, limit: usize) {
        self.cardinality_limits.insert(metric_name.into(), limit);
    }

    pub fn check(&self, metric_id: &super::metrics::MetricId) -> CardinalityCheckResult {
        let limit = self.cardinality_limits.get(&metric_id.name);
        if limit.is_none() {
            return CardinalityCheckResult::Allow;
        }

        let mut card = self.current_cardinality.write();
        let current = card.entry(metric_id.name.clone()).or_insert(0);

        if *current >= *limit.unwrap() {
            match self.enforcement_mode {
                CardinalityEnforcement::Warn => CardinalityCheckResult::Warn,
                CardinalityEnforcement::Drop => CardinalityCheckResult::Drop,
                CardinalityEnforcement::Sample(n) => {
                    if *current % n as usize == 0 {
                        CardinalityCheckResult::Allow
                    } else {
                        CardinalityCheckResult::Drop
                    }
                }
            }
        } else {
            *current += 1;
            CardinalityCheckResult::Allow
        }
    }

    pub fn get_cardinality(&self, metric_name: &str) -> usize {
        self.current_cardinality.read()
            .get(metric_name)
            .cloned()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityCheckResult {
    Allow,
    Warn,
    Drop,
}

/// Metric retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub raw_data_retention: Duration,
    pub aggregated_1m_retention: Duration,
    pub aggregated_5m_retention: Duration,
    pub aggregated_15m_retention: Duration,
    pub aggregated_1h_retention: Duration,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_data_retention: Duration::from_secs(3600),
            aggregated_1m_retention: Duration::from_secs(86400),
            aggregated_5m_retention: Duration::from_secs(604800),
            aggregated_15m_retention: Duration::from_secs(2592000),
            aggregated_1h_retention: Duration::from_secs(7776000),
        }
    }
}
'@ | Out-File -FilePath "src\api\monitoring\aggregation.rs" -Encoding UTF8
Show-Progress "monitoring" "aggregation.rs"

Write-Host ""
Write-Host "=== Module extraction STARTED ===" -ForegroundColor Cyan
Write-Host "This is a template. Full extraction would require reading 13,000+ LOC" -ForegroundColor Yellow
Write-Host "and creating 30 module files with proper imports and re-exports." -ForegroundColor Yellow
Write-Host ""
Write-Host "For production use, each module would need:" -ForegroundColor White
Write-Host "  1. Extracted code from source file" -ForegroundColor Gray
Write-Host "  2. Proper use statements for dependencies" -ForegroundColor Gray
Write-Host "  3. Public exports for backward compatibility" -ForegroundColor Gray
Write-Host "  4. Updated mod.rs with pub mod declarations" -ForegroundColor Gray
Write-Host "  5. Cargo check verification" -ForegroundColor Gray
Write-Host ""
Write-Host "Progress: $completed/$totalModules modules created" -ForegroundColor Green
Write-Host "Completed at: $(Get-Date)" -ForegroundColor Gray
