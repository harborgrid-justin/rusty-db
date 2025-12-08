// Active Session History (ASH)
// Oracle-inspired periodic session sampling for historical query analysis

use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};


/// Session state at the time of sampling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    Active,
    Idle,
    IdleInTransaction,
    Waiting,
    Blocked,
    Terminated,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Active => write!(f, "ACTIVE"),
            SessionState::Idle => write!(f, "IDLE"),
            SessionState::IdleInTransaction => write!(f, "IDLE_IN_TRANSACTION"),
            SessionState::Waiting => write!(f, "WAITING"),
            SessionState::Blocked => write!(f, "BLOCKED"),
            SessionState::Terminated => write!(f, "TERMINATED"),
        }
    }
}

/// Wait class categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WaitClass {
    UserIO,
    SystemIO,
    Concurrency,
    Application,
    Configuration,
    Administrative,
    Network,
    Commit,
    Idle,
    Other,
}

impl fmt::Display for WaitClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaitClass::UserIO => write!(f, "User I/O"),
            WaitClass::SystemIO => write!(f, "System I/O"),
            WaitClass::Concurrency => write!(f, "Concurrency"),
            WaitClass::Application => write!(f, "Application"),
            WaitClass::Configuration => write!(f, "Configuration"),
            WaitClass::Administrative => write!(f, "Administrative"),
            WaitClass::Network => write!(f, "Network"),
            WaitClass::Commit => write!(f, "Commit"),
            WaitClass::Idle => write!(f, "Idle"),
            WaitClass::Other => write!(f, "Other"),
        }
    }
}

/// Active session sample - a snapshot of a session at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshSample {
    pub sample_id: u64,
    pub sample_time: SystemTime,
    pub session_id: u64,
    pub user_id: u64,
    pub program: String,
    pub session_state: SessionState,
    pub wait_class: Option<WaitClass>,
    pub wait_event: Option<String>,
    pub wait_time_us: u64,
    pub sql_id: Option<u64>,
    pub sql_text: Option<String>,
    pub sql_plan_hash: Option<u64>,
    pub blocking_session: Option<u64>,
    pub current_object_id: Option<u64>,
    pub current_file_id: Option<u64>,
    pub current_block_id: Option<u64>,
    pub cpu_time_us: u64,
    pub db_time_us: u64,
    pub temp_space_allocated: u64,
    pub pga_allocated: u64,
}

impl AshSample {
    pub fn new(sample_id: u64, session_id: u64, user_id: u64) -> Self {
        Self {
            sample_id,
            sample_time: SystemTime::now(),
            session_id,
            user_id,
            program: String::new(),
            session_state: SessionState::Idle,
            wait_class: None,
            wait_event: None,
            wait_time_us: 0,
            sql_id: None,
            sql_text: None,
            sql_plan_hash: None,
            blocking_session: None,
            current_object_id: None,
            current_file_id: None,
            current_block_id: None,
            cpu_time_us: 0,
            db_time_us: 0,
            temp_space_allocated: 0,
            pga_allocated: 0,
        }
    }

    pub fn with_state(mut self, state: SessionState) -> Self {
        self.session_state = state;
        self
    }

    pub fn with_wait(mut self, class: WaitClass, event: impl Into<String>, time_us: u64) -> Self {
        self.wait_class = Some(class);
        self.wait_event = Some(event.into());
        self.wait_time_us = time_us;
        self
    }

    pub fn with_sql(mut self, sql_id: u64, sql_text: impl Into<String>, plan_hash: u64) -> Self {
        self.sql_id = Some(sql_id);
        self.sql_text = Some(sql_text.into());
        self.sql_plan_hash = Some(plan_hash);
        self
    }

    pub fn with_blocking_session(mut self, blocking_session: u64) -> Self {
        self.blocking_session = Some(blocking_session);
        self
    }

    pub fn with_object(mut self, object_id: u64, file_id: u64, block_id: u64) -> Self {
        self.current_object_id = Some(object_id);
        self.current_file_id = Some(file_id);
        self.current_block_id = Some(block_id);
        self
    }

    pub fn with_timing(mut self, cpu_time_us: u64, db_time_us: u64) -> Self {
        self.cpu_time_us = cpu_time_us;
        self.db_time_us = db_time_us;
        self
    }

    pub fn with_memory(mut self, temp_space: u64, pga: u64) -> Self {
        self.temp_space_allocated = temp_space;
        self.pga_allocated = pga;
        self
    }
}

/// SQL statistics aggregated from ASH samples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlStatistics {
    pub sql_id: u64,
    pub sql_text: String,
    pub executions: u64,
    pub total_samples: u64,
    pub total_cpu_time_us: u64,
    pub total_db_time_us: u64,
    pub total_wait_time_us: u64,
    pub avg_cpu_time_us: u64,
    pub avg_db_time_us: u64,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub wait_breakdown: HashMap<WaitClass, u64>,
}

