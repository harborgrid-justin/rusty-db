// Diagnostic Dumps
// Automatic diagnostic repository, incident packaging, core dump analysis, health checks

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant, SystemTime};
use std::fmt;
use std::path::PathBuf;

/// Incident severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IncidentSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for IncidentSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IncidentSeverity::Low => write!(f, "LOW"),
            IncidentSeverity::Medium => write!(f, "MEDIUM"),
            IncidentSeverity::High => write!(f, "HIGH"),
            IncidentSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Incident type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IncidentType {
    Crash,
    Hang,
    DataCorruption,
    PerformanceDegradation,
    MemoryLeak,
    DeadLock,
    ConnectionFailure,
    DiskFull,
    ReplicationFailure,
    Other,
}

impl fmt::Display for IncidentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IncidentType::Crash => write!(f, "CRASH"),
            IncidentType::Hang => write!(f, "HANG"),
            IncidentType::DataCorruption => write!(f, "DATA_CORRUPTION"),
            IncidentType::PerformanceDegradation => write!(f, "PERFORMANCE_DEGRADATION"),
            IncidentType::MemoryLeak => write!(f, "MEMORY_LEAK"),
            IncidentType::DeadLock => write!(f, "DEADLOCK"),
            IncidentType::ConnectionFailure => write!(f, "CONNECTION_FAILURE"),
            IncidentType::DiskFull => write!(f, "DISK_FULL"),
            IncidentType::ReplicationFailure => write!(f, "REPLICATION_FAILURE"),
            IncidentType::Other => write!(f, "OTHER"),
        }
    }
}

/// Diagnostic incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: u64,
    pub incident_type: IncidentType,
    pub severity: IncidentSeverity,
    pub timestamp: SystemTime,
    pub description: String,
    pub error_code: Option<u32>,
    pub error_message: Option<String>,
    pub stack_trace: Option<String>,
    pub affected_sessions: Vec<u64>,
    pub affected_queries: Vec<u64>,
    pub system_state: HashMap<String, String>,
    pub resolved: bool,
    pub resolution: Option<String>,
    pub resolved_at: Option<SystemTime>,
}

impl Incident {
    pub fn new(
        id: u64,
        incident_type: IncidentType,
        severity: IncidentSeverity,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id,
            incident_type,
            severity,
            timestamp: SystemTime::now(),
            description: description.into(),
            error_code: None,
            error_message: None,
            stack_trace: None,
            affected_sessions: Vec::new(),
            affected_queries: Vec::new(),
            system_state: HashMap::new(),
            resolved: false,
            resolution: None,
            resolved_at: None,
        }
    }

    pub fn with_error(mut self, code: u32, message: impl Into<String>) -> Self {
        self.error_code = Some(code);
        self.error_message = Some(message.into());
        self
    }

    pub fn with_stack_trace(mut self, trace: impl Into<String>) -> Self {
        self.stack_trace = Some(trace.into());
        self
    }

    pub fn add_affected_session(&mut self, session_id: u64) {
        if !self.affected_sessions.contains(&session_id) {
            self.affected_sessions.push(session_id);
        }
    }

    pub fn add_system_state(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.system_state.insert(key.into(), value.into());
    }

    pub fn resolve(&mut self, resolution: impl Into<String>) {
        self.resolved = true;
        self.resolution = Some(resolution.into());
        self.resolved_at = Some(SystemTime::now());
    }

    pub fn duration(&self) -> Duration {
        let end_time = self.resolved_at.unwrap_or_else(SystemTime::now);
        end_time.duration_since(self.timestamp).unwrap_or(Duration::ZERO)
    }
}

/// Diagnostic dump type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DumpType {
    SystemState,
    ProcessState,
    MemoryDump,
    LockDump,
    TransactionDump,
    BufferCache,
    ErrorStack,
    Full,
}

impl fmt::Display for DumpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DumpType::SystemState => write!(f, "SYSTEM_STATE"),
            DumpType::ProcessState => write!(f, "PROCESS_STATE"),
            DumpType::MemoryDump => write!(f, "MEMORY_DUMP"),
            DumpType::LockDump => write!(f, "LOCK_DUMP"),
            DumpType::TransactionDump => write!(f, "TRANSACTION_DUMP"),
            DumpType::BufferCache => write!(f, "BUFFER_CACHE"),
            DumpType::ErrorStack => write!(f, "ERROR_STACK"),
            DumpType::Full => write!(f, "FULL"),
        }
    }
}

