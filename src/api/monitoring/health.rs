// Monitoring Module
//
// Part of the comprehensive monitoring system for RustyDB

use std::sync::{Arc, Mutex, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::DbError;
use super::metrics_core::*;

// SECTION 3: HEALTH CHECK SYSTEM (500+ lines)
// ============================================================================

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn worst(statuses: &[HealthStatus]) -> HealthStatus {
        if statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
            return HealthStatus::Unhealthy;
        }
        if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
            return HealthStatus::Degraded;
        }
        if statuses.iter().any(|s| matches!(s, HealthStatus::Unknown)) {
            return HealthStatus::Unknown;
        }
        HealthStatus::Healthy
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub duration: Duration,
    pub details: HashMap<String, serde_json::Value>,
}

impl HealthCheckResult {
    pub fn healthy(component: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            component: component.into(),
            message: "OK".to_string(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn degraded(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            component: component.into(),
            message: message.into(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn unhealthy(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            component: component.into(),
            message: message.into(),
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            details: HashMap::new(),
        }
    }

    pub fn with_detail(mut self, key: String, value: serde_json::Value) -> Self {
        self.details.insert(key, value);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Trait for implementing health checks
pub trait HealthChecker: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> HealthCheckResult;
}

/// Liveness probe - is the service alive?
pub struct LivenessProbe {
    started: AtomicBool,
    startup_time: RwLock<Option<SystemTime>>,
}

impl LivenessProbe {
    pub fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            startup_time: RwLock::new(None),
        }
    }

    pub fn mark_started(&self) {
        self.started.store(true, Ordering::SeqCst);
        *self.startup_time.write() = Some(SystemTime::now());
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.startup_time.read()
            .and_then(|start| SystemTime::now().duration_since(start).ok())
    }
}

impl HealthChecker for LivenessProbe {
    fn name(&self) -> &str {
        "liveness"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();

        if self.started.load(Ordering::SeqCst) {
            let mut result = HealthCheckResult::healthy("liveness");
            if let Some(uptime) = self.uptime() {
                result = result.with_detail(
                    "uptime_seconds".to_string(),
                    serde_json::json!(uptime.as_secs()),
                );
            }
            result.with_duration(Duration::from_micros(timer.elapsed_micros()))
        } else {
            HealthCheckResult::unhealthy("liveness", "Service not started")
                .with_duration(Duration::from_micros(timer.elapsed_micros()))
        }
    }
}

impl Default for LivenessProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// Readiness probe - is the service ready to accept traffic?
pub struct ReadinessProbe {
    dependencies: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
    min_healthy_dependencies: usize,
}

impl ReadinessProbe {
    pub fn new(min_healthy: usize) -> Self {
        Self {
            dependencies: Arc::new(RwLock::new(Vec::new())),
            min_healthy_dependencies: min_healthy,
        }
    }

    pub fn add_dependency(&self, checker: Arc<dyn HealthChecker>) {
        self.dependencies.write().push(checker);
    }
}

impl HealthChecker for ReadinessProbe {
    fn name(&self) -> &str {
        "readiness"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let dependencies = self.dependencies.read();

        if dependencies.is_empty() {
            return HealthCheckResult::healthy("readiness")
                .with_duration(Duration::from_micros(timer.elapsed_micros()));
        }

        let results: Vec<_> = dependencies.iter()
            .map(|dep| dep.check())
            .collect();

        let healthy_count = results.iter()
            .filter(|r| r.status.is_healthy())
            .count();

        let status = if healthy_count >= self.min_healthy_dependencies {
            HealthStatus::Healthy
        } else if healthy_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        let mut result = HealthCheckResult {
            status,
            component: "readiness".to_string(),
            message: format!("{}/{} dependencies healthy", healthy_count, dependencies.len()),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        };

        result = result.with_detail(
            "dependency_results".to_string(),
            serde_json::to_value(&results).unwrap(),
        );

        result
    }
}

/// Startup probe - has the service completed initialization?
pub struct StartupProbe {
    initialization_checks: Arc<RwLock<Vec<(String, Arc<AtomicBool>)>>>,
}

impl StartupProbe {
    pub fn new() -> Self {
        Self {
            initialization_checks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_check(&self, name: String, completed: Arc<AtomicBool>) {
        self.initialization_checks.write().push((name, completed));
    }

    pub fn mark_complete(&self, name: &str) {
        let checks = self.initialization_checks.read();
        for (check_name, flag) in checks.iter() {
            if check_name == name {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

impl HealthChecker for StartupProbe {
    fn name(&self) -> &str {
        "startup"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let checks = self.initialization_checks.read();

        let total = checks.len();
        let completed = checks.iter()
            .filter(|(_, flag)| flag.load(Ordering::SeqCst))
            .count();

        let status = if completed == total {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        HealthCheckResult {
            status,
            component: "startup".to_string(),
            message: format!("{}/{} initialization checks completed", completed, total),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("total_checks".to_string(), serde_json::json!(total))
        .with_detail("completed_checks".to_string(), serde_json::json!(completed))
    }
}

impl Default for StartupProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// Database connection health check
pub struct DatabaseHealthCheck {
    name: String,
    active_connections: Arc<RwLock<usize>>,
    max_connections: usize,
    warn_threshold: f64, // percentage
}

impl DatabaseHealthCheck {
    pub fn new(
        name: String,
        active_connections: Arc<RwLock<usize>>,
        max_connections: usize,
    ) -> Self {
        Self {
            name,
            active_connections,
            max_connections,
            warn_threshold: 0.8,
        }
    }
}

impl HealthChecker for DatabaseHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let active = *self.active_connections.read();
        let usage_pct = active as f64 / self.max_connections as f64;

        let status = if usage_pct >= 1.0 {
            HealthStatus::Unhealthy
        } else if usage_pct >= self.warn_threshold {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        HealthCheckResult {
            status,
            component: self.name.clone(),
            message: format!("{}/{} connections in use", active, self.max_connections),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("active_connections".to_string(), serde_json::json!(active))
        .with_detail("max_connections".to_string(), serde_json::json!(self.max_connections))
        .with_detail("usage_percent".to_string(), serde_json::json!(usage_pct * 100.0))
    }
}

/// Memory health check
pub struct MemoryHealthCheck {
    max_memory_bytes: u64,
    current_usage: Arc<RwLock<u64>>,
    warn_threshold: f64,
}

impl MemoryHealthCheck {
    pub fn new(max_memory_bytes: u64, current_usage: Arc<RwLock<u64>>) -> Self {
        Self {
            max_memory_bytes,
            current_usage,
            warn_threshold: 0.85,
        }
    }
}

impl HealthChecker for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn check(&self) -> HealthCheckResult {
        let timer = Timer::new();
        let current = *self.current_usage.read();
        let usage_pct = current as f64 / self.max_memory_bytes as f64;

        let status = if usage_pct >= 0.95 {
            HealthStatus::Unhealthy
        } else if usage_pct >= self.warn_threshold {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        HealthCheckResult {
            status,
            component: "memory".to_string(),
            message: format!("{:.2}% memory used", usage_pct * 100.0),
            timestamp: SystemTime::now(),
            duration: Duration::from_micros(timer.elapsed_micros()),
            details: HashMap::new(),
        }
        .with_detail("current_bytes".to_string(), serde_json::json!(current))
        .with_detail("max_bytes".to_string(), serde_json::json!(self.max_memory_bytes))
    }
}

/// Self-healing trigger based on health checks
pub struct SelfHealingTrigger {
    name: String,
    health_checker: Arc<dyn HealthChecker>,
    consecutive_failures: Arc<AtomicU64>,
    failure_threshold: u64,
    healing_action: Arc<dyn Fn() -> Result<(), DbError> + Send + Sync>,
}

impl SelfHealingTrigger {
    pub fn new(
        name: String,
        health_checker: Arc<dyn HealthChecker>,
        failure_threshold: u64,
        healing_action: Arc<dyn Fn() -> Result<(), DbError> + Send + Sync>,
    ) -> Self {
        Self {
            name,
            health_checker,
            consecutive_failures: Arc::new(AtomicU64::new(0)),
            failure_threshold,
            healing_action,
        }
    }

    pub fn check_and_heal(&self) -> Result<(), DbError> {
        let result = self.health_checker.check();

        if !result.status.is_healthy() {
            let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;

            if failures >= self.failure_threshold {
                println!("Triggering self-healing for {}: {} consecutive failures",
                    self.name, failures);
                (self.healing_action)()?;
                self.consecutive_failures.store(0, Ordering::SeqCst);
            }
        } else {
            self.consecutive_failures.store(0, Ordering::SeqCst);
        }

        Ok(())
    }
}

/// Health check coordinator
pub struct HealthCheckCoordinator {
    checkers: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
    liveness_probe: Arc<LivenessProbe>,
    readiness_probe: Arc<ReadinessProbe>,
    startup_probe: Arc<StartupProbe>,
    self_healing_triggers: Arc<RwLock<Vec<SelfHealingTrigger>>>,
}

impl HealthCheckCoordinator {
    pub fn new() -> Self {
        Self {
            checkers: Arc::new(RwLock::new(Vec::new())),
            liveness_probe: Arc::new(LivenessProbe::new()),
            readiness_probe: Arc::new(ReadinessProbe::new(1)),
            startup_probe: Arc::new(StartupProbe::new()),
            self_healing_triggers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_checker(&self, checker: Arc<dyn HealthChecker>) {
        self.checkers.write().push(checker);
    }

    pub fn add_self_healing_trigger(&self, trigger: SelfHealingTrigger) {
        self.self_healing_triggers.write().push(trigger);
    }

    pub fn liveness(&self) -> HealthCheckResult {
        self.liveness_probe.check()
    }

    pub fn readiness(&self) -> HealthCheckResult {
        self.readiness_probe.check()
    }

    pub fn startup(&self) -> HealthCheckResult {
        self.startup_probe.check()
    }

    pub fn check_all(&self) -> Vec<HealthCheckResult> {
        let checkers = self.checkers.read();
        checkers.iter().map(|c| c.check()).collect()
    }

    pub fn overall_health(&self) -> HealthStatus {
        let results = self.check_all();
        let statuses: Vec<_> = results.iter().map(|r| r.status).collect();
        HealthStatus::worst(&statuses)
    }

    pub fn run_self_healing(&self) {
        let triggers = self.self_healing_triggers.read();
        for trigger in triggers.iter() {
            if let Err(e) = trigger.check_and_heal() {
                eprintln!("Self-healing trigger '{}' failed: {:?}", trigger.name, e);
            }
        }
    }
}

impl Default for HealthCheckCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