impl SqlStatistics {
    pub fn new(sql_id: u64, sql_text: impl Into<String>) -> Self {
        Self {
            sql_id,
            sql_text: sql_text.into(),
            executions: 0,
            total_samples: 0,
            total_cpu_time_us: 0,
            total_db_time_us: 0,
            total_wait_time_us: 0,
            avg_cpu_time_us: 0,
            avg_db_time_us: 0,
            first_seen: SystemTime::now(),
            last_seen: SystemTime::now(),
            wait_breakdown: HashMap::new(),
        }
    }

    pub fn add_sample(&mut self, sample: &AshSample) {
        self.total_samples += 1;
        self.total_cpu_time_us += sample.cpu_time_us;
        self.total_db_time_us += sample.db_time_us;
        self.total_wait_time_us += sample.wait_time_us;
        self.last_seen = sample.sample_time;

        if let Some(wait_class) = sample.wait_class {
            *self.wait_breakdown.entry(wait_class).or_insert(0) += sample.wait_time_us;
        }

        // Recalculate averages
        self.avg_cpu_time_us = self.total_cpu_time_us / self.total_samples;
        self.avg_db_time_us = self.total_db_time_us / self.total_samples;
    }
}

/// Session statistics aggregated from ASH samples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub session_id: u64,
    pub user_id: u64,
    pub program: String,
    pub total_samples: u64,
    pub state_breakdown: HashMap<SessionState, u64>,
    pub wait_breakdown: HashMap<WaitClass, u64>,
    pub total_cpu_time_us: u64,
    pub total_db_time_us: u64,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
}

impl SessionStatistics {
    pub fn new(session_id: u64, user_id: u64, program: impl Into<String>) -> Self {
        Self {
            session_id,
            user_id,
            program: program.into(),
            total_samples: 0,
            state_breakdown: HashMap::new(),
            wait_breakdown: HashMap::new(),
            total_cpu_time_us: 0,
            total_db_time_us: 0,
            first_seen: SystemTime::now(),
            last_seen: SystemTime::now(),
        }
    }

    pub fn add_sample(&mut self, sample: &AshSample) {
        self.total_samples += 1;
        *self.state_breakdown.entry(sample.session_state).or_insert(0) += 1;

        if let Some(wait_class) = sample.wait_class {
            *self.wait_breakdown.entry(wait_class).or_insert(0) += sample.wait_time_us;
        }

        self.total_cpu_time_us += sample.cpu_time_us;
        self.total_db_time_us += sample.db_time_us;
        self.last_seen = sample.sample_time;
    }
}

/// Active Session History repository
pub struct ActiveSessionHistory {
    samples: Arc<RwLock<VecDeque<AshSample>>>,
    max_samples: usize,
    sample_interval: Duration,
    last_sample_id: Arc<RwLock<u64>>,
    sql_statistics: Arc<RwLock<HashMap<u64, SqlStatistics>>>,
    session_statistics: Arc<RwLock<HashMap<u64, SessionStatistics>>>,
    enabled: Arc<RwLock<bool>>,
}

