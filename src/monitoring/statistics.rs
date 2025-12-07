// System Statistics
// V$-style system views, session statistics, system-wide counters, wait event categories

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant, SystemTime};

/// V$SESSION - Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSession {
    pub session_id: u64,
    pub user_id: u64,
    pub username: String,
    pub program: String,
    pub machine: String,
    pub osuser: String,
    pub process_id: u64,
    pub status: String,
    pub schema_name: String,
    pub logon_time: SystemTime,
    pub last_call_time: SystemTime,
    pub sql_id: Option<u64>,
    pub sql_text: Option<String>,
    pub blocking_session: Option<u64>,
    pub wait_class: Option<String>,
    pub wait_event: Option<String>,
    pub wait_time_us: u64,
    pub state: String,
}

impl VSession {
    pub fn new(session_id: u64, user_id: u64, username: impl Into<String>) -> Self {
        Self {
            session_id,
            user_id,
            username: username.into(),
            program: String::new(),
            machine: String::new(),
            osuser: String::new(),
            process_id: 0,
            status: "ACTIVE".to_string(),
            schema_name: "PUBLIC".to_string(),
            logon_time: SystemTime::now(),
            last_call_time: SystemTime::now(),
            sql_id: None,
            sql_text: None,
            blocking_session: None,
            wait_class: None,
            wait_event: None,
            wait_time_us: 0,
            state: "IDLE".to_string(),
        }
    }
}

/// V$SQL - SQL statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSql {
    pub sql_id: u64,
    pub sql_text: String,
    pub sql_hash: u64,
    pub plan_hash: u64,
    pub executions: u64,
    pub elapsed_time_us: u64,
    pub cpu_time_us: u64,
    pub disk_reads: u64,
    pub buffer_gets: u64,
    pub rows_processed: u64,
    pub parse_calls: u64,
    pub optimizer_cost: f64,
    pub first_load_time: SystemTime,
    pub last_active_time: SystemTime,
    pub is_bind_sensitive: bool,
    pub is_bind_aware: bool,
}

impl VSql {
    pub fn new(sql_id: u64, sql_text: impl Into<String>, sql_hash: u64) -> Self {
        Self {
            sql_id,
            sql_text: sql_text.into(),
            sql_hash,
            plan_hash: 0,
            executions: 0,
            elapsed_time_us: 0,
            cpu_time_us: 0,
            disk_reads: 0,
            buffer_gets: 0,
            rows_processed: 0,
            parse_calls: 0,
            optimizer_cost: 0.0,
            first_load_time: SystemTime::now(),
            last_active_time: SystemTime::now(),
            is_bind_sensitive: false,
            is_bind_aware: false,
        }
    }

    pub fn average_elapsed_time(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            self.elapsed_time_us as f64 / self.executions as f64
        }
    }

    pub fn average_cpu_time(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            self.cpu_time_us as f64 / self.executions as f64
        }
    }

    pub fn buffer_hit_ratio(&self) -> f64 {
        if self.buffer_gets == 0 {
            0.0
        } else {
            ((self.buffer_gets - self.disk_reads) as f64 / self.buffer_gets as f64) * 100.0
        }
    }
}

/// V$SYSSTAT - System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSysstat {
    pub statistic_id: u64,
    pub name: String,
    pub value: u64,
    pub class: String,
}

/// V$SYSTEM_EVENT - System-wide wait events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSystemEvent {
    pub event: String,
    pub total_waits: u64,
    pub total_timeouts: u64,
    pub time_waited_us: u64,
    pub average_wait_us: f64,
    pub wait_class: String,
}

impl VSystemEvent {
    pub fn new(event: impl Into<String>, wait_class: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            total_waits: 0,
            total_timeouts: 0,
            time_waited_us: 0,
            average_wait_us: 0.0,
            wait_class: wait_class.into(),
        }
    }

    pub fn record_wait(&mut self, time_us: u64, timeout: bool) {
        self.total_waits += 1;
        if timeout {
            self.total_timeouts += 1;
        }
        self.time_waited_us += time_us;
        self.average_wait_us = self.time_waited_us as f64 / self.total_waits as f64;
    }
}

/// V$SESSTAT - Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSesstat {
    pub session_id: u64,
    pub statistic_id: u64,
    pub statistic_name: String,
    pub value: u64,
}

/// V$LOCK - Lock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLock {
    pub lock_id: u64,
    pub session_id: u64,
    pub lock_type: String,
    pub mode_held: String,
    pub mode_requested: Option<String>,
    pub object_id: u64,
    pub block: bool,
    pub lock_time: SystemTime,
}