/// Diagnostic dump
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticDump {
    pub id: u64,
    pub dump_type: DumpType,
    pub timestamp: SystemTime,
    pub incident_id: Option<u64>,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub size_bytes: usize,
}

impl DiagnosticDump {
    pub fn new(id: u64, dump_type: DumpType, content: impl Into<String>) -> Self {
        let content = content.into();
        let size_bytes = content.len();

        Self {
            id,
            dump_type,
            timestamp: SystemTime::now(),
            incident_id: None,
            content,
            file_path: None,
            size_bytes,
        }
    }

    pub fn with_incident(mut self, incident_id: u64) -> Self {
        self.incident_id = Some(incident_id);
        self
    }

    pub fn with_file_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }
}

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Unhealthy,
    Critical,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "HEALTHY"),
            HealthStatus::Warning => write!(f, "WARNING"),
            HealthStatus::Unhealthy => write!(f, "UNHEALTHY"),
            HealthStatus::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub check_name: String,
    pub status: HealthStatus,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: SystemTime,
    pub duration: Duration,
}

impl HealthCheckResult {
    pub fn new(check_name: impl Into<String>, status: HealthStatus, message: impl Into<String>) -> Self {
        Self {
            check_name: check_name.into(),
            status,
            message: message.into(),
            details: HashMap::new(),
            timestamp: SystemTime::now(),
            duration: Duration::ZERO,
        }
    }

    pub fn add_detail(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.details.insert(key.into(), value.into());
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }
}

/// Health check trait
pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> HealthCheckResult;
    fn is_critical(&self) -> bool {
        false
    }
}

/// Database connection health check
pub struct ConnectionHealthCheck {
    max_connections: usize,
    current_connections: Arc<RwLock<usize>>,
}

impl ConnectionHealthCheck {
    pub fn new(max_connections: usize, current_connections: Arc<RwLock<usize>>) -> Self {
        Self {
            max_connections,
            current_connections,
        }
    }
}

impl HealthCheck for ConnectionHealthCheck {
    fn name(&self) -> &str {
        "connection_pool"
    }

    fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        let current = *self.current_connections.read();
        let usage = (current as f64 / self.max_connections as f64) * 100.0;

        let (status, message) = if usage >= 95.0 {
            (HealthStatus::Critical, format!("Connection pool at {:.1}% capacity", usage))
        } else if usage >= 80.0 {
            (HealthStatus::Warning, format!("Connection pool at {:.1}% capacity", usage))
        } else {
            (HealthStatus::Healthy, format!("Connection pool healthy ({:.1}% used)", usage))
        };

        let mut result = HealthCheckResult::new(self.name(), status, message);
        result.add_detail("current_connections", current.to_string());
        result.add_detail("max_connections", self.max_connections.to_string());
        result.add_detail("usage_percent", format!("{:.1}", usage));
        result.set_duration(start.elapsed());

        result
    }

    fn is_critical(&self) -> bool {
        true
    }
}

/// Memory health check
pub struct MemoryHealthCheck {
    max_memory_bytes: u64,
    current_memory_bytes: Arc<RwLock<u64>>,
}

impl MemoryHealthCheck {
    pub fn new(max_memory_bytes: u64, current_memory_bytes: Arc<RwLock<u64>>) -> Self {
        Self {
            max_memory_bytes,
            current_memory_bytes,
        }
    }
}

impl HealthCheck for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory_usage"
    }

    fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        let current = *self.current_memory_bytes.read();
        let usage = (current as f64 / self.max_memory_bytes as f64) * 100.0;

        let (status, message) = if usage >= 95.0 {
            (HealthStatus::Critical, format!("Memory usage at {:.1}%", usage))
        } else if usage >= 85.0 {
            (HealthStatus::Warning, format!("Memory usage at {:.1}%", usage))
        } else {
            (HealthStatus::Healthy, format!("Memory usage healthy ({:.1}%)", usage))
        };

        let mut result = HealthCheckResult::new(self.name(), status, message);
        result.add_detail("current_bytes", current.to_string());
        result.add_detail("max_bytes", self.max_memory_bytes.to_string());
        result.add_detail("usage_percent", format!("{:.1}", usage));
        result.set_duration(start.elapsed());

        result
    }

    fn is_critical(&self) -> bool {
        true
    }
}

/// Diagnostic repository
pub struct DiagnosticRepository {
    incidents: Arc<RwLock<HashMap<u64, Incident>>>,
    dumps: Arc<RwLock<HashMap<u64, DiagnosticDump>>>,
    health_checks: Arc<RwLock<Vec<Arc<dyn HealthCheck>>>>,
    incident_history: Arc<RwLock<VecDeque<Incident>>>,
    last_incident_id: Arc<RwLock<u64>>,
    last_dump_id: Arc<RwLock<u64>>,
    max_history: usize,
    adr_base: PathBuf,
}

