// RustyDB Performance Hub - Unified performance dashboard and analysis
// Provides comprehensive performance views and drill-down capabilities

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use crate::Result;

/// Performance Hub - unified performance monitoring and analysis
pub struct PerformanceHub {
    /// Top SQL tracking
    top_sql_tracker: Arc<RwLock<TopSqlTracker>>,

    /// Session activity tracking
    session_tracker: Arc<RwLock<SessionTracker>>,

    /// Wait event analyzer
    wait_analyzer: Arc<RwLock<WaitEventAnalyzer>>,

    /// I/O analyzer
    io_analyzer: Arc<RwLock<IoAnalyzer>>,

    /// Memory analyzer
    memory_analyzer: Arc<RwLock<MemoryAnalyzer>>,

    /// Performance trends
    trend_tracker: Arc<RwLock<TrendTracker>>,

    /// Configuration
    config: Arc<RwLock<PerformanceHubConfig>>,
}

/// Performance Hub configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHubConfig {
    /// Number of top SQL statements to track
    pub top_sql_count: usize,

    /// Number of top sessions to track
    pub top_session_count: usize,

    /// Retention period for performance data (seconds)
    pub retention_period_secs: u64,

    /// Sampling interval (seconds)
    pub sampling_interval_secs: u64,

    /// Enable automatic trend detection
    pub auto_trend_detection: bool,
}

impl Default for PerformanceHubConfig {
    fn default() -> Self {
        Self {
            top_sql_count: 100,
            top_session_count: 50,
            retention_period_secs: 86400, // 24 hours
            sampling_interval_secs: 60,   // 1 minute
            auto_trend_detection: true,
        }
    }
}

/// Top SQL tracker
#[derive(Debug)]
struct TopSqlTracker {
    /// SQL statements by elapsed time
    by_elapsed_time: BTreeMap<u64, SqlStats>,
    /// SQL statements by CPU time
    by_cpu_time: BTreeMap<u64, SqlStats>,
    /// SQL statements by I/O
    by_io: BTreeMap<u64, SqlStats>,
    /// SQL statements by executions
    by_executions: BTreeMap<u64, SqlStats>,
    /// SQL lookup by ID
    sql_lookup: HashMap<String, SqlStats>,
}

impl Default for TopSqlTracker {
    fn default() -> Self {
        Self {
            by_elapsed_time: BTreeMap::new(),
            by_cpu_time: BTreeMap::new(),
            by_io: BTreeMap::new(),
            by_executions: BTreeMap::new(),
            sql_lookup: HashMap::new(),
        }
    }
}

/// SQL statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlStats {
    pub sql_id: String,
    pub sql_text: String,
    pub sql_hash: u64,
    pub plan_hash: u64,
    pub executions: u64,
    pub total_elapsed_time_micros: u64,
    pub total_cpu_time_micros: u64,
    pub total_wait_time_micros: u64,
    pub total_logical_reads: u64,
    pub total_physical_reads: u64,
    pub total_physical_writes: u64,
    pub total_rows_processed: u64,
    pub avg_elapsed_time_micros: u64,
    pub avg_cpu_time_micros: u64,
    pub avg_rows_per_exec: u64,
    pub first_load_time: SystemTime,
    pub last_active_time: SystemTime,
    pub module: Option<String>,
    pub action: Option<String>,
}

/// Session tracker
#[derive(Debug, Default)]
struct SessionTracker {
    active_sessions: HashMap<u64, SessionActivity>,
    session_history: VecDeque<SessionActivity>,
}

/// Session activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionActivity {
    pub session_id: u64,
    pub user_name: String,
    pub program: String,
    pub machine: String,
    pub status: String,
    pub state: SessionState,
    pub sql_id: Option<String>,
    pub sql_text: Option<String>,
    pub wait_class: Option<String>,
    pub wait_event: Option<String>,
    pub cpu_time_micros: u64,
    pub wait_time_micros: u64,
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub temp_space_bytes: u64,
    pub last_call_elapsed: Duration,
    pub logon_time: SystemTime,
    pub last_activity_time: SystemTime,
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Active,
    Waiting,
    Idle,
    Killed,
}