/// V$TRANSACTION - Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VTransaction {
    pub transaction_id: u64,
    pub session_id: u64,
    pub start_time: SystemTime,
    pub status: String,
    pub undo_blocks: u64,
    pub redo_blocks: u64,
    pub isolation_level: String,
}

/// V$SQLAREA - SQL area statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSqlarea {
    pub sql_id: u64,
    pub sql_text: String,
    pub version_count: u64,
    pub executions: u64,
    pub sorts: u64,
    pub loads: u64,
    pub invalidations: u64,
    pub sharable_mem: u64,
    pub persistent_mem: u64,
    pub runtime_mem: u64,
}

/// V$BGPROCESS - Background process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VBgprocess {
    pub process_name: String,
    pub process_id: u64,
    pub description: String,
    pub status: String,
    pub start_time: SystemTime,
}

/// V$PARAMETER - System parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VParameter {
    pub name: String,
    pub value: String,
    pub is_default: bool,
    pub is_modifiable: bool,
    pub description: String,
}

/// V$DATABASE - Database information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VDatabase {
    pub db_name: String,
    pub db_unique_name: String,
    pub db_id: u64,
    pub created: SystemTime,
    pub log_mode: String,
    pub open_mode: String,
    pub platform_name: String,
}

/// Statistics collector for all V$ views
pub struct StatisticsCollector {
    sessions: Arc<RwLock<HashMap<u64, VSession>>>,
    sql_stats: Arc<RwLock<HashMap<u64, VSql>>>,
    system_stats: Arc<RwLock<HashMap<String, VSysstat>>>,
    system_events: Arc<RwLock<HashMap<String, VSystemEvent>>>,
    session_stats: Arc<RwLock<HashMap<(u64, u64), VSesstat>>>,
    locks: Arc<RwLock<HashMap<u64, VLock>>>,
    transactions: Arc<RwLock<HashMap<u64, VTransaction>>>,
    sql_areas: Arc<RwLock<HashMap<u64, VSqlarea>>>,
    bg_processes: Arc<RwLock<HashMap<String, VBgprocess>>>,
    parameters: Arc<RwLock<HashMap<String, VParameter>>>,
    database_info: Arc<RwLock<Option<VDatabase>>>,
    next_stat_id: Arc<RwLock<u64>>,
}