impl DiagnosticRepository {
    pub fn new(adr_base: PathBuf, max_history: usize) -> Self {
        Self {
            incidents: Arc::new(RwLock::new(HashMap::new())),
            dumps: Arc::new(RwLock::new(HashMap::new())),
            health_checks: Arc::new(RwLock::new(Vec::new())),
            incident_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            last_incident_id: Arc::new(RwLock::new(0)),
            last_dump_id: Arc::new(RwLock::new(0)),
            max_history,
            adr_base,
        }
    }

    pub fn register_health_check(&self, check: Arc<dyn HealthCheck>) {
        self.health_checks.write().push(check);
    }

    pub fn create_incident(
        &self,
        incident_type: IncidentType,
        severity: IncidentSeverity,
        description: impl Into<String>,
    ) -> u64 {
        let mut last_id = self.last_incident_id.write();
        *last_id += 1;
        let incident_id = *last_id;
        drop(last_id);

        let incident = Incident::new(incident_id, incident_type, severity, description);

        self.incidents.write().insert(incident_id, incident.clone());

        let mut history = self.incident_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(incident);

        // Auto-generate diagnostic dump for critical incidents
        if severity == IncidentSeverity::Critical {
            self.generate_dump(DumpType::Full, Some(incident_id));
        }

        incident_id
    }

    pub fn update_incident<F>(&self, incident_id: u64, updater: F) -> bool
    where
        F: FnOnce(&mut Incident),
    {
        if let Some(incident) = self.incidents.write().get_mut(&incident_id) {
            updater(incident);
            true
        } else {
            false
        }
    }

    pub fn resolve_incident(&self, incident_id: u64, resolution: impl Into<String>) -> bool {
        self.update_incident(incident_id, |incident| {
            incident.resolve(resolution);
        })
    }

    pub fn get_incident(&self, incident_id: u64) -> Option<Incident> {
        self.incidents.read().get(&incident_id).cloned()
    }

    pub fn get_active_incidents(&self) -> Vec<Incident> {
        self.incidents
            .read()
            .values()
            .filter(|i| !i.resolved)
            .cloned()
            .collect()
    }

    pub fn get_incidents_by_type(&self, incident_type: IncidentType) -> Vec<Incident> {
        self.incidents
            .read()
            .values()
            .filter(|i| i.incident_type == incident_type)
            .cloned()
            .collect()
    }

    pub fn get_critical_incidents(&self) -> Vec<Incident> {
        self.incidents
            .read()
            .values()
            .filter(|i| i.severity == IncidentSeverity::Critical && !i.resolved)
            .cloned()
            .collect()
    }

    pub fn generate_dump(&self, dump_type: DumpType, incident_id: Option<u64>) -> u64 {
        let mut last_id = self.last_dump_id.write();
        *last_id += 1;
        let dump_id = *last_id;
        drop(last_id);

        let content = match dump_type {
            DumpType::SystemState => self.generate_system_state_dump(),
            DumpType::ProcessState => self.generate_process_state_dump(),
            DumpType::MemoryDump => self.generate_memory_dump(),
            DumpType::LockDump => self.generate_lock_dump(),
            DumpType::TransactionDump => self.generate_transaction_dump(),
            DumpType::BufferCache => self.generate_buffer_cache_dump(),
            DumpType::ErrorStack => self.generate_error_stack_dump(),
            DumpType::Full => self.generate_full_dump(),
        };

        let mut dump = DiagnosticDump::new(dump_id, dump_type, content);

        if let Some(inc_id) = incident_id {
            dump = dump.with_incident(inc_id);
        }

        // Generate file path
        let filename = format!("dump_{}_{}.txt", dump_type, dump_id);
        let path = self.adr_base.join("dumps").join(filename);
        dump = dump.with_file_path(path);

        self.dumps.write().insert(dump_id, dump);

        dump_id
    }

