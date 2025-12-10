// comment

pub mod self_tuning;
pub mod self_healing;
pub mod workload_ml;
pub mod auto_indexing;
pub mod predictive;

// Re-export commonly used types from self_tuning
pub use self_tuning::{
    AutoTuner, AggressivenessLevel, TunableParameter, ParameterValue, ParameterConfig,
    PerformanceMetrics, TuningAction, TuningResult, WorkloadCharacteristics,
    StatisticsGatherer, TableStatistics, TuningReport,
};

// Re-export commonly used types from self_healing
pub use self_healing::{
    SelfHealingEngine, IssueType, Severity, DetectedIssue, HealingAction, HealingResult,
    CorruptionDetector, CorruptionInfo, IndexHealthMonitor, IndexHealth,
    ConnectionPoolManager, ConnectionPoolStats, DeadlockResolver,
    MemoryLeakDetector, MemorySnapshot, FailoverOrchestrator, NodeHealth,
    HealingReport,
};

// Re-export commonly used types from workload_ml
pub use workload_ml::{
    WorkloadMLAnalyzer, QueryFeatures, WorkloadClass, KMeansClassifier,
    PerformancePredictor, AnomalyDetector, PatternRecognizer, QueryPattern,
    TimeSeriesAnalyzer, TimeSeriesPoint, Trend, TrendDirection,
};

// Re-export commonly used types from auto_indexing
pub use auto_indexing::{
    AutoIndexingEngine, IndexAdvisor, IndexCandidate, IndexType, ColumnAccessPattern,
    IndexStatistics, ColumnAccessType, IndexMaintenanceScheduler, MaintenanceTask,
    MaintenanceTaskType, IndexRecommendationReport,
};

// Re-export commonly used types from predictive
pub use predictive::{
    CapacityPlanner, StorageGrowthPredictor, ResponseTimePredictor,
    ResourceExhaustionForecaster, MaintenanceWindowOptimizer,
    Forecast, TimeSeriesDataPoint, ResourceExhaustionAlert, ResourceType,
    AlertSeverity, WorkloadIntensity, MaintenanceWindow, CapacityPlanningReport,
};

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::Result;

// Autonomous database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    // Enable automatic tuning
    pub enable_auto_tuning: bool,

    // Enable self-healing
    pub enable_self_healing: bool,

    // Enable automatic indexing
    pub enable_auto_indexing: bool,

    // Aggressiveness level for tuning
    pub tuning_aggressiveness: AggressivenessLevel,

    // Minimum benefit score for index creation
    pub min_index_benefit_score: f64,

    // Enable automatic index creation
    pub auto_create_indexes: bool,

    // Enable automatic index dropping
    pub auto_drop_indexes: bool,

    // Days threshold for unused index detection
    pub unused_index_threshold_days: u64,

    // Enable ML workload analysis
    pub enable_ml_analysis: bool,

    // Minimum pattern occurrences for recognition
    pub min_pattern_occurrences: usize,

    // Enable predictive analytics
    pub enable_predictive_analytics: bool,
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            enable_auto_tuning: true,
            enable_self_healing: true,
            enable_auto_indexing: false,  // Conservative default
            tuning_aggressiveness: AggressivenessLevel::Moderate,
            min_index_benefit_score: 10.0,
            auto_create_indexes: false,
            auto_drop_indexes: false,
            unused_index_threshold_days: 30,
            enable_ml_analysis: true,
            min_pattern_occurrences: 5,
            enable_predictive_analytics: true,
        }
    }
}

// Unified autonomous database manager
pub struct AutonomousDatabase {
    config: Arc<RwLock<AutonomousConfig>>,
    auto_tuner: Arc<AutoTuner>,
    healing_engine: Arc<SelfHealingEngine>,
    ml_analyzer: Arc<WorkloadMLAnalyzer>,
    auto_indexing: Arc<AutoIndexingEngine>,
    capacity_planner: Arc<CapacityPlanner>,
}