impl StatisticsCollector {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            sql_stats: Arc::new(RwLock::new(HashMap::new())),
            system_stats: Arc::new(RwLock::new(HashMap::new())),
            system_events: Arc::new(RwLock::new(HashMap::new())),
            session_stats: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            sql_areas: Arc::new(RwLock::new(HashMap::new())),
            bg_processes: Arc::new(RwLock::new(HashMap::new())),
            parameters: Arc::new(RwLock::new(HashMap::new())),
            database_info: Arc::new(RwLock::new(None)),
            next_stat_id: Arc::new(RwLock::new(0)),
        }
    }

    // V$SESSION operations
    pub fn register_session(&self, session: VSession) {
        self.sessions.write().insert(session.session_id, session);
    }

    pub fn update_session<F>(&self, session_id: u64, updater: F)
    where
        F: FnOnce(&mut VSession),
    {
        if let Some(session) = self.sessions.write().get_mut(&session_id) {
            updater(session);
        }
    }

    pub fn remove_session(&self, session_id: u64) {
        self.sessions.write().remove(&session_id);
    }

    pub fn get_session(&self, session_id: u64) -> Option<VSession> {
        self.sessions.read().get(&session_id).cloned()
    }

    pub fn get_all_sessions(&self) -> Vec<VSession> {
        self.sessions.read().values().cloned().collect()
    }

    pub fn get_active_sessions(&self) -> Vec<VSession> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.status == "ACTIVE")
            .cloned()
            .collect()
    }

    pub fn get_blocking_sessions(&self) -> Vec<VSession> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.blocking_session.is_some())
            .cloned()
            .collect()
    }

    // V$SQL operations
    pub fn register_sql(&self, sql: VSql) {
        self.sql_stats.write().insert(sql.sql_id, sql);
    }

    pub fn update_sql_stats(&self, sql_id: u64, elapsed_us: u64, cpu_us: u64, rows: u64) {
        let mut sql_stats = self.sql_stats.write();
        if let Some(sql) = sql_stats.get_mut(&sql_id) {
            sql.executions += 1;
            sql.elapsed_time_us += elapsed_us;
            sql.cpu_time_us += cpu_us;
            sql.rows_processed += rows;
            sql.last_active_time = SystemTime::now();
        }
    }

    pub fn get_sql_stats(&self, sql_id: u64) -> Option<VSql> {
        self.sql_stats.read().get(&sql_id).cloned()
    }

    pub fn get_top_sql_by_executions(&self, limit: usize) -> Vec<VSql> {
        let mut stats: Vec<_> = self.sql_stats.read().values().cloned().collect();
        stats.sort_by(|a, b| b.executions.cmp(&a.executions));
        stats.into_iter().take(limit).collect()
    }

    pub fn get_top_sql_by_elapsed_time(&self, limit: usize) -> Vec<VSql> {
        let mut stats: Vec<_> = self.sql_stats.read().values().cloned().collect();
        stats.sort_by(|a, b| b.elapsed_time_us.cmp(&a.elapsed_time_us));
        stats.into_iter().take(limit).collect()
    }

    // V$SYSSTAT operations
    pub fn set_system_stat(&self, name: impl Into<String>, value: u64, class: impl Into<String>) {
        let name = name.into();
        let mut stat_id = self.next_stat_id.write();
        *stat_id += 1;

        let stat = VSysstat {
            statistic_id: *stat_id,
            name: name.clone(),
            value,
            class: class.into(),
        };

        self.system_stats.write().insert(name, stat);
    }

    pub fn increment_system_stat(&self, name: &str, delta: u64) {
        if let Some(stat) = self.system_stats.write().get_mut(name) {
            stat.value += delta;
        }
    }

    pub fn get_system_stat(&self, name: &str) -> Option<VSysstat> {
        self.system_stats.read().get(name).cloned()
    }

    pub fn get_all_system_stats(&self) -> Vec<VSysstat> {
        self.system_stats.read().values().cloned().collect()
    }

    // V$SYSTEM_EVENT operations
    pub fn record_system_event(&self, event: impl Into<String>, wait_class: impl Into<String>, time_us: u64, timeout: bool) {
        let event_name = event.into();
        let mut events = self.system_events.write();

        let system_event = events
            .entry(event_name.clone())
            .or_insert_with(|| VSystemEvent::new(event_name, wait_class));

        system_event.record_wait(time_us, timeout);
    }

    pub fn get_system_event(&self, event: &str) -> Option<VSystemEvent> {
        self.system_events.read().get(event).cloned()
    }

    pub fn get_all_system_events(&self) -> Vec<VSystemEvent> {
        self.system_events.read().values().cloned().collect()
    }

    pub fn get_top_events_by_time(&self, limit: usize) -> Vec<VSystemEvent> {
        let mut events: Vec<_> = self.system_events.read().values().cloned().collect();
        events.sort_by(|a, b| b.time_waited_us.cmp(&a.time_waited_us));
        events.into_iter().take(limit).collect()
    }

    // V$SESSTAT operations
    pub fn set_session_stat(&self, session_id: u64, stat_name: impl Into<String>, value: u64) {
        let stat_name = stat_name.into();
        let mut stat_id = self.next_stat_id.write();
        *stat_id += 1;
        let id = *stat_id;

        let stat = VSesstat {
            session_id,
            statistic_id: id,
            statistic_name: stat_name,
            value,
        };

        self.session_stats.write().insert((session_id, id), stat);
    }

    pub fn get_session_stats(&self, session_id: u64) -> Vec<VSesstat> {
        self.session_stats
            .read()
            .values()
            .filter(|s| s.session_id == session_id)
            .cloned()
            .collect()
    }

    // V$LOCK operations
    pub fn register_lock(&self, lock: VLock) {
        self.locks.write().insert(lock.lock_id, lock);
    }

    pub fn release_lock(&self, lock_id: u64) {
        self.locks.write().remove(&lock_id);
    }

    pub fn get_locks_for_session(&self, session_id: u64) -> Vec<VLock> {
        self.locks
            .read()
            .values()
            .filter(|l| l.session_id == session_id)
            .cloned()
            .collect()
    }

    pub fn get_blocking_locks(&self) -> Vec<VLock> {
        self.locks.read().values().filter(|l| l.block).cloned().collect()
    }

    // V$TRANSACTION operations
    pub fn register_transaction(&self, transaction: VTransaction) {
        self.transactions.write().insert(transaction.transaction_id, transaction);
    }

    pub fn update_transaction<F>(&self, transaction_id: u64, updater: F)
    where
        F: FnOnce(&mut VTransaction),
    {
        if let Some(txn) = self.transactions.write().get_mut(&transaction_id) {
            updater(txn);
        }
    }

    pub fn remove_transaction(&self, transaction_id: u64) {
        self.transactions.write().remove(&transaction_id);
    }

    pub fn get_transaction(&self, transaction_id: u64) -> Option<VTransaction> {
        self.transactions.read().get(&transaction_id).cloned()
    }

    pub fn get_active_transactions(&self) -> Vec<VTransaction> {
        self.transactions
            .read()
            .values()
            .filter(|t| t.status == "ACTIVE")
            .cloned()
            .collect()
    }

    // V$SQLAREA operations
    pub fn register_sql_area(&self, sql_area: VSqlarea) {
        self.sql_areas.write().insert(sql_area.sql_id, sql_area);
    }

    pub fn get_sql_area(&self, sql_id: u64) -> Option<VSqlarea> {
        self.sql_areas.read().get(&sql_id).cloned()
    }

    // V$BGPROCESS operations
    pub fn register_bg_process(&self, process: VBgprocess) {
        self.bg_processes.write().insert(process.process_name.clone(), process);
    }

    pub fn get_bg_processes(&self) -> Vec<VBgprocess> {
        self.bg_processes.read().values().cloned().collect()
    }

    // V$PARAMETER operations
    pub fn set_parameter(&self, param: VParameter) {
        self.parameters.write().insert(param.name.clone(), param);
    }

    pub fn get_parameter(&self, name: &str) -> Option<VParameter> {
        self.parameters.read().get(name).cloned()
    }

    pub fn get_all_parameters(&self) -> Vec<VParameter> {
        self.parameters.read().values().cloned().collect()
    }

    // V$DATABASE operations
    pub fn set_database_info(&self, db_info: VDatabase) {
        *self.database_info.write() = Some(db_info);
    }

    pub fn get_database_info(&self) -> Option<VDatabase> {
        self.database_info.read().clone()
    }

    // Utility methods
    pub fn clear_all(&self) {
        self.sessions.write().clear();
        self.sql_stats.write().clear();
        self.system_stats.write().clear();
        self.system_events.write().clear();
        self.session_stats.write().clear();
        self.locks.write().clear();
        self.transactions.write().clear();
        self.sql_areas.write().clear();
        *self.next_stat_id.write() = 0;
    }

    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== System Statistics Report ===\n\n");

        // Database info
        if let Some(db_info) = self.get_database_info() {
            report.push_str(&format!("Database: {}\n", db_info.db_name));
            report.push_str(&format!("Mode: {}\n", db_info.open_mode));
            report.push_str(&format!("Platform: {}\n\n", db_info.platform_name));
        }

        // Session summary
        let sessions = self.get_all_sessions();
        let active_sessions = self.get_active_sessions();
        report.push_str(&format!("Total Sessions: {}\n", sessions.len()));
        report.push_str(&format!("Active Sessions: {}\n\n", active_sessions.len()));

        // Top SQL by executions
        report.push_str("Top SQL by Executions:\n");
        let top_sql = self.get_top_sql_by_executions(10);
        for (i, sql) in top_sql.iter().enumerate() {
            report.push_str(&format!(
                "  {}. SQL ID {}: {} executions, avg {:.2}ms\n",
                i + 1,
                sql.sql_id,
                sql.executions,
                sql.average_elapsed_time() / 1000.0
            ));
        }
        report.push_str("\n");

        // Top wait events
        report.push_str("Top Wait Events:\n");
        let top_events = self.get_top_events_by_time(10);
        for (i, event) in top_events.iter().enumerate() {
            report.push_str(&format!(
                "  {}. {}: {:.2}ms ({} waits)\n",
                i + 1,
                event.event,
                event.time_waited_us as f64 / 1000.0,
                event.total_waits
            ));
        }

        report
    }
}