    fn generate_system_state_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== SYSTEM STATE DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str(&format!("Active Incidents: {}\n", self.get_active_incidents().len()));
        dump.push_str("\n");
        dump
    }

    fn generate_process_state_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== PROCESS STATE DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_memory_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== MEMORY DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_lock_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== LOCK DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_transaction_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== TRANSACTION DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_buffer_cache_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== BUFFER CACHE DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_error_stack_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== ERROR STACK DUMP ===\n\n");
        dump.push_str(&format!("Timestamp: {:?}\n", SystemTime::now()));
        dump.push_str("\n");
        dump
    }

    fn generate_full_dump(&self) -> String {
        let mut dump = String::new();
        dump.push_str("=== FULL SYSTEM DUMP ===\n\n");
        dump.push_str(&self.generate_system_state_dump());
        dump.push_str("\n");
        dump.push_str(&self.generate_process_state_dump());
        dump.push_str("\n");
        dump.push_str(&self.generate_memory_dump());
        dump.push_str("\n");
        dump.push_str(&self.generate_lock_dump());
        dump.push_str("\n");
        dump.push_str(&self.generate_transaction_dump());
        dump
    }

    pub fn get_dump(&self, dump_id: u64) -> Option<DiagnosticDump> {
        self.dumps.read().get(&dump_id).cloned()
    }

    pub fn get_dumps_for_incident(&self, incident_id: u64) -> Vec<DiagnosticDump> {
        self.dumps
            .read()
            .values()
            .filter(|d| d.incident_id == Some(incident_id))
            .cloned()
            .collect()
    }

    pub fn run_health_checks(&self) -> Vec<HealthCheckResult> {
        let checks = self.health_checks.read();
        checks.iter().map(|check| check.check()).collect()
    }

    pub fn get_overall_health(&self) -> HealthStatus {
        let results = self.run_health_checks();

        if results.iter().any(|r| r.status == HealthStatus::Critical) {
            HealthStatus::Critical
        } else if results.iter().any(|r| r.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if results.iter().any(|r| r.status == HealthStatus::Warning) {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    pub fn generate_health_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== System Health Report ===\n\n");

        let overall = self.get_overall_health();
        report.push_str(&format!("Overall Status: {}\n\n", overall));

        let results = self.run_health_checks();
        for result in results {
            report.push_str(&format!("Check: {}\n", result.check_name));
            report.push_str(&format!("  Status: {}\n", result.status));
            report.push_str(&format!("  Message: {}\n", result.message));
            report.push_str(&format!("  Duration: {:?}\n", result.duration));

            if !result.details.is_empty() {
                report.push_str("  Details:\n");
                for (key, value) in &result.details {
                    report.push_str(&format!("    {}: {}\n", key, value));
                }
            }
            report.push_str("\n");
        }

        report
    }

    pub fn clear_old_incidents(&self, older_than: Duration) {
        let cutoff = SystemTime::now() - older_than;
        self.incidents
            .write()
            .retain(|_, incident| incident.timestamp >= cutoff || !incident.resolved);
    }

    pub fn clear_old_dumps(&self, older_than: Duration) {
        let cutoff = SystemTime::now() - older_than;
        self.dumps
            .write()
            .retain(|_, dump| dump.timestamp >= cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let adr = PathBuf::from("/tmp/adr");
        let repo = DiagnosticRepository::new(adr, 100);

        let incident_id = repo.create_incident(
            IncidentType::Crash,
            IncidentSeverity::Critical,
            "Database crashed",
        );

        let incident = repo.get_incident(incident_id).unwrap();
        assert_eq!(incident.incident_type, IncidentType::Crash);
        assert!(!incident.resolved);
    }

    #[test]
    fn test_incident_resolution() {
        let adr = PathBuf::from("/tmp/adr");
        let repo = DiagnosticRepository::new(adr, 100);

        let incident_id = repo.create_incident(
            IncidentType::Hang,
            IncidentSeverity::High,
            "Database hang detected",
        );

        repo.resolve_incident(incident_id, "Restarted background process");

        let incident = repo.get_incident(incident_id).unwrap();
        assert!(incident.resolved);
        assert_eq!(incident.resolution, Some("Restarted background process".to_string()));
    }

    #[test]
    fn test_diagnostic_dump() {
        let adr = PathBuf::from("/tmp/adr");
        let repo = DiagnosticRepository::new(adr, 100);

        let dump_id = repo.generate_dump(DumpType::SystemState, None);
        let dump = repo.get_dump(dump_id).unwrap();

        assert_eq!(dump.dump_type, DumpType::SystemState);
        assert!(!dump.content.is_empty());
    }

    #[test]
    fn test_health_checks() {
        let adr = PathBuf::from("/tmp/adr");
        let repo = DiagnosticRepository::new(adr, 100);

        let current_connections = Arc::new(RwLock::new(50));
        let check = Arc::new(ConnectionHealthCheck::new(100, current_connections));

        repo.register_health_check(check);

        let results = repo.run_health_checks();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, HealthStatus::Healthy);
    }
}


