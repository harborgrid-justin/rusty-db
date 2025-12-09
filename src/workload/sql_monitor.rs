// RustyDB SQL Monitor - Real-time SQL execution monitoring
// Provides real-time monitoring of SQL execution with plan tracking and wait analysis

use std::time::SystemTime;
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use crate::Result;
use crate::error::DbError;

/// Real-time SQL Monitor for tracking query execution
pub struct SqlMonitor {
    /// Active executions being monitored
    active_executions: Arc<RwLock<HashMap<ExecutionId, SqlExecution>>>,

    /// Execution history (ring buffer)
    execution_history: Arc<RwLock<VecDeque<SqlExecution>>>,

    /// Execution plans cache
    plan_cache: Arc<RwLock<HashMap<u64, ExecutionPlan>>>,

    /// Performance alerts
    alerts: Arc<RwLock<Vec<PerformanceAlert>>>,

    /// Configuration
    config: Arc<RwLock<MonitorConfig>>,

    /// Next execution ID
    next_execution_id: Arc<RwLock<ExecutionId>>,
}

/// Unique identifier for a SQL execution
pub type ExecutionId = u64;

/// SQL Monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Maximum number of active executions to track
    pub max_active_executions: usize,

    /// History size (number of completed executions to keep)
    pub history_size: usize,

    /// Long-running query threshold (seconds)
    pub long_running_threshold_secs: u64,

    /// Enable automatic plan capture
    pub auto_plan_capture: bool,

    /// Enable wait event tracking
    pub track_wait_events: bool,

    /// Enable parallel execution monitoring
    pub track_parallel_execution: bool,

    /// Sampling interval for active executions (milliseconds)
    pub sampling_interval_ms: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            max_active_executions: 1000,
            history_size: 10000,
            long_running_threshold_secs: 60,
            auto_plan_capture: true,
            track_wait_events: true,
            track_parallel_execution: true,
            sampling_interval_ms: 100,
        }
    }
}

/// SQL execution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlExecution {
    pub execution_id: ExecutionId,
    pub sql_id: String,
    pub sql_text: String,
    pub sql_hash: u64,
    pub plan_hash: u64,

    /// Session information
    pub session_id: u64,
    pub user_name: String,
    pub program: String,
    pub module: Option<String>,
    pub action: Option<String>,

    /// Timing information
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub elapsed_time: Duration,
    pub cpu_time: Duration,
    pub wait_time: Duration,

    /// Execution status
    pub status: ExecutionStatus,
    pub current_operation: Option<String>,
    pub progress_pct: f64,

    /// Resource consumption
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub physical_writes: u64,
    pub temp_space_allocated: u64,
    pub temp_space_used: u64,
    pub memory_used_bytes: u64,
    pub rows_processed: u64,

    /// Wait events
    pub wait_events: Vec<WaitEventDetail>,
    pub current_wait: Option<WaitEventDetail>,

    /// Execution plan
    pub plan_operations: Vec<PlanOperationMetrics>,

    /// Parallel execution
    pub parallel_info: Option<ParallelExecutionInfo>,

    /// Bind variables
    pub bind_variables: Vec<BindVariable>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Currently executing
    Executing,
    /// Waiting for resource
    Waiting,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

/// Wait event detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEventDetail {
    pub wait_class: String,
    pub wait_event: String,
    pub wait_time_micros: u64,
    pub wait_count: u64,
    pub p1: u64,
    pub p2: u64,
    pub p3: u64,
    pub timestamp: SystemTime,
}

/// Plan operation metrics (real-time metrics for each plan step)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOperationMetrics {
    pub operation_id: u32,
    pub operation_name: String,
    pub object_name: Option<String>,

    /// Actual metrics
    pub actual_rows: u64,
    pub actual_time_micros: u64,
    pub actual_memory_bytes: u64,
    pub actual_temp_bytes: u64,

    /// Estimated metrics
    pub estimated_rows: u64,
    pub estimated_cost: f64,

    /// Comparison
    pub cardinality_error_ratio: f64,

    /// Current state
    pub starts: u64,
    pub first_row_time: Option<SystemTime>,
    pub last_row_time: Option<SystemTime>,
}

/// Parallel execution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionInfo {
    pub coordinator_session_id: u64,
    pub degree_of_parallelism: u32,
    pub requested_dop: u32,
    pub actual_dop: u32,
    pub server_set: Vec<ParallelServer>,
    pub downgrade_reason: Option<String>,
}