/// Wait event analyzer
#[derive(Debug, Default)]
struct WaitEventAnalyzer {
    wait_events: HashMap<String, WaitEventMetrics>,
    wait_history: VecDeque<WaitEventSample>,
}

/// Wait event metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEventMetrics {
    pub wait_class: String,
    pub wait_event: String,
    pub total_waits: u64,
    pub total_wait_time_micros: u64,
    pub avg_wait_time_micros: u64,
    pub max_wait_time_micros: u64,
    pub time_waited_pct: f64,
    pub sessions_affected: u64,
}

/// Wait event sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEventSample {
    pub timestamp: SystemTime,
    pub wait_class: String,
    pub wait_event: String,
    pub wait_time_micros: u64,
    pub session_id: u64,
}

/// I/O analyzer
#[derive(Debug, Default)]
struct IoAnalyzer {
    file_stats: HashMap<String, FileIoStats>,
    io_history: VecDeque<IoSample>,
}

/// File I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIoStats {
    pub file_name: String,
    pub file_type: FileType,
    pub reads: u64,
    pub writes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_time_micros: u64,
    pub write_time_micros: u64,
    pub avg_read_time_ms: f64,
    pub avg_write_time_ms: f64,
}

/// File type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    DataFile,
    TempFile,
    LogFile,
    ControlFile,
}

/// I/O sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoSample {
    pub timestamp: SystemTime,
    pub file_name: String,
    pub operation: IoOperation,
    pub bytes: u64,
    pub duration_micros: u64,
}

/// I/O operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoOperation {
    Read,
    Write,
}

/// Memory analyzer
#[derive(Debug, Default)]
struct MemoryAnalyzer {
    current_usage: MemoryUsage,
    usage_history: VecDeque<MemoryUsage>,
    component_breakdown: HashMap<String, u64>,
}

/// Memory usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub timestamp: SystemTime,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub sga_bytes: u64,
    pub pga_bytes: u64,
    pub shared_pool_bytes: u64,
    pub buffer_cache_bytes: u64,
    pub log_buffer_bytes: u64,
    pub usage_pct: f64,
}

impl Default for MemoryUsage {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            total_memory_bytes: 0,
            used_memory_bytes: 0,
            free_memory_bytes: 0,
            sga_bytes: 0,
            pga_bytes: 0,
            shared_pool_bytes: 0,
            buffer_cache_bytes: 0,
            log_buffer_bytes: 0,
            usage_pct: 0.0,
        }
    }
}

/// Trend tracker
#[derive(Debug, Default)]
struct TrendTracker {
    cpu_trend: VecDeque<TrendDataPoint>,
    io_trend: VecDeque<TrendDataPoint>,
    memory_trend: VecDeque<TrendDataPoint>,
    throughput_trend: VecDeque<TrendDataPoint>,
}

/// Trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub moving_avg: f64,
    pub std_dev: f64,
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub timestamp: SystemTime,
    pub top_sql: Vec<SqlStats>,
    pub top_sessions: Vec<SessionActivity>,
    pub top_wait_events: Vec<WaitEventMetrics>,
    pub top_io_files: Vec<FileIoStats>,
    pub memory_usage: MemoryUsage,
    pub system_metrics: SystemMetrics,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_pct: f64,
    pub active_sessions: u32,
    pub queries_per_second: f64,
    pub transactions_per_second: f64,
    pub buffer_cache_hit_ratio: f64,
    pub average_active_sessions: f64,
    pub db_time_per_second: f64,
}

impl PerformanceHub {
    /// Create a new Performance Hub
    pub fn new() -> Self {
        Self::with_config(PerformanceHubConfig::default())
    }

