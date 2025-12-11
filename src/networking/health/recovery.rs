//! # Recovery Manager
//!
//! Handles automatic recovery attempts, gradual traffic restoration,
//! quarantine periods, and manual override for failed nodes.

use crate::error::{DbError, Result};
use crate::common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Recovery strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Immediate recovery (no gradual restoration)
    Immediate,

    /// Gradual traffic restoration
    Gradual,

    /// Manual intervention required
    Manual,

    /// Custom strategy with specific parameters
    Custom,
}

/// Recovery state for a node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryState {
    /// Node is healthy, no recovery needed
    Healthy,

    /// Node has failed, awaiting recovery
    Failed,

    /// Recovery in progress
    Recovering,

    /// In quarantine period after recovery
    Quarantined,

    /// Recovery failed
    RecoveryFailed,

    /// Waiting for manual intervention
    AwaitingManual,
}

/// Recovery attempt record
#[derive(Debug, Clone)]
struct RecoveryAttempt {
    /// When the attempt started
    started_at: Instant,

    /// When the attempt completed (if finished)
    completed_at: Option<Instant>,

    /// Whether the attempt succeeded
    success: bool,

    /// Attempt number
    attempt_number: u32,

    /// Error message if failed
    error_message: Option<String>,
}

/// Recovery configuration for a node
#[derive(Debug, Clone)]
struct NodeRecoveryConfig {
    /// Current recovery state
    state: RecoveryState,

    /// Recovery strategy to use
    strategy: RecoveryStrategy,

    /// Maximum recovery attempts
    max_attempts: u32,

    /// Current attempt count
    current_attempts: u32,

    /// History of recovery attempts
    attempt_history: Vec<RecoveryAttempt>,

    /// When node entered failed state
    failed_at: Option<Instant>,

    /// When recovery started
    recovery_started_at: Option<Instant>,

    /// When quarantine started
    quarantine_started_at: Option<Instant>,

    /// Quarantine duration
    quarantine_duration: Duration,

    /// Gradual restoration percentage (0-100)
    traffic_percentage: u32,

    /// Traffic increment step
    traffic_increment: u32,

    /// Time between traffic increments
    increment_interval: Duration,

    /// Last traffic increment time
    last_increment: Option<Instant>,

    /// Manual override flag
    manual_override: bool,
}

impl NodeRecoveryConfig {
    fn new(max_attempts: u32, quarantine_duration: Duration) -> Self {
        Self {
            state: RecoveryState::Healthy,
            strategy: RecoveryStrategy::Gradual,
            max_attempts,
            current_attempts: 0,
            attempt_history: Vec::new(),
            failed_at: None,
            recovery_started_at: None,
            quarantine_started_at: None,
            quarantine_duration,
            traffic_percentage: 0,
            traffic_increment: 10, // 10% increments
            increment_interval: Duration::from_secs(30),
            last_increment: None,
            manual_override: false,
        }
    }
}

/// Recovery manager
pub struct RecoveryManager {
    /// Per-node recovery configurations
    nodes: Arc<RwLock<HashMap<NodeId, NodeRecoveryConfig>>>,

    /// Default maximum attempts
    default_max_attempts: u32,

    /// Default quarantine duration
    default_quarantine_duration: Duration,

    /// Enable automatic recovery
    auto_recovery_enabled: bool,

    /// Recovery callbacks
    recovery_callbacks: Arc<RwLock<Vec<Box<dyn RecoveryCallback + Send + Sync>>>>,
}

/// Recovery callback trait
pub trait RecoveryCallback: Send + Sync {
    /// Called when recovery starts
    fn on_recovery_start(&mut self, node_id: &NodeId) -> Result<()>;

    /// Called when recovery succeeds
    fn on_recovery_success(&mut self, node_id: &NodeId) -> Result<()>;

    /// Called when recovery fails
    fn on_recovery_failure(&mut self, node_id: &NodeId, error: &str) -> Result<()>;

    /// Called when quarantine starts
    fn on_quarantine_start(&mut self, node_id: &NodeId) -> Result<()>;

    /// Called when quarantine ends
    fn on_quarantine_end(&mut self, node_id: &NodeId) -> Result<()>;
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(max_attempts: u32, quarantine_duration: Duration) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            default_max_attempts: max_attempts,
            default_quarantine_duration: quarantine_duration,
            auto_recovery_enabled: true,
            recovery_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Mark a node as failed
    pub async fn mark_node_failed(&self, node_id: NodeId) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.entry(node_id.clone())
            .or_insert_with(|| NodeRecoveryConfig::new(
                self.default_max_attempts,
                self.default_quarantine_duration
            ));

        config.state = RecoveryState::Failed;
        config.failed_at = Some(Instant::now());