/// Parallel server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelServer {
    pub server_id: u32,
    pub session_id: u64,
    pub status: String,
    pub cpu_time_micros: u64,
    pub wait_time_micros: u64,
    pub rows_processed: u64,
}

/// Bind variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindVariable {
    pub position: u32,
    pub name: Option<String>,
    pub data_type: String,
    pub value: String,
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_hash: u64,
    pub sql_id: String,
    pub timestamp: SystemTime,
    pub operations: Vec<PlanOperation>,
    pub optimizer_mode: String,
    pub optimizer_cost: f64,
}

/// Plan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOperation {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub operation: String,
    pub options: Option<String>,
    pub object_owner: Option<String>,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub depth: u32,
    pub position: u32,
    pub cost: f64,
    pub cardinality: u64,
    pub bytes: u64,
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub access_predicates: Option<String>,
    pub filter_predicates: Option<String>,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub alert_id: u64,
    pub execution_id: ExecutionId,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: SystemTime,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    LongRunningQuery,
    ExcessiveWaits,
    HighMemoryUsage,
    ExcessiveTempSpace,
    CardinalityMismatch,
    PlanChange,
    ResourceContention,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Execution statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    pub total_executions: u64,
    pub active_executions: usize,
    pub completed_executions: u64,
    pub failed_executions: u64,
    pub avg_elapsed_time_ms: f64,
    pub avg_cpu_time_ms: f64,
    pub avg_wait_time_ms: f64,
    pub avg_rows_processed: u64,
    pub total_logical_reads: u64,
    pub total_physical_reads: u64,
}

impl SqlMonitor {
    /// Create a new SQL Monitor
    pub fn new() -> Self {
        Self::with_config(MonitorConfig::default())
    }

    /// Create a new SQL Monitor with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::new())),
            plan_cache: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(RwLock::new(config)),
            next_execution_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Start monitoring a SQL execution
    pub fn start_execution(
        &self,
        sql_text: String,
        session_id: u64,
        user_name: String,
        program: String,
    ) -> Result<ExecutionId> {
        let mut active = self.active_executions.write();
        let mut next_id = self.next_execution_id.write();

        let execution_id = *next_id;
        *next_id += 1;

        let sql_id = self.compute_sql_id(&sql_text);
        let sql_hash = self.compute_hash(&sql_text);
        let plan_hash = sql_hash; // Simplified

        let execution = SqlExecution {
            execution_id,
            sql_id,
            sql_text,
            sql_hash,
            plan_hash,
            session_id,
            user_name,
            program,
            module: None,
            action: None,
            start_time: SystemTime::now(),
            end_time: None,
            elapsed_time: Duration::from_secs(0),
            cpu_time: Duration::from_secs(0),
            wait_time: Duration::from_secs(0),
            status: ExecutionStatus::Executing,
            current_operation: None,
            progress_pct: 0.0,
            logical_reads: 0,
            physical_reads: 0,
            physical_writes: 0,
            temp_space_allocated: 0,
            temp_space_used: 0,
            memory_used_bytes: 0,
            rows_processed: 0,
            wait_events: Vec::new(),
            current_wait: None,
            plan_operations: Vec::new(),
            parallel_info: None,
            bind_variables: Vec::new(),
        };

        active.insert(execution_id, execution);

        Ok(execution_id)
    }

    /// Update execution metrics
    pub fn update_execution_metrics(
        &self,
        execution_id: ExecutionId,
        cpu_time: Duration,
        wait_time: Duration,
        logical_reads: u64,
        physical_reads: u64,
        rows_processed: u64,
    ) -> Result<()> {
        let (elapsed_secs, should_alert) = {
            let mut active = self.active_executions.write();

            let execution = active
                .get_mut(&execution_id)
                .ok_or_else(|| DbError::NotFound(format!("Execution {} not found", execution_id)))?);

            let now = SystemTime::now();
            execution.elapsed_time = now.duration_since(execution.start_time).unwrap_or_default();
            execution.cpu_time = cpu_time;
            execution.wait_time = wait_time;
            execution.logical_reads = logical_reads;
            execution.physical_reads = physical_reads;
            execution.rows_processed = rows_processed;

            let elapsed = execution.elapsed_time.as_secs();
            let config = self.config.read();
            let should_alert = elapsed > config.long_running_threshold_secs;

            (elapsed, should_alert)
        };

