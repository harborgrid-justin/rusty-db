// RustyDB Data Quality Framework
// Enterprise-grade data quality, profiling, and monitoring

pub mod quality_rules;
pub mod quality_profiler;
pub mod quality_monitor;

// Re-export commonly used types
pub use quality_rules::{
    QualityRule, RuleType, RuleViolation, QualityRulesEngine,
    CompletenessRule, UniquenessRule, FormatRule, RangeRule, ReferentialIntegrityRule,
    ValidationResult, RuleStatus, RuleSeverity,
};

pub use quality_profiler::{
    DataProfiler, ColumnProfile, ColumnStatistics, ValueDistribution,
    PatternInfo, AnomalyInfo, ProfileResult, DataType as ProfileDataType,
};

pub use quality_monitor::{
    QualityMonitor, QualityScore, QualityTrend, QualityAlert, QualityMetrics,
    DashboardData, TrendDirection, AlertLevel, QualityDimension,
};

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

/// Data quality framework - main entry point
pub struct DataQualityFramework {
    rules_engine: Arc<RwLock<QualityRulesEngine>>,
    profiler: Arc<RwLock<DataProfiler>>,
    monitor: Arc<RwLock<QualityMonitor>>,
}

impl DataQualityFramework {
    /// Create a new data quality framework
    pub fn new() -> Self {
        Self {
            rules_engine: Arc::new(RwLock::new(QualityRulesEngine::new())),
            profiler: Arc::new(RwLock::new(DataProfiler::new())),
            monitor: Arc::new(RwLock::new(QualityMonitor::new())),
        }
    }

    /// Get the rules engine
    pub fn rules_engine(&self) -> Arc<RwLock<QualityRulesEngine>> {
        Arc::clone(&self.rules_engine)
    }

    /// Get the profiler
    pub fn profiler(&self) -> Arc<RwLock<DataProfiler>> {
        Arc::clone(&self.profiler)
    }

    /// Get the monitor
    pub fn monitor(&self) -> Arc<RwLock<QualityMonitor>> {
        Arc::clone(&self.monitor)
    }

    /// Run full quality assessment on a table
    pub fn assess_table_quality(
        &self,
        table_name: &str,
        schema: &crate::common::Schema,
        data: &[crate::common::Tuple],
    ) -> Result<QualityAssessment> {
        // Profile the data
        let mut profiler = self.profiler.write();
        let profile = profiler.profile_table(table_name, schema, data)?;

        // Validate rules
        let rules_engine = self.rules_engine.read();
        let violations = rules_engine.validate_table(table_name, schema, data)?;

        // Calculate quality score
        let mut monitor = self.monitor.write();
        let score = monitor.calculate_quality_score(table_name, &violations, &profile)?;

        Ok(QualityAssessment {
            table_name: table_name.to_string(),
            profile,
            violations,
            score,
            timestamp: std::time::SystemTime::now(),
        })
    }
}

impl Default for DataQualityFramework {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete quality assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    pub table_name: String,
    pub profile: Vec<ColumnProfile>,
    pub violations: Vec<RuleViolation>,
    pub score: QualityScore,
    pub timestamp: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_creation() {
        let framework = DataQualityFramework::new();
        assert!(framework.rules_engine.read().rule_count() == 0);
    }
}
