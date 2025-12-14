// # Health Monitoring System
//
// Enterprise-grade health monitoring with heartbeats, failure detection,
// and comprehensive node health tracking for RustyDB distributed database.
//
// ## Features
//
// - Heartbeat management with adaptive intervals
// - Phi Accrual failure detection
// - Multiple health check types (TCP, HTTP, gRPC, custom)
// - Health aggregation and scoring
// - Liveness and readiness probes
// - Automatic recovery management
// - Metrics and reporting
//
// ## Usage
//
// ```rust
// use rusty_db::networking::health::{HealthMonitor, HealthConfig};
//
// let config = HealthConfig::default();
// let mut monitor = HealthMonitor::new(config);
// monitor.start().await?;
// ```

use crate::common::{Component, HealthStatus, NodeId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub mod aggregator;
pub mod checker;
pub mod detector;
pub mod heartbeat;
pub mod liveness;
pub mod recovery;
pub mod reporter;

pub use aggregator::HealthAggregator;
pub use checker::HealthChecker;
pub use detector::{FailureDetector, PhiAccrualDetector};
pub use heartbeat::HeartbeatManager;
pub use liveness::{LivenessProbe, ReadinessProbe, StartupProbe};
pub use recovery::RecoveryManager;
pub use reporter::{HealthReport, HealthReporter};

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Heartbeat interval
    pub heartbeat_interval: Duration,

    /// Heartbeat timeout
    pub heartbeat_timeout: Duration,

    /// Phi accrual threshold (higher = more tolerant)
    pub phi_threshold: f64,

    /// Health check interval
    pub check_interval: Duration,

    /// Enable automatic recovery
    pub enable_auto_recovery: bool,

    /// Recovery retry attempts
    pub recovery_attempts: u32,

    /// Quarantine duration after recovery
    pub quarantine_duration: Duration,

    /// Enable metrics reporting
    pub enable_metrics: bool,

    /// Metrics export interval
    pub metrics_interval: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(1),
            heartbeat_timeout: Duration::from_secs(5),
            phi_threshold: 8.0,
            check_interval: Duration::from_secs(10),
            enable_auto_recovery: true,
            recovery_attempts: 3,
            quarantine_duration: Duration::from_secs(30),
            enable_metrics: true,
            metrics_interval: Duration::from_secs(60),
        }
    }
}

/// Health event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthEvent {
    /// Node became healthy
    NodeHealthy { node_id: NodeId, timestamp: u64 },

    /// Node became unhealthy
    NodeUnhealthy {
        node_id: NodeId,
        reason: String,
        timestamp: u64,
    },

    /// Node suspected of failure
    NodeSuspected {
        node_id: NodeId,
        phi_value: f64,
        timestamp: u64,
    },

    /// Node recovered
    NodeRecovered {
        node_id: NodeId,
        attempts: u32,
        timestamp: u64,
    },

    /// Recovery failed
    RecoveryFailed {
        node_id: NodeId,
        reason: String,
        timestamp: u64,
    },

    /// Heartbeat missed
    HeartbeatMissed {
        node_id: NodeId,
        consecutive_misses: u32,
        timestamp: u64,
    },

    /// Health check failed
    HealthCheckFailed {
        node_id: NodeId,
        check_type: String,
        reason: String,
        timestamp: u64,
    },
}

/// Health event listener trait
pub trait HealthEventListener: Send + Sync {
    /// Handle a health event
    fn on_health_event(&mut self, event: HealthEvent) -> Result<()>;
}

