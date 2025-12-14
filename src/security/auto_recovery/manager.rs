// # Auto-Recovery Manager
//
// Central orchestrator for all recovery components.

use crate::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tokio::time::sleep;

use super::checkpoint_management::*;
use super::recovery_strategies::*;
use super::state_restoration::*;

// Constants
/// Reserved for recovery config
#[allow(dead_code)]
const MAX_RECOVERY_TIME: Duration = Duration::from_secs(300);
const CRASH_DETECTION_TIMEOUT: Duration = Duration::from_secs(5);
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(300);
const CORRUPTION_SCAN_RATE: usize = 100;

// ============================================================================
// Auto-Recovery Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct AutoRecoveryConfig {
    pub auto_recovery_enabled: bool,
    pub max_concurrent_recoveries: usize,
    pub crash_detection_timeout: Duration,
    pub health_check_interval: Duration,
    pub checkpoint_interval: Duration,
    pub corruption_scan_rate: usize,
    pub predictive_recovery_enabled: bool,
}

impl Default for AutoRecoveryConfig {
    fn default() -> Self {
        Self {
            auto_recovery_enabled: true,
            max_concurrent_recoveries: 3,
            crash_detection_timeout: CRASH_DETECTION_TIMEOUT,
            health_check_interval: HEALTH_CHECK_INTERVAL,
            checkpoint_interval: CHECKPOINT_INTERVAL,
            corruption_scan_rate: CORRUPTION_SCAN_RATE,
            predictive_recovery_enabled: true,
        }
    }
}

// Recovery statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RecoveryStatistics {
    pub total_failures_detected: u64,
    pub total_recoveries_attempted: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub avg_rto_seconds: u64,
    pub max_rto_seconds: u64,
    pub rto_compliance_rate: f64,
    pub predictive_recoveries: u64,
}

// Auto-recovery manager
pub struct AutoRecoveryManager {
    config: AutoRecoveryConfig,
    crash_detector: Arc<CrashDetector>,
    rollback_manager: Arc<TransactionRollbackManager>,
    corruption_detector: Arc<CorruptionDetector>,
    data_repairer: Arc<DataRepairer>,
    snapshot_manager: Arc<StateSnapshotManager>,
    health_monitor: Arc<HealthMonitor>,
    self_healer: Arc<SelfHealer>,
    failures: Arc<RwLock<HashMap<u64, DetectedFailure>>>,
    next_failure_id: Arc<AtomicU64>,
    active_recoveries: Arc<RwLock<HashSet<u64>>>,
    stats: Arc<RwLock<RecoveryStatistics>>,
    shutdown: Arc<AtomicBool>,
}

