// RustyDB Workload Intelligence Module
// Oracle-like AWR, SQL Tuning Advisor, and Performance Diagnostics

use std::time::SystemTime;
pub mod repository;
pub mod sql_tuning;
pub mod sql_monitor;
pub mod performance_hub;
pub mod advisor;

// Re-export commonly used types from repository
pub use repository::{
    WorkloadRepository, WorkloadSnapshot, RepositoryConfig, SnapshotId, BaselineId,
    Baseline, BaselineType, InstanceInfo, SystemStatistics, SqlStatementStats,
    SessionStats, WaitEventStats, IoStatistics, MemoryStatistics, TimeModelStats,
    LoadProfile, OsStatistics, TablespaceStats, SegmentStats, AggregatedMetrics,
    SnapshotComparison, DeltaStatistics, RepositoryStats,
};

// Re-export commonly used types from sql_tuning
pub use sql_tuning::{
    SqlTuningAdvisor, TuningConfig, TuningTask, TaskId, TaskStatus, TuningScope,
    TuningRecommendation, RecommendationType, BenefitType, RecommendationDetails,
    SqlProfile, ProfileStatus, SqlProfileDetails, IndexRecommendation,
    RestructureRecommendation, RewriteType, StatisticsRecommendation,
    AlternativePlanDetails, PlanAnalysis, PlanOperation, PlanIssue,
    IssueSeverity, IssueType, AccessPath, AccessType, OptimizerStatistics,
    TableStatistics, IndexStatistics, ColumnStatistics, Histogram, HistogramType,
    HistogramBucket,
};

// Re-export commonly used types from sql_monitor
pub use sql_monitor::{
    SqlMonitor, MonitorConfig, SqlExecution, ExecutionId, ExecutionStatus,
    WaitEventDetail, PlanOperationMetrics, ParallelExecutionInfo, ParallelServer,
    BindVariable, ExecutionPlan, PlanOperation as MonitorPlanOperation,
    PerformanceAlert, AlertType, AlertSeverity, ExecutionStatistics,
};

// Re-export commonly used types from performance_hub
pub use performance_hub::{
    PerformanceHub, PerformanceHubConfig, SqlStats, SessionActivity, SessionState,
    WaitEventMetrics, WaitEventSample, FileIoStats, FileType, IoSample, IoOperation,
    MemoryUsage, TrendDataPoint, PerformanceSummary, SystemMetrics,
};

// Re-export commonly used types from advisor
pub use advisor::{
    DiagnosticAdvisor, AdvisorConfig, AnalysisRun, AnalysisId, AnalysisStatus,
    AnalysisScope, Finding, FindingType, FindingSeverity, ImpactType, Evidence,
    EvidenceType, Recommendation, RecommendationPriority, RecommendationCategory,
    ImplementationEffort, PerformanceBaseline, BaselineMetrics, AnalysisSummary,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use crate::Result;

// Unified Workload Intelligence Hub
//
// Integrates all workload intelligence components:
// - AWR-like workload repository
// - SQL Tuning Advisor
// - Real-time SQL monitoring
// - Performance dashboard
// - Automatic diagnostic advisor (ADDM)
pub struct WorkloadIntelligence {
    // Workload repository for historical analysis
    pub repository: Arc<WorkloadRepository>,

    // SQL Tuning Advisor for query optimization
    pub sql_tuning: Arc<SqlTuningAdvisor>,

    // Real-time SQL monitor
    pub sql_monitor: Arc<SqlMonitor>,

    // Performance hub for unified views
    pub performance_hub: Arc<PerformanceHub>,

    // Diagnostic advisor for automatic problem detection
    pub advisor: Arc<DiagnosticAdvisor>,

    // Configuration
    config: Arc<RwLock<WorkloadIntelligenceConfig>>,
}

// Workload Intelligence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadIntelligenceConfig {
    // Enable automatic snapshot collection
    pub auto_snapshot_enabled: bool,

    // Snapshot interval (seconds)
    pub snapshot_interval_secs: u64,

    // Enable automatic analysis
    pub auto_analysis_enabled: bool,

    // Analysis interval (seconds)
    pub analysis_interval_secs: u64,

    // Enable SQL monitoring
    pub sql_monitoring_enabled: bool,

    // Enable SQL tuning recommendations
    pub sql_tuning_enabled: bool,
}

impl Default for WorkloadIntelligenceConfig {
    fn default() -> Self {
        Self {
            auto_snapshot_enabled: true,
            snapshot_interval_secs: 3600, // 1 hour
            auto_analysis_enabled: true,
            analysis_interval_secs: 3600,
            sql_monitoring_enabled: true,
            sql_tuning_enabled: true,
        }
    }
}

impl WorkloadIntelligence {
    // Create a new Workload Intelligence system with default configuration
    pub fn new() -> Self {
        Self::with_config(WorkloadIntelligenceConfig::default())
    }