    /// Create a new Performance Hub with custom configuration
    pub fn with_config(config: PerformanceHubConfig) -> Self {
        Self {
            top_sql_tracker: Arc::new(RwLock::new(TopSqlTracker::default())),
            session_tracker: Arc::new(RwLock::new(SessionTracker::default())),
            wait_analyzer: Arc::new(RwLock::new(WaitEventAnalyzer::default())),
            io_analyzer: Arc::new(RwLock::new(IoAnalyzer::default())),
            memory_analyzer: Arc::new(RwLock::new(MemoryAnalyzer::default())),
            trend_tracker: Arc::new(RwLock::new(TrendTracker::default())),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Record SQL execution
    pub fn record_sql_execution(
        &self,
        sql_id: String,
        sql_text: String,
        elapsed_time: Duration,
        cpu_time: Duration,
        logical_reads: u64,
        physical_reads: u64,
        rows_processed: u64,
    ) {
        let elapsed_micros = elapsed_time.as_micros() as u64;
        let cpu_micros = cpu_time.as_micros() as u64;

        let mut tracker = self.top_sql_tracker.write();

        let _stats = tracker.sql_lookup.entry(sql_id.clone()).or_insert_with(|| {
            SqlStats {
                sql_id: sql_id.clone(),
                sql_text: sql_text.clone(),
                sql_hash: 0,
                plan_hash: 0,
                executions: 0,
                total_elapsed_time_micros: 0,
                total_cpu_time_micros: 0,
                total_wait_time_micros: 0,
                total_logical_reads: 0,
                total_physical_reads: 0,
                total_physical_writes: 0,
                total_rows_processed: 0,
                avg_elapsed_time_micros: 0,
                avg_cpu_time_micros: 0,
                avg_rows_per_exec: 0,
                first_load_time: SystemTime::now(),
                last_active_time: SystemTime::now(),
                module: None,
                action: None,
            }
        });

        stats.executions += 1;
        stats.total_elapsed_time_micros += elapsed_micros;
        stats.total_cpu_time_micros += cpu_micros;
        stats.total_logical_reads += logical_reads;
        stats.total_physical_reads += physical_reads;
        stats.total_rows_processed += rows_processed;
        stats.avg_elapsed_time_micros = stats.total_elapsed_time_micros / stats.executions;
        stats.avg_cpu_time_micros = stats.total_cpu_time_micros / stats.executions;
        stats.avg_rows_per_exec = stats.total_rows_processed / stats.executions;
        stats.last_active_time = SystemTime::now();

        // Clone stats before updating indexes
        let stats_clone = stats.clone();

        // Update indexes
        tracker.by_elapsed_time.insert(stats_clone.total_elapsed_time_micros, stats_clone.clone());
        tracker.by_cpu_time.insert(stats_clone.total_cpu_time_micros, stats_clone.clone());
        tracker.by_io.insert(stats_clone.total_physical_reads, stats_clone.clone());
        tracker.by_executions.insert(stats_clone.executions, stats_clone);
    }

    /// Update session activity
    pub fn update_session(
        &self,
        session_id: u64,
        user_name: String,
        program: String,
        state: SessionState,
        cpu_time: Duration,
        logical_reads: u64,
    ) {
        let mut tracker = self.session_tracker.write();

        let activity = tracker.active_sessions.entry(session_id).or_insert_with(|| {
            SessionActivity {
                session_id,
                user_name: user_name.clone(),
                program: program.clone(),
                machine: "localhost".to_string(),
                status: "ACTIVE".to_string(),
                state: state.clone(),
                sql_id: None,
                sql_text: None,
                wait_class: None,
                wait_event: None,
                cpu_time_micros: 0,
                wait_time_micros: 0,
                logical_reads: 0,
                physical_reads: 0,
                temp_space_bytes: 0,
                last_call_elapsed: Duration::from_secs(0),
                logon_time: SystemTime::now(),
                last_activity_time: SystemTime::now(),
            }
        });

        activity.state = state;
        activity.cpu_time_micros = cpu_time.as_micros() as u64;
        activity.logical_reads = logical_reads;
        activity.last_activity_time = SystemTime::now();
    }

    /// Record wait event
    pub fn record_wait_event(
        &self,
        session_id: u64,
        wait_class: String,
        wait_event: String,
        wait_time: Duration,
    ) {
        let mut analyzer = self.wait_analyzer.write();

        let key = format!("{}:{}", wait_class, wait_event);
        let wait_micros = wait_time.as_micros() as u64;

        let metrics = analyzer.wait_events.entry(key).or_insert_with(|| {
            WaitEventMetrics {
                wait_class: wait_class.clone(),
                wait_event: wait_event.clone(),
                total_waits: 0,
                total_wait_time_micros: 0,
                avg_wait_time_micros: 0,
                max_wait_time_micros: 0,
                time_waited_pct: 0.0,
                sessions_affected: 0,
            }
        });

        metrics.total_waits += 1;
        metrics.total_wait_time_micros += wait_micros;
        metrics.avg_wait_time_micros = metrics.total_wait_time_micros / metrics.total_waits;
        metrics.max_wait_time_micros = metrics.max_wait_time_micros.max(wait_micros);

        let sample = WaitEventSample {
            timestamp: SystemTime::now(),
            wait_class,
            wait_event,
            wait_time_micros: wait_micros,
            session_id,
        };

        analyzer.wait_history.push_back(sample);
    }

    /// Record I/O operation
    pub fn record_io_operation(
        &self,
        file_name: String,
        file_type: FileType,
        operation: IoOperation,
        bytes: u64,
        duration: Duration,
    ) {
        let mut analyzer = self.io_analyzer.write();

        let duration_micros = duration.as_micros() as u64;

        let _stats = analyzer.file_stats.entry(file_name.clone()).or_insert_with(|| {
            FileIoStats {
                file_name: file_name.clone(),
                file_type: file_type.clone(),
                reads: 0,
                writes: 0,
                read_bytes: 0,
                write_bytes: 0,
                read_time_micros: 0,
                write_time_micros: 0,
                avg_read_time_ms: 0.0,
                avg_write_time_ms: 0.0,
            }
        });

        match operation {
            IoOperation::Read => {
                stats.reads += 1;
                stats.read_bytes += bytes;
                stats.read_time_micros += duration_micros;
                stats.avg_read_time_ms = (stats.read_time_micros / stats.reads.max(1)) as f64 / 1000.0;
            }
            IoOperation::Write => {
                stats.writes += 1;
                stats.write_bytes += bytes;
                stats.write_time_micros += duration_micros;
                stats.avg_write_time_ms = (stats.write_time_micros / stats.writes.max(1)) as f64 / 1000.0;
            }
        }

        let sample = IoSample {
            timestamp: SystemTime::now(),
            file_name,
            operation,
            bytes,
            duration_micros,
        };

        analyzer.io_history.push_back(sample);
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, usage: MemoryUsage) {
        let mut analyzer = self.memory_analyzer.write();
        analyzer.current_usage = usage.clone();
        analyzer.usage_history.push_back(usage);

        // Keep only recent history
        while analyzer.usage_history.len() > 1440 {
            // 24 hours at 1-minute intervals
            analyzer.usage_history.pop_front();
        }
    }

    /// Get top SQL by elapsed time
    pub fn get_top_sql_by_elapsed_time(&self, limit: usize) -> Vec<SqlStats> {
        let tracker = self.top_sql_tracker.read();
        tracker
            .by_elapsed_time
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get top SQL by CPU time
    pub fn get_top_sql_by_cpu_time(&self, limit: usize) -> Vec<SqlStats> {
        let tracker = self.top_sql_tracker.read();
        tracker
            .by_cpu_time
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get top SQL by I/O
    pub fn get_top_sql_by_io(&self, limit: usize) -> Vec<SqlStats> {
        let tracker = self.top_sql_tracker.read();
        tracker
            .by_io
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get top sessions
    pub fn get_top_sessions(&self, limit: usize) -> Vec<SessionActivity> {
        let tracker = self.session_tracker.read();
        let mut sessions: Vec<SessionActivity> = tracker.active_sessions.values().cloned().collect();
        sessions.sort_by(|a, b| b.cpu_time_micros.cmp(&a.cpu_time_micros));
        sessions.into_iter().take(limit).collect()
    }

    /// Get top wait events
    pub fn get_top_wait_events(&self, limit: usize) -> Vec<WaitEventMetrics> {
        let analyzer = self.wait_analyzer.read();
        let mut events: Vec<WaitEventMetrics> = analyzer.wait_events.values().cloned().collect();
        events.sort_by(|a, b| b.total_wait_time_micros.cmp(&a.total_wait_time_micros));
        events.into_iter().take(limit).collect()
    }

    /// Get top I/O files
    pub fn get_top_io_files(&self, limit: usize) -> Vec<FileIoStats> {
        let analyzer = self.io_analyzer.read();
        let mut files: Vec<FileIoStats> = analyzer.file_stats.values().cloned().collect();
        files.sort_by(|a, b| (b.reads + b.writes).cmp(&(a.reads + a.writes)));
        files.into_iter().take(limit).collect()
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> MemoryUsage {
        self.memory_analyzer.read().current_usage.clone()
    }

    /// Get performance summary
    pub fn get_performance_summary(&self) -> PerformanceSummary {
        PerformanceSummary {
            timestamp: SystemTime::now(),
            top_sql: self.get_top_sql_by_elapsed_time(10),
            top_sessions: self.get_top_sessions(10),
            top_wait_events: self.get_top_wait_events(10),
            top_io_files: self.get_top_io_files(10),
            memory_usage: self.get_memory_usage(),
            system_metrics: SystemMetrics {
                cpu_usage_pct: 0.0,
                active_sessions: self.session_tracker.read().active_sessions.len() as u32,
                queries_per_second: 0.0,
                transactions_per_second: 0.0,
                buffer_cache_hit_ratio: 0.0,
                average_active_sessions: 0.0,
                db_time_per_second: 0.0,
            },
        }
    }

    /// Get wait event analysis for a time period
    pub fn get_wait_event_analysis(&self, start_time: SystemTime, end_time: SystemTime) -> Vec<WaitEventMetrics> {
        let analyzer = self.wait_analyzer.read();
        analyzer.wait_events.values().cloned().collect()
    }

    /// Get I/O analysis for a time period
    pub fn get_io_analysis(&self, start_time: SystemTime, end_time: SystemTime) -> Vec<FileIoStats> {
        let analyzer = self.io_analyzer.read();
        analyzer.file_stats.values().cloned().collect()
    }
}

impl Default for PerformanceHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_sql_execution() {
        let hub = PerformanceHub::new();
        hub.record_sql_execution(
            "sql1".to_string(),
            "SELECT * FROM users".to_string(),
            Duration::from_millis(100),
            Duration::from_millis(50),
            1000,
            100,
            50,
        );

        let top_sql = hub.get_top_sql_by_elapsed_time(5);
        assert_eq!(top_sql.len(), 1);
        assert_eq!(top_sql[0].executions, 1);
    }

    #[test]
    fn test_session_tracking() {
        let hub = PerformanceHub::new();
        hub.update_session(
            1,
            "test_user".to_string(),
            "test_program".to_string(),
            SessionState::Active,
            Duration::from_secs(5),
            1000,
        );

        let sessions = hub.get_top_sessions(10);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, 1);
    }

    #[test]
    fn test_wait_event_recording() {
        let hub = PerformanceHub::new();
        hub.record_wait_event(
            1,
            "User I/O".to_string(),
            "db file sequential read".to_string(),
            Duration::from_micros(1000),
        );

        let wait_events = hub.get_top_wait_events(10);
        assert_eq!(wait_events.len(), 1);
        assert_eq!(wait_events[0].total_waits, 1);
    }
}


