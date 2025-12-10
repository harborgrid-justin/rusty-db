//! # Port Health Checking
//!
//! Health monitoring for network ports and services.
//!
//! ## Features
//!
//! - **Port Availability**: Check if ports are available for binding
//! - **Conflict Detection**: Detect bind conflicts
//! - **Exhaustion Monitoring**: Monitor port pool exhaustion
//! - **Periodic Health Checks**: Automated health monitoring

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener as StdTcpListener, UdpSocket as StdUdpSocket};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::interval;
use serde::{Deserialize, Serialize};

/// Health status for a port
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Port is healthy and available
    Healthy,

    /// Port is in use (expected state for active listeners)
    InUse,

    /// Port binding failed
    BindError,

    /// Port is unreachable
    Unreachable,

    /// Permission denied
    PermissionDenied,

    /// Unknown status
    Unknown,
}

impl HealthStatus {
    /// Check if the status is considered healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::InUse)
    }

    /// Check if the status indicates an error
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            HealthStatus::BindError | HealthStatus::Unreachable | HealthStatus::PermissionDenied
        )
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::InUse => write!(f, "In Use"),
            HealthStatus::BindError => write!(f, "Bind Error"),
            HealthStatus::Unreachable => write!(f, "Unreachable"),
            HealthStatus::PermissionDenied => write!(f, "Permission Denied"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Port number
    pub port: u16,

    /// Health status
    pub status: HealthStatus,

    /// Last check time
    pub checked_at: SystemTime,

    /// Optional error message
    pub error_message: Option<String>,

    /// Response time in milliseconds (if applicable)
    pub response_time_ms: Option<u64>,
}

impl HealthCheckResult {
    /// Create a new health check result
    pub fn new(port: u16, status: HealthStatus) -> Self {
        Self {
            port,
            status,
            checked_at: SystemTime::now(),
            error_message: None,
            response_time_ms: None,
        }
    }

    /// Set error message
    pub fn with_error(mut self, message: String) -> Self {
        self.error_message = Some(message);
        self
    }

    /// Set response time
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Check interval in seconds
    pub check_interval: u64,

    /// Timeout for health checks in milliseconds
    pub timeout_ms: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: 60,
            timeout_ms: 5000,
        }
    }
}

/// Port availability checker
pub struct PortAvailabilityChecker;

impl PortAvailabilityChecker {
    /// Check if a TCP port is available for binding
    pub fn check_tcp_available(port: u16) -> HealthStatus {
        let addr = format!("0.0.0.0:{}", port);

        match StdTcpListener::bind(&addr) {
            Ok(_) => HealthStatus::Healthy,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::AddrInUse => HealthStatus::InUse,
                    std::io::ErrorKind::PermissionDenied => HealthStatus::PermissionDenied,
                    _ => HealthStatus::BindError,
                }
            }
        }
    }

    /// Check if a UDP port is available for binding
    pub fn check_udp_available(port: u16) -> HealthStatus {
        let addr = format!("0.0.0.0:{}", port);

        match StdUdpSocket::bind(&addr) {
            Ok(_) => HealthStatus::Healthy,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::AddrInUse => HealthStatus::InUse,
                    std::io::ErrorKind::PermissionDenied => HealthStatus::PermissionDenied,
                    _ => HealthStatus::BindError,
                }
            }
        }
    }

    /// Check multiple TCP ports
    pub fn check_tcp_ports(ports: &[u16]) -> Vec<(u16, HealthStatus)> {
        ports.iter()
            .map(|&port| (port, Self::check_tcp_available(port)))
            .collect()
    }

    /// Check if a specific address is available
    pub fn check_addr_available(addr: &SocketAddr) -> HealthStatus {
        match StdTcpListener::bind(addr) {
            Ok(_) => HealthStatus::Healthy,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::AddrInUse => HealthStatus::InUse,
                    std::io::ErrorKind::PermissionDenied => HealthStatus::PermissionDenied,
                    _ => HealthStatus::BindError,
                }
            }
        }
    }
}

/// Conflict detector for port bindings
pub struct ConflictDetector {
    /// Known active ports
    active_ports: Arc<RwLock<HashMap<u16, String>>>,
}