impl AutonomousDatabase {
    // Create a new autonomous database manager
    pub fn new(config: AutonomousConfig) -> Self {
        let auto_tuner = AutoTuner::new(config.tuning_aggressiveness);
        let healing_engine = SelfHealingEngine::new();
        let ml_analyzer = WorkloadMLAnalyzer::new();
        let auto_indexing = AutoIndexingEngine::new();
        let capacity_planner = CapacityPlanner::new();

        // Configure components based on config
        if !config.enable_auto_tuning {
            auto_tuner.disable();
        }

        if !config.enable_self_healing {
            healing_engine.disable();
        }

        if config.auto_create_indexes {
            auto_indexing.enable_auto_create();
        }

        if config.auto_drop_indexes {
            auto_indexing.enable_auto_drop();
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            auto_tuner: Arc::new(auto_tuner),
            healing_engine: Arc::new(healing_engine),
            ml_analyzer: Arc::new(ml_analyzer),
            auto_indexing: Arc::new(auto_indexing),
            capacity_planner: Arc::new(capacity_planner),
        }
    }

    // Start all autonomous operations
    pub async fn start(self: Arc<Self>) {
        let config = self.config.read().clone();

        if config.enable_auto_tuning {
            let tuner = Arc::clone(&self.auto_tuner);
            tokio::spawn(async move {
                tuner.start_optimization_loop().await;
            });
        }

        if config.enable_self_healing {
            let engine = Arc::clone(&self.healing_engine);
            tokio::spawn(async move {
                engine.start_healing_loop().await;
            });
        }

        if config.enable_ml_analysis {
            // ML training runs periodically
            let analyzer = Arc::clone(&self.ml_analyzer);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                loop {
                    interval.tick().await;
                    if let Err(e) = analyzer.train_models() {
                        tracing::error!("ML model training failed: {}", e);
                    }
                }
            });
        }
    }

    // Get auto-tuner component
    pub fn auto_tuner(&self) -> &Arc<AutoTuner> {
        &self.auto_tuner
    }

    // Get self-healing engine
    pub fn healing_engine(&self) -> &Arc<SelfHealingEngine> {
        &self.healing_engine
    }

    // Get ML analyzer
    pub fn ml_analyzer(&self) -> &Arc<WorkloadMLAnalyzer> {
        &self.ml_analyzer
    }

    // Get auto-indexing engine
    pub fn auto_indexing(&self) -> &Arc<AutoIndexingEngine> {
        &self.auto_indexing
    }

    // Get capacity planner
    pub fn capacity_planner(&self) -> &Arc<CapacityPlanner> {
        &self.capacity_planner
    }

    // Update configuration
    pub fn update_config(&self, config: AutonomousConfig) {
        *self.config.write() = config;
    }

    // Get current configuration
    pub fn get_config(&self) -> AutonomousConfig {
        self.config.read().clone()
    }

    // Generate comprehensive autonomous operations report
    pub fn generate_report(&self, current_capacity_gb: f64) -> Result<AutonomousReport> {
        let tuning_report = self.auto_tuner.get_tuning_report();
        let healing_report = self.healing_engine.get_healing_report();
        let index_report = self.auto_indexing.get_recommendations();
        let capacity_report = self.capacity_planner.generate_report(current_capacity_gb)?;
        let recurring_patterns = self.ml_analyzer.get_recurring_patterns();

        Ok(AutonomousReport {
            tuning: tuning_report,
            healing: healing_report,
            indexing: index_report,
            capacity: capacity_report,
            pattern_count: recurring_patterns.len(),
            timestamp: std::time::SystemTime::now(),
        })
    }
}

// Comprehensive autonomous operations report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousReport {
    pub tuning: TuningReport,
    pub healing: HealingReport,
    pub indexing: IndexRecommendationReport,
    pub capacity: CapacityPlanningReport,
    pub pattern_count: usize,
    pub timestamp: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::Duration;
use std::time::SystemTime;

    #[test]
    fn test_autonomous_config_default() {
        let config = AutonomousConfig::default();
        assert!(config.enable_auto_tuning);
        assert!(config.enable_self_healing);
        assert!(!config.auto_create_indexes);  // Conservative default
    }

    #[tokio::test]
    async fn test_autonomous_database_creation() {
        let config = AutonomousConfig::default();
        let db = Arc::new(AutonomousDatabase::new(config));

        assert!(db.auto_tuner().is_enabled());
        assert!(db.healing_engine().is_enabled());
    }

    #[test]
    fn test_autonomous_report_generation() {
        let config = AutonomousConfig::default();
        let db = AutonomousDatabase::new(config);

        let report = db.generate_report(1000.0);
        assert!(report.is_ok());
    }
}
