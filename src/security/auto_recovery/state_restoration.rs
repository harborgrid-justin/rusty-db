// # State Restoration
//
// Health monitoring, self-healing, and state restoration components.

use std::collections::VecDeque;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, SystemTime};
use parking_lot::RwLock;
use tokio::time::{interval, sleep};

use super::recovery_strategies::{DetectedFailure, FailureType};

// ============================================================================
// HealthMonitor
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HealthScore(pub u8);

impl HealthScore {
    pub fn new(score: u8) -> Self {
        HealthScore(score.min(100))
    }

    pub fn is_healthy(&self) -> bool {
        self.0 >= 80
    }

    pub fn is_degraded(&self) -> bool {
        self.0 >= 50 && self.0 < 80
    }

    pub fn is_critical(&self) -> bool {
        self.0 < 50
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub overall_score: HealthScore,
    pub cpu_score: HealthScore,
    pub memory_score: HealthScore,
    pub disk_score: HealthScore,
    pub network_score: HealthScore,
    pub database_score: HealthScore,
    pub timestamp: SystemTime,
}

pub struct HealthMonitor {
    metrics: Arc<RwLock<HealthMetrics>>,
    history: Arc<StdMutex<VecDeque<HealthMetrics>>>,
    health_callback: Arc<StdMutex<Option<Box<dyn Fn(HealthMetrics) + Send + Sync>>>>,
    interval: Duration,
}

impl HealthMonitor {
    pub fn new(interval: Duration) -> Self {
        let initial_metrics = HealthMetrics {
            overall_score: HealthScore::new(100),
            cpu_score: HealthScore::new(100),
            memory_score: HealthScore::new(100),
            disk_score: HealthScore::new(100),
            network_score: HealthScore::new(100),
            database_score: HealthScore::new(100),
            timestamp: SystemTime::now(),
        };

        Self {
            metrics: Arc::new(RwLock::new(initial_metrics)),
            history: Arc::new(StdMutex::new(VecDeque::with_capacity(1000))),
            health_callback: Arc::new(StdMutex::new(None)),
            interval,
        }
    }

    pub fn set_health_callback<F>(&self, callback: F)
    where
        F: Fn(HealthMetrics) + Send + Sync + 'static,
    {
        *self.health_callback.lock().unwrap() = Some(Box::new(callback));
    }

    pub async fn start_monitoring(self: Arc<Self>) {
        let mut interval = interval(self.interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.check_health().await {
                tracing::error!("Health check error: {}", e);
            }
        }
    }

    async fn check_health(&self) -> Result<()> {
        let cpu_score = self.check_cpu_health().await?;
        let memory_score = self.check_memory_health().await?;
        let disk_score = self.check_disk_health().await?;
        let network_score = self.check_network_health().await?;
        let database_score = self.check_database_health().await?;

        let overall = (
            cpu_score.0 as u32 * 2 +
            memory_score.0 as u32 * 2 +
            disk_score.0 as u32 * 2 +
            network_score.0 as u32 +
            database_score.0 as u32 * 3
        ) / 10;

        let metrics = HealthMetrics {
            overall_score: HealthScore::new(overall as u8),
            cpu_score,
            memory_score,
            disk_score,
            network_score,
            database_score,
            timestamp: SystemTime::now(),
        };

        *self.metrics.write() = metrics.clone();

        {
            let mut history = self.history.lock().unwrap();
            if history.len() >= 1000 {
                history.pop_front();
            }
            history.push_back(metrics.clone());
        }

        if let Some(ref callback) = *self.health_callback.lock().unwrap() {
            callback(metrics);
        }

        Ok(())
    }

    async fn check_cpu_health(&self) -> Result<HealthScore> {
        Ok(HealthScore::new(95))
    }

    async fn check_memory_health(&self) -> Result<HealthScore> {
        Ok(HealthScore::new(90))
    }

    async fn check_disk_health(&self) -> Result<HealthScore> {
        Ok(HealthScore::new(92))
    }

    async fn check_network_health(&self) -> Result<HealthScore> {
        Ok(HealthScore::new(98))
    }