impl Default for StatisticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_registration() {
        let collector = StatisticsCollector::new();
        let session = VSession::new(1, 100, "testuser");

        collector.register_session(session);
        assert_eq!(collector.get_all_sessions().len(), 1);

        collector.remove_session(1);
        assert_eq!(collector.get_all_sessions().len(), 0);
    }

    #[test]
    fn test_sql_stats() {
        let collector = StatisticsCollector::new();
        let sql = VSql::new(1, "SELECT * FROM users", 12345);

        collector.register_sql(sql);
        collector.update_sql_stats(1, 1000, 500, 10);

        let stats = collector.get_sql_stats(1).unwrap();
        assert_eq!(stats.executions, 1);
        assert_eq!(stats.elapsed_time_us, 1000);
    }

    #[test]
    fn test_system_stats() {
        let collector = StatisticsCollector::new();

        collector.set_system_stat("parse count", 100, "SQL");
        collector.increment_system_stat("parse count", 50);

        let stat = collector.get_system_stat("parse count").unwrap();
        assert_eq!(stat.value, 150);
    }

    #[test]
    fn test_system_events() {
        let collector = StatisticsCollector::new();

        collector.record_system_event("db file sequential read", "User I/O", 1000, false);
        collector.record_system_event("db file sequential read", "User I/O", 1500, false);

        let event = collector.get_system_event("db file sequential read").unwrap();
        assert_eq!(event.total_waits, 2);
        assert_eq!(event.time_waited_us, 2500);
    }
}