        // Check for long-running query
        if should_alert {
            self.raise_alert(
                execution_id,
                AlertType::LongRunningQuery,
                AlertSeverity::Warning,
                format!("Query has been running for {} seconds", elapsed_secs),
            )?);
        }

        Ok(())
    }

    /// Record a wait event
    pub fn record_wait_event(
        &self,
        execution_id: ExecutionId,
        wait_class: String,
        wait_event: String,
        wait_time_micros: u64,
    ) -> Result<()> {
        let mut active = self.active_executions.write();

        let execution = active
            .get_mut(&execution_id)
            .ok_or_else(|| DbError::NotFound(format!("Execution {} not found", execution_id)))?);

        let wait_detail = WaitEventDetail {
            wait_class: wait_class.clone(),
            wait_event: wait_event.clone(),
            wait_time_micros,
            wait_count: 1,
            p1: 0,
            p2: 0,
            p3: 0,
            timestamp: SystemTime::now(),
        };

        execution.current_wait = Some(wait_detail.clone());
        execution.wait_events.push(wait_detail);

        Ok(())
    }

    /// Update plan operation metrics
    pub fn update_plan_operation(
        &self,
        execution_id: ExecutionId,
        operation_id: u32,
        actual_rows: u64,
        actual_time_micros: u64,
        starts: u64,
    ) -> Result<()> {
        let mut active = self.active_executions.write();

        let execution = active
            .get_mut(&execution_id)
            .ok_or_else(|| DbError::NotFound(format!("Execution {} not found", execution_id)))?);

        // Find or create operation metrics
        if let Some(op) = execution.plan_operations.iter_mut().find(|o| o.operation_id == operation_id) {
            op.actual_rows = actual_rows;
            op.actual_time_micros = actual_time_micros;
            op.starts = starts;
            op.last_row_time = Some(SystemTime::now());

            // Calculate cardinality error
            if op.estimated_rows > 0 {
                op.cardinality_error_ratio = actual_rows as f64 / op.estimated_rows as f64;
            }
        }

        Ok(())
    }

    /// Complete an execution
    pub fn complete_execution(&self, execution_id: ExecutionId, status: ExecutionStatus) -> Result<()> {
        let mut execution = {
            let mut active = self.active_executions.write();
            active.remove(&execution_id)
        };

        if let Some(ref mut exec) = execution {
            exec.end_time = Some(SystemTime::now());
            exec.status = status;

            let now = SystemTime::now();
            exec.elapsed_time = now.duration_since(exec.start_time).unwrap_or_default();

            // Add to history
            let mut history = self.execution_history.write();
            let config = self.config.read();

            if history.len() >= config.history_size {
                history.pop_front();
            }
            history.push_back(exec.clone());

            Ok(())
        } else {
            Err(DbError::NotFound(format!("Execution {} not found", execution_id)))
        }
    }

    /// Get active execution details
    pub fn get_active_execution(&self, execution_id: ExecutionId) -> Option<SqlExecution> {
        self.active_executions.read().get(&execution_id).cloned()
    }

    /// Get all active executions
    pub fn get_active_executions(&self) -> Vec<SqlExecution> {
        self.active_executions.read().values().cloned().collect()
    }

    /// Get execution from history
    pub fn get_execution_from_history(&self, execution_id: ExecutionId) -> Option<SqlExecution> {
        self.execution_history
            .read()
            .iter()
            .find(|e| e.execution_id == execution_id)
            .cloned()
    }

    /// Get long-running queries
    pub fn get_long_running_queries(&self) -> Vec<SqlExecution> {
        let config = self.config.read());
        let threshold = Duration::from_secs(config.long_running_threshold_secs);

        self.active_executions
            .read()
            .values()
            .filter(|e| e.elapsed_time > threshold)
            .cloned()
            .collect()
    }

    /// Get top queries by elapsed time
    pub fn get_top_queries_by_elapsed_time(&self, limit: usize) -> Vec<SqlExecution> {
        let mut executions: Vec<SqlExecution> = self.execution_history.read().iter().cloned().collect();
        executions.sort_by(|a, b| b.elapsed_time.cmp(&a.elapsed_time));
        executions.into_iter().take(limit).collect()
    }

    /// Get top queries by CPU time
    pub fn get_top_queries_by_cpu_time(&self, limit: usize) -> Vec<SqlExecution> {
        let mut executions: Vec<SqlExecution> = self.execution_history.read().iter().cloned().collect();
        executions.sort_by(|a, b| b.cpu_time.cmp(&a.cpu_time));
        executions.into_iter().take(limit).collect()
    }

    /// Get top queries by I/O
    pub fn get_top_queries_by_io(&self, limit: usize) -> Vec<SqlExecution> {
        let mut executions: Vec<SqlExecution> = self.execution_history.read().iter().cloned().collect();
        executions.sort_by(|a, b| b.physical_reads.cmp(&a.physical_reads));
        executions.into_iter().take(limit).collect()
    }

    /// Get execution statistics
    pub fn get_execution_statistics(&self) -> ExecutionStatistics {
        let active = self.active_executions.read();
        let history = self.execution_history.read();

        let total = history.len() as u64;
        let mut completed = 0u64;
        let mut failed = 0u64;
        let mut total_elapsed = 0f64;
        let mut total_cpu = 0f64;
        let mut total_wait = 0f64;
        let mut total_rows = 0u64;
        let mut total_logical_reads = 0u64;
        let mut total_physical_reads = 0u64;

        for exec in history.iter() {
            match exec.status {
                ExecutionStatus::Completed => completed += 1,
                ExecutionStatus::Failed => failed += 1,
                _ => {}
            }

            total_elapsed += exec.elapsed_time.as_secs_f64() * 1000.0;
            total_cpu += exec.cpu_time.as_secs_f64() * 1000.0;
            total_wait += exec.wait_time.as_secs_f64() * 1000.0;
            total_rows += exec.rows_processed;
            total_logical_reads += exec.logical_reads;
            total_physical_reads += exec.physical_reads;
        }

        let count = total.max(1) as f64;

        ExecutionStatistics {
            total_executions: total,
            active_executions: active.len(),
            completed_executions: completed,
            failed_executions: failed,
            avg_elapsed_time_ms: total_elapsed / count,
            avg_cpu_time_ms: total_cpu / count,
            avg_wait_time_ms: total_wait / count,
            avg_rows_processed: (total_rows as f64 / count) as u64,
            total_logical_reads,
            total_physical_reads,
        }
    }

    /// Raise a performance alert
    fn raise_alert(
        &self,
        execution_id: ExecutionId,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
    ) -> Result<()> {
        let mut alerts = self.alerts.write();

        let alert_id = alerts.len() as u64 + 1;

        alerts.push(PerformanceAlert {
            alert_id,
            execution_id,
            alert_type,
            severity,
            message,
            timestamp: SystemTime::now(),
        });

        Ok(())
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<PerformanceAlert> {
        self.alerts.read().clone()
    }

    /// Clear alerts
    pub fn clear_alerts(&self) {
        self.alerts.write().clear();
    }

    /// Compute SQL ID
    fn compute_sql_id(&self, sql_text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql_text.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Compute hash
    fn compute_hash(&self, text: &str) -> u64 {

        let mut hasher = DefaultHasher::new());
        text.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for SqlMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_execution() {
        let monitor = SqlMonitor::new();
        let exec_id = monitor
            .start_execution(
                "SELECT * FROM users".to_string(),
                1,
                "test_user".to_string(),
                "test_program".to_string(),
            )
            .unwrap();

        assert_eq!(exec_id, 1);

        let execution = monitor.get_active_execution(exec_id);
        assert!(execution.is_some());
        assert_eq!(execution.unwrap().status, ExecutionStatus::Executing);
    }

    #[test]
    fn test_complete_execution() {
        let monitor = SqlMonitor::new();
        let exec_id = monitor
            .start_execution(
                "SELECT * FROM users".to_string(),
                1,
                "test_user".to_string(),
                "test_program".to_string(),
            )
            .unwrap();

        monitor.complete_execution(exec_id, ExecutionStatus::Completed).unwrap();

        let active = monitor.get_active_execution(exec_id);
        assert!(active.is_none());

        let from_history = monitor.get_execution_from_history(exec_id);
        assert!(from_history.is_some());
    }

    #[test]
    fn test_execution_statistics() {
        let monitor = SqlMonitor::new();

        for i in 0..10 {
            let exec_id = monitor
                .start_execution(
                    format!("SELECT * FROM users WHERE id = {}", i),
                    i,
                    "test_user".to_string(),
                    "test_program".to_string(),
                )
                .unwrap());

            monitor.complete_execution(exec_id, ExecutionStatus::Completed).unwrap();
        }

        let stats = monitor.get_execution_statistics();
        assert_eq!(stats.completed_executions, 10);
        assert_eq!(stats.active_executions, 0);
    }
}