    // Create a new Workload Intelligence system with custom configuration
    pub fn with_config(config: WorkloadIntelligenceConfig) -> Self {
        let repo_config = RepositoryConfig {
            snapshot_interval_secs: config.snapshot_interval_secs,
            auto_collection_enabled: config.auto_snapshot_enabled,
            ..Default::default()
        };

        let tuning_config = TuningConfig {
            ..Default::default()
        };

        let monitor_config = MonitorConfig {
            ..Default::default()
        };

        let hub_config = PerformanceHubConfig {
            ..Default::default()
        };

        let advisor_config = AdvisorConfig {
            auto_analysis_enabled: config.auto_analysis_enabled,
            analysis_interval_secs: config.analysis_interval_secs,
            ..Default::default()
        };

        Self {
            repository: Arc::new(WorkloadRepository::with_config(repo_config)),
            sql_tuning: Arc::new(SqlTuningAdvisor::with_config(tuning_config)),
            sql_monitor: Arc::new(SqlMonitor::with_config(monitor_config)),
            performance_hub: Arc::new(PerformanceHub::with_config(hub_config)),
            advisor: Arc::new(DiagnosticAdvisor::with_config(advisor_config)),
            config: Arc::new(RwLock::new(config)),
        }
    }

    // Capture a workload snapshot
    pub fn capture_snapshot(&self) -> Result<SnapshotId> {
        // Collect current performance data
        let snapshot = self.collect_current_snapshot();

        // Store in repository
        self.repository.capture_snapshot(snapshot)
    }

    // Collect current snapshot data
    fn collect_current_snapshot(&self) -> WorkloadSnapshot {
        let now = SystemTime::now();

        // Get data from performance hub
        let summary = self.performance_hub.get_performance_summary();

        WorkloadSnapshot {
            snapshot_id: 0, // Will be assigned by repository
            timestamp: now,
            instance_info: InstanceInfo {
                instance_name: "rustydb1".to_string(),
                host_name: "localhost".to_string(),
                version: "1.0.0".to_string(),
                startup_time: now,
                database_size_bytes: 1024 * 1024 * 1024,
                uptime_seconds: 3600,
            },
            system_stats: SystemStatistics {
                transactions_committed: 1000,
                transactions_rolled_back: 10,
                queries_executed: 5000,
                queries_per_second: summary.system_metrics.queries_per_second,
                transactions_per_second: summary.system_metrics.transactions_per_second,
                logical_reads: 100000,
                physical_reads: 5000,
                physical_writes: 2000,
                buffer_cache_hit_ratio: summary.system_metrics.buffer_cache_hit_ratio,
                library_cache_hit_ratio: 98.0,
                parse_count_total: 500,
                parse_count_hard: 50,
                execute_count: 5000,
                user_calls: 1000,
                recursive_calls: 4000,
                redo_size_bytes: 1024 * 1024,
                redo_writes: 100,
                sorts_memory: 450,
                sorts_disk: 50,
            },
            top_sql: summary.top_sql.into_iter().map(|s| SqlStatementStats {
                sql_id: s.sql_id,
                sql_text: s.sql_text,
                sql_hash: s.sql_hash,
                executions: s.executions,
                elapsed_time_micros: s.total_elapsed_time_micros,
                cpu_time_micros: s.total_cpu_time_micros,
                buffer_gets: s.total_logical_reads,
                disk_reads: s.total_physical_reads,
                rows_processed: s.total_rows_processed,
                parse_calls: 0,
                sorts: 0,
                fetches: 0,
                px_servers_executions: 0,
                elapsed_time_per_exec_micros: s.avg_elapsed_time_micros,
                cpu_time_per_exec_micros: s.avg_cpu_time_micros,
                module: s.module,
                action: s.action,
            }).collect(),
            session_stats: summary.top_sessions.into_iter().map(|s| SessionStats {
                session_id: s.session_id,
                user_name: s.user_name,
                program: s.program,
                machine: s.machine,
                status: s.status,
                logical_reads: s.logical_reads,
                physical_reads: s.physical_reads,
                cpu_time_micros: s.cpu_time_micros,
                elapsed_time_micros: s.wait_time_micros + s.cpu_time_micros,
                active_time_micros: s.cpu_time_micros,
                current_sql_id: s.sql_id,
                wait_class: s.wait_class,
                wait_event: s.wait_event,
            }).collect(),
            wait_events: summary.top_wait_events.into_iter().map(|w| WaitEventStats {
                wait_class: w.wait_class,
                wait_event: w.wait_event,
                total_waits: w.total_waits,
                total_wait_time_micros: w.total_wait_time_micros,
                avg_wait_time_micros: w.avg_wait_time_micros,
                time_waited_pct: w.time_waited_pct,
            }).collect(),
            io_stats: IoStatistics {
                datafile_reads: 5000,
                datafile_writes: 2000,
                datafile_read_bytes: 5000 * 8192,
                datafile_write_bytes: 2000 * 8192,
                tempfile_reads: 100,
                tempfile_writes: 100,
                redo_writes: 100,
                redo_write_bytes: 1024 * 1024,
                avg_read_time_ms: 2.5,
                avg_write_time_ms: 5.0,
                iops: 1000.0,
                throughput_mbps: 50.0,
            },
            memory_stats: MemoryStatistics {
                sga_size_bytes: summary.memory_usage.sga_bytes,
                pga_size_bytes: summary.memory_usage.pga_bytes,
                shared_pool_size_bytes: summary.memory_usage.shared_pool_bytes,
                buffer_cache_size_bytes: summary.memory_usage.buffer_cache_bytes,
                log_buffer_size_bytes: summary.memory_usage.log_buffer_bytes,
                pga_aggregate_target_bytes: 256 * 1024 * 1024,
                pga_aggregate_allocated_bytes: 200 * 1024 * 1024,
                pga_aggregate_auto_target_bytes: 256 * 1024 * 1024,
                process_count: 100,
                session_count: summary.system_metrics.active_sessions,
            },
            time_model: TimeModelStats {
                db_time_micros: 3600 * 1_000_000,
                db_cpu_time_micros: 1800 * 1_000_000,
                background_cpu_time_micros: 100 * 1_000_000,
                connection_mgmt_time_micros: 10 * 1_000_000,
                parse_time_elapsed_micros: 50 * 1_000_000,
                hard_parse_time_micros: 25 * 1_000_000,
                sql_execute_time_micros: 3500 * 1_000_000,
                pl_sql_execution_time_micros: 100 * 1_000_000,
                pl_sql_compilation_time_micros: 10 * 1_000_000,
            },
            load_profile: LoadProfile {
                transactions_per_second: summary.system_metrics.transactions_per_second,
                commits_per_second: summary.system_metrics.transactions_per_second * 0.9_f64,
                rollbacks_per_second: summary.system_metrics.transactions_per_second * 0.1_f64,
                physical_reads_per_second: 50.0,
                physical_writes_per_second: 20.0,
                logical_reads_per_second: 1000.0,
                redo_size_per_second_bytes: 10240.0,
                parses_per_second: 5.0,
                hard_parses_per_second: 0.5,
                executes_per_second: summary.system_metrics.queries_per_second,
                user_calls_per_second: 20.0,
            },
            os_stats: OsStatistics {
                cpu_count: 8,
                cpu_usage_pct: summary.system_metrics.cpu_usage_pct,
                load_average: 2.5,
                physical_memory_bytes: summary.memory_usage.total_memory_bytes,
                free_memory_bytes: summary.memory_usage.free_memory_bytes,
                swap_total_bytes: 8 * 1024 * 1024 * 1024,
                swap_free_bytes: 7 * 1024 * 1024 * 1024,
                network_rx_bytes_per_sec: 1024.0 * 1024.0,
                network_tx_bytes_per_sec: 512.0 * 1024.0,
            },
            tablespace_usage: vec![],
            segment_stats: vec![],
        }
    }