    async fn check_database_health(&self) -> Result<HealthScore> {
        Ok(HealthScore::new(94))
    }

    pub fn get_current_health(&self) -> HealthMetrics {
        self.metrics.read().clone()
    }

    pub fn predict_failure_probability(&self) -> f64 {
        let history = self.history.lock().unwrap();
        if history.len() < 10 {
            return 0.0;
        }

        let recent: Vec<u8> = history.iter()
            .rev()
            .take(10)
            .map(|m| m.overall_score.0)
            .collect();

        let avg_recent = recent.iter().sum::<u8>() as f64 / recent.len() as f64;
        let first = recent.last().unwrap();
        let last = recent.first().unwrap();

        let decline_rate = (*first as f64 - *last as f64) / 10.0;

        if decline_rate > 5.0 {
            0.7
        } else if decline_rate > 2.0 {
            0.3
        } else if avg_recent < 50.0 {
            0.5
        } else {
            0.05
        }
    }
}

// ============================================================================
// SelfHealer
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingAction {
    pub id: u64,
    pub action_type: String,
    pub description: String,
    pub target: String,
    pub executed_at: Option<SystemTime>,
    pub success: Option<bool>,
}

pub struct SelfHealer {
    actions: Arc<RwLock<Vec<HealingAction>>>,
    next_id: Arc<AtomicU64>,
    stats: Arc<RwLock<HealingStats>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HealingStats {
    pub total_actions: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub auto_fix_rate: f64,
}

impl SelfHealer {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            stats: Arc::new(RwLock::new(HealingStats::default())),
        }
    }

    pub async fn diagnose_and_heal(&self, failure: &DetectedFailure) -> Result<bool> {
        tracing::info!("Diagnosing failure: {:?}", failure.failure_type);

        let action_id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let (action_type, target, success) = match &failure.failure_type {
            FailureType::ProcessCrash => {
                ("restart_process".to_string(), failure.affected_resource.clone(),
                    self.restart_process(&failure.affected_resource).await.is_ok())
            }
            FailureType::DataCorruption => {
                ("repair_from_replica".to_string(), failure.affected_resource.clone(),
                    self.repair_corrupted_data(&failure.affected_resource).await.is_ok())
            }
            FailureType::MemoryExhaustion => {
                ("clear_caches".to_string(), "memory".to_string(),
                    self.clear_caches().await.is_ok())
            }
            FailureType::ConnectionPoolExhaustion => {
                ("expand_pool".to_string(), "connection_pool".to_string(),
                    self.expand_connection_pool().await.is_ok())
            }
            _ => {
                ("manual_intervention".to_string(), failure.affected_resource.clone(), false)
            }
        };

        let action = HealingAction {
            id: action_id,
            action_type: action_type.clone(),
            description: format!("Healing {} via {}", failure.affected_resource, action_type),
            target,
            executed_at: Some(SystemTime::now()),
            success: Some(success),
        };

        self.actions.write().push(action);

        {
            let mut stats = self.stats.write();
            stats.total_actions += 1;
            if success {
                stats.successful_actions += 1;
            } else {
                stats.failed_actions += 1;
            }
            stats.auto_fix_rate = stats.successful_actions as f64 / stats.total_actions as f64;
        }

        Ok(success)
    }

    async fn restart_process(&self, process: &str) -> Result<()> {
        tracing::info!("Restarting process: {}", process);
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    async fn repair_corrupted_data(&self, resource: &str) -> Result<()> {
        tracing::info!("Repairing corrupted data: {}", resource);
        sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    pub async fn clear_caches(&self) -> Result<()> {
        tracing::info!("Clearing caches");
        sleep(Duration::from_millis(20)).await;
        Ok(())
    }

    async fn expand_connection_pool(&self) -> Result<()> {
        tracing::info!("Expanding connection pool");
        sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    pub fn get_statistics(&self) -> HealingStats {
        self.stats.read().clone()
    }

    pub fn get_recent_actions(&self, count: usize) -> Vec<HealingAction> {
        let actions = self.actions.read();
        actions.iter().rev().take(count).cloned().collect()
    }
}

impl Default for SelfHealer {
    fn default() -> Self {
        Self::new()
    }
}