/// Main health monitor
pub struct HealthMonitor {
    config: HealthConfig,
    heartbeat_manager: Arc<RwLock<HeartbeatManager>>,
    failure_detector: Arc<RwLock<dyn FailureDetector + Send + Sync>>,
    health_checks: Arc<RwLock<HashMap<NodeId, Vec<Box<dyn HealthChecker + Send + Sync>>>>>,
    aggregator: Arc<RwLock<HealthAggregator>>,
    reporter: Arc<RwLock<HealthReporter>>,
    recovery_manager: Arc<RwLock<RecoveryManager>>,
    liveness_probes: Arc<RwLock<HashMap<NodeId, Box<dyn LivenessProbe + Send + Sync>>>>,
    readiness_probes: Arc<RwLock<HashMap<NodeId, Box<dyn ReadinessProbe + Send + Sync>>>>,
    event_listeners: Arc<RwLock<Vec<Box<dyn HealthEventListener + Send + Sync>>>>,
    running: Arc<RwLock<bool>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthConfig) -> Self {
        let heartbeat_manager = Arc::new(RwLock::new(HeartbeatManager::new(
            config.heartbeat_interval,
            config.heartbeat_timeout,
        )));

        let failure_detector = Arc::new(RwLock::new(PhiAccrualDetector::new(
            config.phi_threshold,
            100,
        )));

        let aggregator = Arc::new(RwLock::new(HealthAggregator::new()));
        let reporter = Arc::new(RwLock::new(HealthReporter::new()));
        let recovery_manager = Arc::new(RwLock::new(RecoveryManager::new(
            config.recovery_attempts,
            config.quarantine_duration,
        )));

        Self {
            config,
            heartbeat_manager,
            failure_detector,
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            aggregator,
            reporter,
            recovery_manager,
            liveness_probes: Arc::new(RwLock::new(HashMap::new())),
            readiness_probes: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the health monitor
    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(DbError::InvalidState(
                "Health monitor already running".to_string(),
            ));
        }
        *running = true;
        drop(running);

        // Start heartbeat loop
        self.start_heartbeat_loop().await;

        // Start health check loop
        self.start_health_check_loop().await;

        // Start metrics reporting loop
        if self.config.enable_metrics {
            self.start_metrics_loop().await;
        }

        Ok(())
    }

    /// Stop the health monitor
    pub async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    /// Add a health check for a node
    pub async fn add_health_check(
        &self,
        node_id: NodeId,
        checker: Box<dyn HealthChecker + Send + Sync>,
    ) -> Result<()> {
        let mut checks = self.health_checks.write().await;
        checks.entry(node_id).or_insert_with(Vec::new).push(checker);
        Ok(())
    }

    /// Add an event listener
    pub async fn add_event_listener(
        &self,
        listener: Box<dyn HealthEventListener + Send + Sync>,
    ) -> Result<()> {
        let mut listeners = self.event_listeners.write().await;
        listeners.push(listener);
        Ok(())
    }

    /// Get health status for a node
    pub async fn get_node_health(&self, node_id: &NodeId) -> Result<HealthStatus> {
        let aggregator = self.aggregator.read().await;
        Ok(aggregator.get_node_status(node_id).await?)
    }

    /// Get health report
    pub async fn get_health_report(&self) -> Result<HealthReport> {
        let reporter = self.reporter.read().await;
        Ok(reporter.generate_report().await?)
    }

    /// Record heartbeat from a node
    pub async fn record_heartbeat(&self, node_id: NodeId) -> Result<()> {
        let mut manager = self.heartbeat_manager.write().await;
        manager.record_heartbeat(node_id.clone())?;

        let mut detector = self.failure_detector.write().await;
        detector.record_heartbeat(node_id, Instant::now())?;

        Ok(())
    }

    /// Emit a health event
    async fn emit_event(&self, event: HealthEvent) -> Result<()> {
        let mut listeners = self.event_listeners.write().await;
        for listener in listeners.iter_mut() {
            listener.on_health_event(event.clone())?;
        }
        Ok(())
    }

    /// Start heartbeat monitoring loop
    async fn start_heartbeat_loop(&self) {
        let heartbeat_manager = Arc::clone(&self.heartbeat_manager);
        let _failure_detector = Arc::clone(&self.failure_detector);
        let running = Arc::clone(&self.running);
        let interval = self.config.heartbeat_interval;

        tokio::spawn(async move {
            while *running.read().await {
                // Check for failed heartbeats
                let manager = heartbeat_manager.read().await;
                let failed_nodes = manager.get_failed_nodes();
                drop(manager);

                for _node_id in failed_nodes {
                    // Emit event
                    // (event emission would happen here)
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Start health check loop
    async fn start_health_check_loop(&self) {
        let health_checks = Arc::clone(&self.health_checks);
        let aggregator = Arc::clone(&self.aggregator);
        let running = Arc::clone(&self.running);
        let interval = self.config.check_interval;

        tokio::spawn(async move {
            while *running.read().await {
                let checks = health_checks.read().await;

                for (node_id, node_checks) in checks.iter() {
                    for checker in node_checks {
                        match checker.check().await {
                            Ok(result) => {
                                let mut agg = aggregator.write().await;
                                let _ = agg.record_check_result(node_id.clone(), result).await;
                            }
                            Err(_) => {
                                // Handle check error
                            }
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Start metrics reporting loop
    async fn start_metrics_loop(&self) {
        let reporter = Arc::clone(&self.reporter);
        let running = Arc::clone(&self.running);
        let interval = self.config.metrics_interval;

        tokio::spawn(async move {
            while *running.read().await {
                let reporter = reporter.read().await;
                let _ = reporter.export_metrics().await;
                drop(reporter);

                tokio::time::sleep(interval).await;
            }
        });
    }
}

impl Component for HealthMonitor {
    fn initialize(&mut self) -> Result<()> {
        // Initialization would be done in start()
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Use tokio::runtime::Handle to run async code in sync context
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|e| DbError::Runtime(format!("No tokio runtime: {}", e)))?;

        handle.block_on(async { self.stop().await })
    }

    fn health_check(&self) -> HealthStatus {
        // Use tokio::runtime::Handle to run async code in sync context
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => return HealthStatus::Unknown,
        };

        let running = handle.block_on(async { *self.running.read().await });

        if running {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);
        assert_eq!(monitor.health_check(), HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_monitor_start_stop() {
        let config = HealthConfig::default();
        let mut monitor = HealthMonitor::new(config);

        assert!(monitor.start().await.is_ok());
        assert_eq!(monitor.health_check(), HealthStatus::Healthy);

        assert!(monitor.stop().await.is_ok());
        assert_eq!(monitor.health_check(), HealthStatus::Unhealthy);
    }
}