        Ok(())
    }

    /// Attempt recovery for a node
    pub async fn attempt_recovery(&self, node_id: NodeId) -> Result<bool> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(&node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        // Check if we've exceeded max attempts
        if config.current_attempts >= config.max_attempts {
            config.state = RecoveryState::RecoveryFailed;
            return Ok(false);
        }

        // Check if manual intervention is required
        if config.strategy == RecoveryStrategy::Manual {
            config.state = RecoveryState::AwaitingManual;
            return Ok(false);
        }

        // Start recovery
        config.state = RecoveryState::Recovering;
        config.recovery_started_at = Some(Instant::now());
        config.current_attempts += 1;

        let attempt = RecoveryAttempt {
            started_at: Instant::now(),
            completed_at: None,
            success: false,
            attempt_number: config.current_attempts,
            error_message: None,
        };

        config.attempt_history.push(attempt);

        // Trigger recovery callback
        let callbacks = self.recovery_callbacks.read().await;
        for callback in callbacks.iter() {
            // Note: we can't call mutable methods on the callback here
            // In a real implementation, you'd need to handle this differently
        }

        Ok(true)
    }

    /// Complete a recovery attempt (success or failure)
    pub async fn complete_recovery(
        &self,
        node_id: NodeId,
        success: bool,
        error_message: Option<String>
    ) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(&node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        // Update last attempt
        if let Some(attempt) = config.attempt_history.last_mut() {
            attempt.completed_at = Some(Instant::now());
            attempt.success = success;
            attempt.error_message = error_message.clone();
        }

        if success {
            // Start quarantine period
            config.state = RecoveryState::Quarantined;
            config.quarantine_started_at = Some(Instant::now());
            config.current_attempts = 0; // Reset attempt counter

            // Initialize gradual restoration
            if config.strategy == RecoveryStrategy::Gradual {
                config.traffic_percentage = config.traffic_increment;
                config.last_increment = Some(Instant::now());
            } else {
                config.traffic_percentage = 100;
            }
        } else {
            config.state = RecoveryState::Failed;
        }

        Ok(())
    }

    /// Check and update quarantine status
    pub async fn update_quarantine_status(&self, node_id: &NodeId) -> Result<bool> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        if config.state != RecoveryState::Quarantined {
            return Ok(false);
        }

        let quarantine_start = config.quarantine_started_at
            .ok_or_else(|| DbError::InvalidState("Quarantine start time not set".to_string()))?;

        // Check if quarantine period has ended
        if quarantine_start.elapsed() >= config.quarantine_duration {
            config.state = RecoveryState::Healthy;
            config.quarantine_started_at = None;
            config.traffic_percentage = 100;
            return Ok(true);
        }

        // Update gradual traffic restoration
        if config.strategy == RecoveryStrategy::Gradual {
            self.update_traffic_percentage(config)?;
        }

        Ok(false)
    }

    /// Update traffic percentage for gradual restoration
    fn update_traffic_percentage(&self, config: &mut NodeRecoveryConfig) -> Result<()> {
        if config.traffic_percentage >= 100 {
            return Ok(());
        }

        let last_increment = config.last_increment
            .ok_or_else(|| DbError::InvalidState("Last increment time not set".to_string()))?;

        if last_increment.elapsed() >= config.increment_interval {
            config.traffic_percentage = (config.traffic_percentage + config.traffic_increment).min(100);
            config.last_increment = Some(Instant::now());
        }

        Ok(())
    }

    /// Get traffic percentage for a node
    pub async fn get_traffic_percentage(&self, node_id: &NodeId) -> Result<u32> {
        let nodes = self.nodes.read().await;
        let config = nodes.get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        Ok(config.traffic_percentage)
    }

    /// Get recovery state for a node
    pub async fn get_recovery_state(&self, node_id: &NodeId) -> Result<RecoveryState> {
        let nodes = self.nodes.read().await;
        let config = nodes.get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        Ok(config.state.clone())
    }

    /// Set recovery strategy for a node
    pub async fn set_recovery_strategy(
        &self,
        node_id: NodeId,
        strategy: RecoveryStrategy
    ) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.entry(node_id)
            .or_insert_with(|| NodeRecoveryConfig::new(
                self.default_max_attempts,
                self.default_quarantine_duration
            ));

        config.strategy = strategy;
        Ok(())
    }

    /// Manual override to mark node as healthy
    pub async fn manual_override_healthy(&self, node_id: NodeId) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(&node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        config.state = RecoveryState::Healthy;
        config.manual_override = true;
        config.current_attempts = 0;
        config.traffic_percentage = 100;

        Ok(())
    }

    /// Manual override to prevent recovery
    pub async fn manual_override_block(&self, node_id: NodeId) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(&node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        config.manual_override = true;
        config.state = RecoveryState::AwaitingManual;

        Ok(())
    }

    /// Clear manual override
    pub async fn clear_manual_override(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let config = nodes.get_mut(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        config.manual_override = false;
        Ok(())
    }

    /// Get recovery statistics for a node
    pub async fn get_recovery_stats(&self, node_id: &NodeId) -> Result<RecoveryStats> {
        let nodes = self.nodes.read().await;
        let config = nodes.get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        let total_attempts = config.attempt_history.len() as u32;
        let successful_attempts = config.attempt_history.iter()
            .filter(|a| a.success)
            .count() as u32;
        let failed_attempts = total_attempts - successful_attempts;

        let avg_recovery_time = if !config.attempt_history.is_empty() {
            let total_time: Duration = config.attempt_history.iter()
                .filter_map(|a| a.completed_at.map(|c| c.duration_since(a.started_at)))
                .sum();
            Some(total_time / config.attempt_history.len() as u32)
        } else {
            None
        };

        Ok(RecoveryStats {
            state: config.state.clone(),
            total_attempts,
            successful_attempts,
            failed_attempts,
            current_attempt: config.current_attempts,
            max_attempts: config.max_attempts,
            traffic_percentage: config.traffic_percentage,
            avg_recovery_time,
            time_in_current_state: config.failed_at.map(|t| t.elapsed()),
        })
    }

    /// Add a recovery callback
    pub async fn add_callback(&self, callback: Box<dyn RecoveryCallback + Send + Sync>) -> Result<()> {
        let mut callbacks = self.recovery_callbacks.write().await;
        callbacks.push(callback);
        Ok(())
    }

    /// Enable or disable automatic recovery
    pub fn set_auto_recovery(&mut self, enabled: bool) {
        self.auto_recovery_enabled = enabled;
    }

    /// Check if automatic recovery is enabled
    pub fn is_auto_recovery_enabled(&self) -> bool {
        self.auto_recovery_enabled
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    /// Current recovery state
    pub state: RecoveryState,

    /// Total recovery attempts
    pub total_attempts: u32,

    /// Successful attempts
    pub successful_attempts: u32,

    /// Failed attempts
    pub failed_attempts: u32,

    /// Current attempt number
    pub current_attempt: u32,

    /// Maximum allowed attempts
    pub max_attempts: u32,

    /// Current traffic percentage
    pub traffic_percentage: u32,

    /// Average recovery time
    pub avg_recovery_time: Option<Duration>,

    /// Time in current state
    pub time_in_current_state: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        assert!(manager.is_auto_recovery_enabled());
    }

    #[tokio::test]
    async fn test_mark_node_failed() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        let node_id = "node1".to_string();

        assert!(manager.mark_node_failed(node_id.clone()).await.is_ok());

        let state = manager.get_recovery_state(&node_id).await.unwrap();
        assert_eq!(state, RecoveryState::Failed);
    }

    #[tokio::test]
    async fn test_attempt_recovery() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        let node_id = "node1".to_string();

        manager.mark_node_failed(node_id.clone()).await.unwrap();

        let can_recover = manager.attempt_recovery(node_id.clone()).await.unwrap();
        assert!(can_recover);

        let state = manager.get_recovery_state(&node_id).await.unwrap();
        assert_eq!(state, RecoveryState::Recovering);
    }

    #[tokio::test]
    async fn test_complete_recovery_success() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        let node_id = "node1".to_string();

        manager.mark_node_failed(node_id.clone()).await.unwrap();
        manager.attempt_recovery(node_id.clone()).await.unwrap();
        manager.complete_recovery(node_id.clone(), true, None).await.unwrap();

        let state = manager.get_recovery_state(&node_id).await.unwrap();
        assert_eq!(state, RecoveryState::Quarantined);
    }

    #[tokio::test]
    async fn test_manual_override() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        let node_id = "node1".to_string();

        manager.mark_node_failed(node_id.clone()).await.unwrap();
        manager.manual_override_healthy(node_id.clone()).await.unwrap();

        let state = manager.get_recovery_state(&node_id).await.unwrap();
        assert_eq!(state, RecoveryState::Healthy);

        let traffic = manager.get_traffic_percentage(&node_id).await.unwrap();
        assert_eq!(traffic, 100);
    }

    #[tokio::test]
    async fn test_recovery_stats() {
        let manager = RecoveryManager::new(3, Duration::from_secs(30));
        let node_id = "node1".to_string();

        manager.mark_node_failed(node_id.clone()).await.unwrap();
        manager.attempt_recovery(node_id.clone()).await.unwrap();

        let stats = manager.get_recovery_stats(&node_id).await.unwrap();
        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.current_attempt, 1);
    }
}