    // Run automatic diagnostic analysis
    pub fn run_diagnostic_analysis(
        &self,
        start_snapshot_id: u64,
        end_snapshot_id: u64,
    ) -> Result<AnalysisId> {
        let analysis_id = self.advisor.create_analysis(
            format!("Auto Analysis {}:{}", start_snapshot_id, end_snapshot_id),
            start_snapshot_id,
            end_snapshot_id,
            AnalysisScope::Database,
        )?;

        self.advisor.execute_analysis(analysis_id)?;

        Ok(analysis_id)
    }

    // Get comprehensive performance report
    pub fn get_performance_report(&self) -> PerformanceReport {
        PerformanceReport {
            timestamp: SystemTime::now(),
            repository_stats: self.repository.get_repository_stats(),
            performance_summary: self.performance_hub.get_performance_summary(),
            active_executions: self.sql_monitor.get_active_executions().len(),
            execution_stats: self.sql_monitor.get_execution_statistics(),
        }
    }
}

impl Default for WorkloadIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub timestamp: SystemTime,
    pub repository_stats: RepositoryStats,
    pub performance_summary: PerformanceSummary,
    pub active_executions: usize,
    pub execution_stats: ExecutionStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_intelligence_creation() {
        let wi = WorkloadIntelligence::new();
        let report = wi.get_performance_report();
        assert!(report.timestamp <= SystemTime::now());
    }

    #[test]
    fn test_snapshot_capture() {
        let wi = WorkloadIntelligence::new();
        let snapshot_id = wi.capture_snapshot().unwrap();
        assert_eq!(snapshot_id, 1);

        let snapshot = wi.repository.get_snapshot(snapshot_id);
        assert!(snapshot.is_some());
    }

    #[test]
    fn test_diagnostic_analysis() {
        let wi = WorkloadIntelligence::new();

        // Capture two snapshots
        let snap1 = wi.capture_snapshot().unwrap();
        let snap2 = wi.capture_snapshot().unwrap();

        // Run analysis
        let analysis_id = wi.run_diagnostic_analysis(snap1, snap2).unwrap();
        assert_eq!(analysis_id, 1);

        let findings = wi.advisor.get_findings(analysis_id);
        assert!(findings.is_some());
    }
}