impl ActiveSessionHistory {
    pub fn new(max_samples: usize, sample_interval: Duration) -> Self {
        Self {
            samples: Arc::new(RwLock::new(VecDeque::with_capacity(max_samples))),
            max_samples,
            sample_interval,
            last_sample_id: Arc::new(RwLock::new(0)),
            sql_statistics: Arc::new(RwLock::new(HashMap::new())),
            session_statistics: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn record_sample(&self, mut sample: AshSample) {
        if !self.is_enabled() {
            return;
        }

        // Assign sample ID
        let mut last_id = self.last_sample_id.write();
        *last_id += 1;
        sample.sample_id = *last_id;
        drop(last_id);

        // Update SQL statistics
        if let Some(sql_id) = sample.sql_id {
            let mut sql_stats = self.sql_statistics.write();
            let _stats = sql_stats
                .entry(sql_id)
                .or_insert_with(|| SqlStatistics::new(sql_id, sample.sql_text.clone().unwrap_or_default()));
            stats.add_sample(&sample);
        }

        // Update session statistics
        let mut session_stats = self.session_statistics.write();
        let _stats = session_stats
            .entry(sample.session_id)
            .or_insert_with(|| SessionStatistics::new(sample.session_id, sample.user_id, sample.program.clone()));
        stats.add_sample(&sample);
        drop(session_stats);

        // Store sample
        let mut samples = self.samples.write();
        if samples.len() >= self.max_samples {
            samples.pop_front();
        }
        samples.push_back(sample);
    }

    pub fn get_samples(&self, limit: usize) -> Vec<AshSample> {
        let samples = self.samples.read();
        samples.iter().rev().take(limit).cloned().collect()
    }

    pub fn get_samples_for_session(&self, session_id: u64, limit: usize) -> Vec<AshSample> {
        let samples = self.samples.read();
        samples
            .iter()
            .rev()
            .filter(|s| s.session_id == session_id)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_samples_for_sql(&self, sql_id: u64, limit: usize) -> Vec<AshSample> {
        let samples = self.samples.read();
        samples
            .iter()
            .rev()
            .filter(|s| s.sql_id == Some(sql_id))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_samples_in_range(&self, start: SystemTime, end: SystemTime) -> Vec<AshSample> {
        let samples = self.samples.read();
        samples
            .iter()
            .filter(|s| s.sample_time >= start && s.sample_time <= end)
            .cloned()
            .collect()
    }

    pub fn get_active_sessions(&self) -> Vec<AshSample> {
        let samples = self.samples.read();
        let now = SystemTime::now();
        let threshold = now - Duration::from_secs(10);

        samples
            .iter()
            .rev()
            .filter(|s| {
                s.sample_time >= threshold && s.session_state == SessionState::Active
            })
            .cloned()
            .collect()
    }

    pub fn get_blocking_sessions(&self) -> Vec<AshSample> {
        let samples = self.samples.read();
        samples
            .iter()
            .rev()
            .filter(|s| s.blocking_session.is_some())
            .cloned()
            .collect()
    }

    pub fn get_top_sql_by_cpu(&self, limit: usize) -> Vec<SqlStatistics> {
        let sql_stats = self.sql_statistics.read();
        let mut stats: Vec<_> = sql_stats.values().cloned().collect();
        stats.sort_by(|a, b| b.total_cpu_time_us.cmp(&a.total_cpu_time_us));
        stats.into_iter().take(limit).collect()
    }

    pub fn get_top_sql_by_db_time(&self, limit: usize) -> Vec<SqlStatistics> {
        let sql_stats = self.sql_statistics.read();
        let mut stats: Vec<_> = sql_stats.values().cloned().collect();
        stats.sort_by(|a, b| b.total_db_time_us.cmp(&a.total_db_time_us));
        stats.into_iter().take(limit).collect()
    }

    pub fn get_top_sql_by_wait_time(&self, limit: usize) -> Vec<SqlStatistics> {
        let sql_stats = self.sql_statistics.read();
        let mut stats: Vec<_> = sql_stats.values().cloned().collect();
        stats.sort_by(|a, b| b.total_wait_time_us.cmp(&a.total_wait_time_us));
        stats.into_iter().take(limit).collect()
    }

    pub fn get_sql_statistics(&self, sql_id: u64) -> Option<SqlStatistics> {
        self.sql_statistics.read().get(&sql_id).cloned()
    }

    pub fn get_session_statistics(&self, session_id: u64) -> Option<SessionStatistics> {
        self.session_statistics.read().get(&session_id).cloned()
    }

    pub fn get_wait_event_summary(&self) -> HashMap<WaitClass, u64> {
        let samples = self.samples.read();
        let mut summary: HashMap<WaitClass, u64> = HashMap::new();

        for sample in samples.iter() {
            if let Some(wait_class) = sample.wait_class {
                *summary.entry(wait_class).or_insert(0) += sample.wait_time_us;
            }
        }

        summary
    }

    pub fn get_session_state_summary(&self) -> HashMap<SessionState, u64> {
        let samples = self.samples.read();
        let mut summary: HashMap<SessionState, u64> = HashMap::new();

        for sample in samples.iter() {
            *summary.entry(sample.session_state).or_insert(0) += 1;
        }

        summary
    }

    pub fn clear(&self) {
        self.samples.write().clear();
        self.sql_statistics.write().clear();
        self.session_statistics.write().clear();
        *self.last_sample_id.write() = 0;
    }

    pub fn clear_old_samples(&self, older_than: Duration) {
        let cutoff = SystemTime::now() - older_than;
        let mut samples = self.samples.write();
        samples.retain(|s| s.sample_time >= cutoff);
    }

    pub fn get_sample_count(&self) -> usize {
        self.samples.read().len()
    }

    pub fn get_sample_interval(&self) -> Duration {
        self.sample_interval
    }
}

impl Default for ActiveSessionHistory {
    fn default() -> Self {
        Self::new(100000, Duration::from_secs(1))
    }
}

/// ASH report generator
pub struct AshReportGenerator<'a> {
    ash: &'a ActiveSessionHistory,
}

impl<'a> AshReportGenerator<'a> {
    pub fn new(ash: &'a ActiveSessionHistory) -> Self {
        Self { ash }
    }

    pub fn generate_summary_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Active Session History Summary ===\n\n");

        let sample_count = self.ash.get_sample_count();
        report.push_str(&format!("Total Samples: {}\n", sample_count));
        report.push_str(&format!("Sample Interval: {:?}\n\n", self.ash.get_sample_interval()));

        // Session state summary
        report.push_str("Session State Breakdown:\n");
        let state_summary = self.ash.get_session_state_summary();
        for (state, count) in state_summary.iter() {
            let percentage = (*count as f64 / sample_count as f64) * 100.0;
            report.push_str(&format!("  {}: {} ({:.2}%)\n", state, count, percentage));
        }
        report.push_str("\n");

        // Wait event summary
        report.push_str("Wait Event Breakdown:\n");
        let wait_summary = self.ash.get_wait_event_summary();
        let total_wait_time: u64 = wait_summary.values().sum();
        for (wait_class, time_us) in wait_summary.iter() {
            let percentage = (*time_us as f64 / total_wait_time as f64) * 100.0;
            report.push_str(&format!(
                "  {}: {:.2}ms ({:.2}%)\n",
                wait_class,
                *time_us as f64 / 1000.0,
                percentage
            ));
        }
        report.push_str("\n");

        // Top SQL by DB time
        report.push_str("Top SQL by DB Time:\n");
        let top_sql = self.ash.get_top_sql_by_db_time(10);
        for (i, sql) in top_sql.iter().enumerate() {
            report.push_str(&format!(
                "  {}. SQL ID {}: {:.2}ms (samples: {})\n",
                i + 1,
                sql.sql_id,
                sql.total_db_time_us as f64 / 1000.0,
                sql.total_samples
            ));
            report.push_str(&format!("     {}\n", sql.sql_text));
        }

        report
    }

    pub fn generate_blocking_sessions_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Blocking Sessions Report ===\n\n");

        let blocking = self.ash.get_blocking_sessions();
        if blocking.is_empty() {
            report.push_str("No blocking sessions detected.\n");
            return report;
        }

        for sample in blocking.iter() {
            report.push_str(&format!(
                "Session {} blocked by session {} at {:?}\n",
                sample.session_id,
                sample.blocking_session.unwrap(),
                sample.sample_time
            ));
            if let Some(sql) = &sample.sql_text {
                report.push_str(&format!("  SQL: {}\n", sql));
            }
            if let Some(wait_event) = &sample.wait_event {
                report.push_str(&format!("  Wait Event: {}\n", wait_event));
            }
            report.push_str("\n");
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ash_sample() {
        let sample = AshSample::new(1, 100, 1)
            .with_state(SessionState::Active)
            .with_sql(1001, "SELECT * FROM users", 12345)
            .with_timing(5000, 10000);

        assert_eq!(sample.session_id, 100);
        assert_eq!(sample.session_state, SessionState::Active);
        assert_eq!(sample.sql_id, Some(1001));
        assert_eq!(sample.cpu_time_us, 5000);
    }

    #[test]
    fn test_active_session_history() {
        let ash = ActiveSessionHistory::new(100::from_secs(1));

        let sample = AshSample::new(0, 100, 1)
            .with_state(SessionState::Active)
            .with_sql(1001, "SELECT * FROM users", 12345);

        ash.record_sample(sample);

        assert_eq!(ash.get_sample_count(), 1);
        let samples = ash.get_samples(10);
        assert_eq!(samples.len(), 1);
    }

    #[test]
    fn test_sql_statistics() {
        let ash = ActiveSessionHistory::new(100::from_secs(1));

        for _i in 0..5 {
            let sample = AshSample::new(0, 100, 1)
                .with_state(SessionState::Active)
                .with_sql(1001, "SELECT * FROM users", 12345)
                .with_timing(1000, 2000);

            ash.record_sample(sample);
        }

        let _stats = ash.get_sql_statistics(1001).unwrap();
        assert_eq!(stats.total_samples, 5);
        assert_eq!(stats.total_cpu_time_us, 5000);
        assert_eq!(stats.avg_cpu_time_us, 1000);
    }

    #[test]
    fn test_top_sql() {
        let ash = ActiveSessionHistory::new(100::from_secs(1));

        for _i in 0..3 {
            let sample = AshSample::new(0, 100, 1)
                .with_sql(1001, "SELECT * FROM users", 12345)
                .with_timing(1000, 5000);
            ash.record_sample(sample);
        }

        for _i in 0..2 {
            let sample = AshSample::new(0, 101, 1)
                .with_sql(1002, "SELECT * FROM orders", 54321)
                .with_timing(500, 2000);
            ash.record_sample(sample);
        }

        let top_sql = ash.get_top_sql_by_db_time(10);
        assert_eq!(top_sql.len(), 2);
        assert_eq!(top_sql[0].sql_id, 1001); // More DB time
    }
}