impl AutoRecoveryManager {
    pub fn new(config: AutoRecoveryConfig) -> Self {
        let crash_detector = Arc::new(CrashDetector::new(config.crash_detection_timeout));
        let rollback_manager = Arc::new(TransactionRollbackManager::new());
        let corruption_detector = Arc::new(CorruptionDetector::new(config.corruption_scan_rate));
        let data_repairer = Arc::new(DataRepairer::new());
        let snapshot_manager = Arc::new(StateSnapshotManager::new(config.checkpoint_interval));
        let health_monitor = Arc::new(HealthMonitor::new(config.health_check_interval));
        let self_healer = Arc::new(SelfHealer::new());

        Self {
            config,
            crash_detector,
            rollback_manager,
            corruption_detector,
            data_repairer,
            snapshot_manager,
            health_monitor,
            self_healer,
            failures: Arc::new(RwLock::new(HashMap::new())),
            next_failure_id: Arc::new(AtomicU64::new(1)),
            active_recoveries: Arc::new(RwLock::new(HashSet::new())),
            stats: Arc::new(RwLock::new(RecoveryStatistics::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        tracing::info!("Starting auto-recovery system");

        // Setup callbacks
        {
            let manager = Arc::clone(&self);
            self.crash_detector.set_crash_callback(move |pid, reason| {
                let failure = DetectedFailure {
                    id: manager.next_failure_id.fetch_add(1, Ordering::SeqCst),
                    failure_type: FailureType::ProcessCrash,
                    severity: FailureSeverity::Critical,
                    affected_resource: format!("process_{}", pid),
                    detected_at: SystemTime::now(),
                    description: reason.clone(),
                    context: HashMap::new(),
                };
                manager.handle_failure(failure);
            });
        }

        {
            let manager = Arc::clone(&self);
            self.corruption_detector
                .set_corruption_callback(move |corruption| {
                    let failure = DetectedFailure {
                        id: manager.next_failure_id.fetch_add(1, Ordering::SeqCst),
                        failure_type: FailureType::DataCorruption,
                        severity: FailureSeverity::High,
                        affected_resource: corruption.file_path.clone(),
                        detected_at: SystemTime::now(),
                        description: format!(
                            "Page {} corrupted (checksum mismatch)",
                            corruption.page_id
                        ),
                        context: HashMap::from([
                            ("page_id".to_string(), corruption.page_id.to_string()),
                            (
                                "expected_checksum".to_string(),
                                format!("0x{:x}", corruption.expected_checksum),
                            ),
                            (
                                "actual_checksum".to_string(),
                                format!("0x{:x}", corruption.actual_checksum),
                            ),
                        ]),
                    };
                    manager.handle_failure(failure);
                });
        }

        {
            let manager = Arc::clone(&self);
            self.health_monitor.set_health_callback(move |metrics| {
                if metrics.overall_score.is_critical() {
                    let failure = DetectedFailure {
                        id: manager.next_failure_id.fetch_add(1, Ordering::SeqCst),
                        failure_type: FailureType::HealthCheckFailure,
                        severity: FailureSeverity::Critical,
                        affected_resource: "system".to_string(),
                        detected_at: SystemTime::now(),
                        description: format!("Critical health score: {}", metrics.overall_score.0),
                        context: HashMap::new(),
                    };
                    manager.handle_failure(failure);
                }
            });
        }

        // Start all monitoring tasks
        tokio::spawn(Arc::clone(&self.crash_detector).start_monitoring());
        tokio::spawn(Arc::clone(&self.corruption_detector).start_scanning());
        tokio::spawn(Arc::clone(&self.snapshot_manager).start_checkpointing());
        tokio::spawn(Arc::clone(&self.health_monitor).start_monitoring());

        // Start recovery orchestration loop
        tokio::spawn(Arc::clone(&self).recovery_orchestration_loop());

        // Start predictive recovery loop
        if self.config.predictive_recovery_enabled {
            tokio::spawn(Arc::clone(&self).predictive_recovery_loop());
        }

        tracing::info!("Auto-recovery system started successfully");

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping auto-recovery system");
        self.shutdown.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn handle_failure(&self, failure: DetectedFailure) {
        tracing::warn!(
            "Failure detected: {:?} - {}",
            failure.failure_type,
            failure.description
        );

        self.failures.write().insert(failure.id, failure.clone());

        self.stats.write().total_failures_detected += 1;

        if self.config.auto_recovery_enabled {
            let manager = Arc::new(self.clone());
            tokio::spawn(async move {
                if let Err(e) = manager.recover_from_failure(failure).await {
                    tracing::error!("Recovery failed: {}", e);
                }
            });
        }
    }

    async fn recover_from_failure(&self, failure: DetectedFailure) -> Result<()> {
        let should_delay = {
            let active = self.active_recoveries.read();
            active.len() >= self.config.max_concurrent_recoveries
        };

        if should_delay {
            tracing::warn!("Recovery delayed: too many concurrent recoveries");
            sleep(Duration::from_secs(5)).await;
        }

        self.active_recoveries.write().insert(failure.id);

        let start = Instant::now();
        tracing::info!(
            "Starting recovery for failure {}: {:?}",
            failure.id,
            failure.failure_type
        );

        let result = match failure.failure_type {
            FailureType::ProcessCrash => self.self_healer.diagnose_and_heal(&failure).await,
            FailureType::DataCorruption => self.recover_corrupted_data(&failure).await,
            FailureType::TransactionDeadlock => self.recover_from_deadlock(&failure).await,
            FailureType::MemoryExhaustion => self.self_healer.diagnose_and_heal(&failure).await,
            FailureType::ConnectionPoolExhaustion => {
                self.self_healer.diagnose_and_heal(&failure).await
            }
            FailureType::HealthCheckFailure => self.recover_from_health_failure(&failure).await,
            _ => {
                tracing::warn!(
                    "No automatic recovery for failure type: {:?}",
                    failure.failure_type
                );
                Ok(false)
            }
        };

        self.active_recoveries.write().remove(&failure.id);

        let rto_seconds = start.elapsed().as_secs();

        {
            let mut stats = self.stats.write();
            stats.total_recoveries_attempted += 1;

            if result.is_ok() && result.as_ref().unwrap() == &true {
                stats.successful_recoveries += 1;
                stats.avg_rto_seconds = (stats.avg_rto_seconds + rto_seconds) / 2;
                stats.max_rto_seconds = stats.max_rto_seconds.max(rto_seconds);

                if rto_seconds <= 120 {
                    stats.rto_compliance_rate = (stats.rto_compliance_rate
                        * (stats.successful_recoveries - 1) as f64
                        + 1.0)
                        / stats.successful_recoveries as f64;
                }

                tracing::info!(
                    "Recovery completed successfully in {}s (RTO target: 120s)",
                    rto_seconds
                );
            } else {
                stats.failed_recoveries += 1;
                tracing::error!("Recovery failed after {}s", rto_seconds);
            }
        }

        result.map(|_| ())
    }

    async fn recover_corrupted_data(&self, failure: &DetectedFailure) -> Result<bool> {
        if let Some(page_id_str) = failure.context.get("page_id") {
            let page_id: u64 = page_id_str
                .parse()
                .map_err(|_| DbError::InvalidInput("Invalid page_id".to_string()))?;

            self.data_repairer
                .repair_page(page_id, &failure.affected_resource)
                .await?;

            self.corruption_detector.mark_repaired(page_id)?;

            Ok(true)
        } else {
            Err(DbError::InvalidInput(
                "Missing page_id in context".to_string(),
            ))
        }
    }

    async fn recover_from_deadlock(&self, _failure: &DetectedFailure) -> Result<bool> {
        tracing::info!("Recovering from deadlock by rolling back transactions");

        let count = self.rollback_manager.rollback_all_inflight().await?;

        tracing::info!("Rolled back {} transactions to resolve deadlock", count);
        Ok(true)
    }

    async fn recover_from_health_failure(&self, _failure: &DetectedFailure) -> Result<bool> {
        tracing::info!("Recovering from health failure");

        self.self_healer.clear_caches().await?;

        self.snapshot_manager.create_checkpoint().await?;

        Ok(true)
    }

    async fn recovery_orchestration_loop(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(10));

        while !self.shutdown.load(Ordering::SeqCst) {
            interval.tick().await;
        }
    }

    async fn predictive_recovery_loop(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(60));

        while !self.shutdown.load(Ordering::SeqCst) {
            interval.tick().await;

            let failure_probability = self.health_monitor.predict_failure_probability();

            if failure_probability > 0.7 {
                tracing::warn!(
                    "High failure probability detected: {:.1}%",
                    failure_probability * 100.0
                );

                if let Err(e) = self.take_preventive_action().await {
                    tracing::error!("Preventive action failed: {}", e);
                } else {
                    self.stats.write().predictive_recoveries += 1;
                }
            }
        }
    }

    async fn take_preventive_action(&self) -> Result<()> {
        tracing::info!("Taking preventive action");

        self.snapshot_manager.create_checkpoint().await?;

        self.self_healer.clear_caches().await?;

        Ok(())
    }

    pub fn get_statistics(&self) -> RecoveryStatistics {
        self.stats.read().clone()
    }

    pub fn get_comprehensive_statistics(&self) -> ComprehensiveRecoveryStats {
        ComprehensiveRecoveryStats {
            recovery: self.stats.read().clone(),
            crash: self.crash_detector.get_statistics(),
            rollback: self.rollback_manager.get_statistics(),
            corruption: self.corruption_detector.get_statistics(),
            repair: self.data_repairer.get_statistics(),
            snapshot: self.snapshot_manager.get_statistics(),
            healing: self.self_healer.get_statistics(),
        }
    }
}

impl Clone for AutoRecoveryManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            crash_detector: Arc::clone(&self.crash_detector),
            rollback_manager: Arc::clone(&self.rollback_manager),
            corruption_detector: Arc::clone(&self.corruption_detector),
            data_repairer: Arc::clone(&self.data_repairer),
            snapshot_manager: Arc::clone(&self.snapshot_manager),
            health_monitor: Arc::clone(&self.health_monitor),
            self_healer: Arc::clone(&self.self_healer),
            failures: Arc::clone(&self.failures),
            next_failure_id: Arc::clone(&self.next_failure_id),
            active_recoveries: Arc::clone(&self.active_recoveries),
            stats: Arc::clone(&self.stats),
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

// Comprehensive recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveRecoveryStats {
    pub recovery: RecoveryStatistics,
    pub crash: CrashStats,
    pub rollback: RollbackStats,
    pub corruption: CorruptionStats,
    pub repair: RepairStats,
    pub snapshot: SnapshotStats,
    pub healing: HealingStats,
}