impl ConflictDetector {
    /// Create a new conflict detector
    pub fn new() -> Self {
        Self {
            active_ports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an active port
    pub async fn register_port(&self, port: u16, service: String) {
        let mut active = self.active_ports.write().await;
        active.insert(port, service);
    }

    /// Unregister a port
    pub async fn unregister_port(&self, port: u16) {
        let mut active = self.active_ports.write().await;
        active.remove(&port);
    }

    /// Check for conflicts
    pub async fn check_conflict(&self, port: u16) -> Option<String> {
        let active = self.active_ports.read().await;
        active.get(&port).cloned()
    }

    /// Get all active ports
    pub async fn get_active_ports(&self) -> Vec<(u16, String)> {
        let active = self.active_ports.read().await;
        active.iter()
            .map(|(&port, service)| (port, service.clone()))
            .collect()
    }

    /// Detect conflicts in a port list
    pub async fn detect_conflicts(&self, ports: &[u16]) -> Vec<(u16, String)> {
        let active = self.active_ports.read().await;
        ports.iter()
            .filter_map(|&port| {
                active.get(&port).map(|service| (port, service.clone()))
            })
            .collect()
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Port exhaustion monitor
pub struct ExhaustionMonitor {
    /// Total available ports
    total_ports: usize,

    /// Currently allocated ports
    allocated_ports: Arc<RwLock<usize>>,

    /// Warning threshold (percentage)
    warning_threshold: f64,

    /// Critical threshold (percentage)
    critical_threshold: f64,
}

impl ExhaustionMonitor {
    /// Create a new exhaustion monitor
    pub fn new(total_ports: usize, warning_threshold: f64, critical_threshold: f64) -> Self {
        Self {
            total_ports,
            allocated_ports: Arc::new(RwLock::new(0)),
            warning_threshold,
            critical_threshold,
        }
    }

    /// Update allocated port count
    pub async fn update_allocated(&self, count: usize) {
        let mut allocated = self.allocated_ports.write().await;
        *allocated = count;
    }

    /// Get utilization percentage
    pub async fn utilization(&self) -> f64 {
        let allocated = *self.allocated_ports.read().await;
        (allocated as f64 / self.total_ports as f64) * 100.0
    }

    /// Check if at warning level
    pub async fn is_warning(&self) -> bool {
        self.utilization().await >= self.warning_threshold
    }

    /// Check if at critical level
    pub async fn is_critical(&self) -> bool {
        self.utilization().await >= self.critical_threshold
    }

    /// Get exhaustion status
    pub async fn get_status(&self) -> ExhaustionStatus {
        let utilization = self.utilization().await;

        if utilization >= self.critical_threshold {
            ExhaustionStatus::Critical
        } else if utilization >= self.warning_threshold {
            ExhaustionStatus::Warning
        } else {
            ExhaustionStatus::Normal
        }
    }

    /// Get available port count
    pub async fn available_count(&self) -> usize {
        let allocated = *self.allocated_ports.read().await;
        self.total_ports.saturating_sub(allocated)
    }
}

/// Exhaustion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExhaustionStatus {
    /// Normal usage
    Normal,

    /// Warning threshold exceeded
    Warning,

    /// Critical threshold exceeded
    Critical,
}

/// Port health checker
pub struct PortHealthChecker {
    config: HealthCheckConfig,
    health_history: Arc<RwLock<HashMap<u16, Vec<HealthCheckResult>>>>,
    conflict_detector: ConflictDetector,
    exhaustion_monitor: Arc<RwLock<Option<ExhaustionMonitor>>>,
}

impl PortHealthChecker {
    /// Create a new port health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            health_history: Arc::new(RwLock::new(HashMap::new())),
            conflict_detector: ConflictDetector::new(),
            exhaustion_monitor: Arc::new(RwLock::new(None)),
        }
    }

    /// Check health of a single port
    pub async fn check_port(&self, port: u16) -> Result<HealthCheckResult> {
        let start_time = SystemTime::now();

        let status = tokio::task::spawn_blocking(move || {
            PortAvailabilityChecker::check_tcp_available(port)
        })
        .await
        .map_err(|e| DbError::Internal(format!("Health check task failed: {}", e)))?;

        let response_time = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;

        let result = HealthCheckResult::new(port, status)
            .with_response_time(response_time);

        // Store in history
        let mut history = self.health_history.write().await;
        history.entry(port)
            .or_insert_with(Vec::new)
            .push(result.clone());

        // Keep only last 100 results per port
        if let Some(results) = history.get_mut(&port) {
            if results.len() > 100 {
                results.drain(0..results.len() - 100);
            }
        }

        Ok(result)
    }

    /// Check health of multiple ports
    pub async fn check_ports(&mut self, ports: &[u16]) -> Result<HashMap<u16, HealthStatus>> {
        let mut results = HashMap::new();

        for &port in ports {
            let result = self.check_port(port).await?;
            results.insert(port, result.status);
        }

        Ok(results)
    }

    /// Get health history for a port
    pub async fn get_history(&self, port: u16) -> Vec<HealthCheckResult> {
        let history = self.health_history.read().await;
        history.get(&port).cloned().unwrap_or_default()
    }

    /// Start periodic health checking
    pub async fn start_periodic_checks(&self, ports: Vec<u16>) -> Result<()> {
        let check_interval = Duration::from_secs(self.config.check_interval);
        let mut interval_timer = interval(check_interval);

        let health_history = self.health_history.clone();

        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;

                for &port in &ports {
                    let status = tokio::task::spawn_blocking(move || {
                        PortAvailabilityChecker::check_tcp_available(port)
                    })
                    .await
                    .unwrap_or(HealthStatus::Unknown);

                    let result = HealthCheckResult::new(port, status);

                    let mut history = health_history.write().await;
                    history.entry(port)
                        .or_insert_with(Vec::new)
                        .push(result);

                    // Keep only last 100 results
                    if let Some(results) = history.get_mut(&port) {
                        if results.len() > 100 {
                            results.drain(0..results.len() - 100);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Register an exhaustion monitor
    pub async fn set_exhaustion_monitor(&self, monitor: ExhaustionMonitor) {
        let mut guard = self.exhaustion_monitor.write().await;
        *guard = Some(monitor);
    }

    /// Get exhaustion status
    pub async fn get_exhaustion_status(&self) -> Option<ExhaustionStatus> {
        let guard = self.exhaustion_monitor.read().await;
        if let Some(monitor) = guard.as_ref() {
            Some(monitor.get_status().await)
        } else {
            None
        }
    }

    /// Get conflict detector
    pub fn conflict_detector(&self) -> &ConflictDetector {
        &self.conflict_detector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(HealthStatus::InUse.is_healthy());
        assert!(!HealthStatus::BindError.is_healthy());

        assert!(HealthStatus::BindError.is_error());
        assert!(!HealthStatus::Healthy.is_error());
    }

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::new(8080, HealthStatus::Healthy)
            .with_response_time(50)
            .with_error("Test error".to_string());

        assert_eq!(result.port, 8080);
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.response_time_ms, Some(50));
        assert_eq!(result.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_port_availability_checker() {
        // Port 0 should be available (OS will assign)
        let status = PortAvailabilityChecker::check_tcp_available(0);
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_conflict_detector() {
        let detector = ConflictDetector::new();

        detector.register_port(8080, "Service1".to_string()).await;

        let conflict = detector.check_conflict(8080).await;
        assert_eq!(conflict, Some("Service1".to_string()));

        let no_conflict = detector.check_conflict(9090).await;
        assert_eq!(no_conflict, None);
    }

    #[tokio::test]
    async fn test_exhaustion_monitor() {
        let monitor = ExhaustionMonitor::new(100, 75.0, 90.0);

        monitor.update_allocated(50).await;
        assert_eq!(monitor.utilization().await, 50.0);
        assert!(!monitor.is_warning().await);

        monitor.update_allocated(80).await;
        assert!(monitor.is_warning().await);
        assert!(!monitor.is_critical().await);

        monitor.update_allocated(95).await;
        assert!(monitor.is_critical().await);
    }

    #[tokio::test]
    async fn test_exhaustion_status() {
        let monitor = ExhaustionMonitor::new(100, 75.0, 90.0);

        monitor.update_allocated(50).await;
        assert_eq!(monitor.get_status().await, ExhaustionStatus::Normal);

        monitor.update_allocated(80).await;
        assert_eq!(monitor.get_status().await, ExhaustionStatus::Warning);

        monitor.update_allocated(95).await;
        assert_eq!(monitor.get_status().await, ExhaustionStatus::Critical);
    }

    #[tokio::test]
    async fn test_port_health_checker() {
        let config = HealthCheckConfig::default();
        let checker = PortHealthChecker::new(config);

        // Check a port that should be available
        let result = checker.check_port(0).await.unwrap();
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.response_time_ms.is_some());
    }

    #[tokio::test]
    async fn test_health_history() {
        let config = HealthCheckConfig::default();
        let checker = PortHealthChecker::new(config);

        checker.check_port(8080).await.unwrap();
        checker.check_port(8080).await.unwrap();

        let history = checker.get_history(8080).await;
        assert_eq!(history.len(), 2);
    }
}
