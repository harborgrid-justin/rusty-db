//! # Heartbeat Management
//!
//! Implements heartbeat tracking with RTT measurement, failure counting,
//! and adaptive intervals for distributed node health monitoring.

use crate::error::{DbError, Result};
use crate::common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Peer status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Peer is healthy and responsive
    Healthy,

    /// Peer is suspected of failure
    Suspected,

    /// Peer has failed
    Failed,

    /// Peer is in recovery
    Recovering,

    /// Peer is in quarantine after recovery
    Quarantined,

    /// Peer status unknown
    Unknown,
}

/// Heartbeat state for a single peer
#[derive(Debug, Clone)]
pub struct HeartbeatState {
    /// Last time a heartbeat was received
    pub last_seen: Instant,

    /// Round-trip time of last heartbeat
    pub last_latency: Duration,

    /// Average latency over recent heartbeats
    pub avg_latency: Duration,

    /// Number of consecutive failed heartbeats
    pub consecutive_failures: u32,

    /// Total heartbeats received
    pub total_heartbeats: u64,

    /// Total heartbeats missed
    pub total_missed: u64,

    /// Current status of the peer
    pub status: PeerStatus,

    /// Adaptive interval (can be adjusted based on network conditions)
    pub adaptive_interval: Duration,

    /// Latency history for calculation
    latency_history: Vec<Duration>,

    /// Maximum history size
    max_history_size: usize,
}

impl HeartbeatState {
    /// Create a new heartbeat state
    pub fn new(adaptive_interval: Duration) -> Self {
        Self {
            last_seen: Instant::now(),
            last_latency: Duration::from_secs(0),
            avg_latency: Duration::from_secs(0),
            consecutive_failures: 0,
            total_heartbeats: 0,
            total_missed: 0,
            status: PeerStatus::Unknown,
            adaptive_interval,
            latency_history: Vec::with_capacity(100),
            max_history_size: 100,
        }
    }

    /// Record a successful heartbeat
    pub fn record_heartbeat(&mut self, latency: Duration) {
        self.last_seen = Instant::now();
        self.last_latency = latency;
        self.consecutive_failures = 0;
        self.total_heartbeats += 1;

        // Update latency history
        self.latency_history.push(latency);
        if self.latency_history.len() > self.max_history_size {
            self.latency_history.remove(0);
        }

        // Update average latency
        self.update_avg_latency();

        // Adapt interval based on latency
        self.adapt_interval();

        // Update status
        if self.status != PeerStatus::Healthy && self.status != PeerStatus::Quarantined {
            self.status = PeerStatus::Healthy;
        }
    }

    /// Record a missed heartbeat
    pub fn record_miss(&mut self) {
        self.consecutive_failures += 1;
        self.total_missed += 1;

        // Update status based on consecutive failures
        if self.consecutive_failures >= 3 {
            self.status = PeerStatus::Failed;
        } else if self.consecutive_failures >= 1 {
            self.status = PeerStatus::Suspected;
        }
    }

    /// Check if the heartbeat has timed out
    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }

    /// Get time since last heartbeat
    pub fn time_since_last_heartbeat(&self) -> Duration {
        self.last_seen.elapsed()
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_heartbeats + self.total_missed;
        if total == 0 {
            0.0
        } else {
            self.total_heartbeats as f64 / total as f64
        }
    }

    /// Update average latency
    fn update_avg_latency(&mut self) {
        if self.latency_history.is_empty() {
            return;
        }

        let sum: Duration = self.latency_history.iter().sum();
        self.avg_latency = sum / self.latency_history.len() as u32;
    }

    /// Adapt interval based on network conditions
    fn adapt_interval(&mut self) {
        // Increase interval if network is stable (low jitter)
        // Decrease interval if network is unstable (high jitter)
        if self.latency_history.len() < 10 {
            return;
        }

        let jitter = self.calculate_jitter();
        let jitter_threshold = Duration::from_millis(50);

        if jitter < jitter_threshold {
            // Stable network, can increase interval slightly
            let max_interval = self.adaptive_interval.mul_f64(1.5);
            if self.adaptive_interval < max_interval {
                self.adaptive_interval = self.adaptive_interval.mul_f64(1.1);
            }
        } else {
            // Unstable network, decrease interval
            let min_interval = Duration::from_millis(500);
            if self.adaptive_interval > min_interval {
                self.adaptive_interval = self.adaptive_interval.mul_f64(0.9);
            }
        }
    }

    /// Calculate latency jitter (standard deviation)
    fn calculate_jitter(&self) -> Duration {
        if self.latency_history.len() < 2 {
            return Duration::from_secs(0);
        }

        let avg = self.avg_latency.as_secs_f64();
        let variance: f64 = self.latency_history
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - avg;
                diff * diff
            })
            .sum::<f64>() / self.latency_history.len() as f64;

        Duration::from_secs_f64(variance.sqrt())
    }
}

/// Heartbeat manager for tracking multiple peers
pub struct HeartbeatManager {
    /// Heartbeat interval
    interval: Duration,

    /// Heartbeat timeout
    timeout: Duration,

    /// Peer heartbeat states
    peers: HashMap<NodeId, HeartbeatState>,

    /// Enable adaptive intervals
    adaptive_intervals: bool,
}

impl HeartbeatManager {
    /// Create a new heartbeat manager
    pub fn new(interval: Duration, timeout: Duration) -> Self {
        Self {
            interval,
            timeout,
            peers: HashMap::new(),
            adaptive_intervals: true,
        }
    }

    /// Register a new peer
    pub fn register_peer(&mut self, node_id: NodeId) -> Result<()> {
        if self.peers.contains_key(&node_id) {
            return Err(DbError::AlreadyExists(format!("Peer {} already registered", node_id)));
        }

        let state = HeartbeatState::new(self.interval);
        self.peers.insert(node_id, state);
        Ok(())
    }

    /// Unregister a peer
    pub fn unregister_peer(&mut self, node_id: &NodeId) -> Result<()> {
        self.peers.remove(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Peer {} not found", node_id)))?;
        Ok(())
    }

    /// Record a heartbeat from a peer
    pub fn record_heartbeat(&mut self, node_id: NodeId) -> Result<()> {
        self.record_heartbeat_with_latency(node_id, Duration::from_secs(0))
    }

    /// Record a heartbeat with measured latency
    pub fn record_heartbeat_with_latency(&mut self, node_id: NodeId, latency: Duration) -> Result<()> {
        let state = self.peers.entry(node_id.clone())
            .or_insert_with(|| HeartbeatState::new(self.interval));

        state.record_heartbeat(latency);
        Ok(())
    }

    /// Check all peers for timeouts
    pub fn check_timeouts(&mut self) -> Vec<NodeId> {
        let mut timed_out = Vec::new();

        for (node_id, state) in self.peers.iter_mut() {
            if state.is_timed_out(self.timeout) {
                state.record_miss();
                timed_out.push(node_id.clone());
            }
        }

        timed_out
    }

    /// Get heartbeat state for a peer
    pub fn get_peer_state(&self, node_id: &NodeId) -> Option<&HeartbeatState> {
        self.peers.get(node_id)
    }

    /// Get mutable heartbeat state for a peer
    pub fn get_peer_state_mut(&mut self, node_id: &NodeId) -> Option<&mut HeartbeatState> {
        self.peers.get_mut(node_id)
    }

    /// Get all failed nodes
    pub fn get_failed_nodes(&self) -> Vec<NodeId> {
        self.peers.iter()
            .filter(|(_, state)| state.status == PeerStatus::Failed)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all suspected nodes
    pub fn get_suspected_nodes(&self) -> Vec<NodeId> {
        self.peers.iter()
            .filter(|(_, state)| state.status == PeerStatus::Suspected)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all healthy nodes
    pub fn get_healthy_nodes(&self) -> Vec<NodeId> {
        self.peers.iter()
            .filter(|(_, state)| state.status == PeerStatus::Healthy)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get average latency across all peers
    pub fn avg_cluster_latency(&self) -> Duration {
        if self.peers.is_empty() {
            return Duration::from_secs(0);
        }

        let sum: Duration = self.peers.values()
            .map(|s| s.avg_latency)
            .sum();

        sum / self.peers.len() as u32
    }

    /// Get cluster health score (0.0 to 1.0)
    pub fn cluster_health_score(&self) -> f64 {
        if self.peers.is_empty() {
            return 1.0;
        }

        let total_score: f64 = self.peers.values()
            .map(|s| s.success_rate())
            .sum();

        total_score / self.peers.len() as f64
    }

    /// Set adaptive intervals enabled/disabled
    pub fn set_adaptive_intervals(&mut self, enabled: bool) {
        self.adaptive_intervals = enabled;
    }

    /// Get adaptive interval for a peer
    pub fn get_adaptive_interval(&self, node_id: &NodeId) -> Option<Duration> {
        if self.adaptive_intervals {
            self.peers.get(node_id).map(|s| s.adaptive_interval)
        } else {
            Some(self.interval)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_state_creation() {
        let state = HeartbeatState::new(Duration::from_secs(1));
        assert_eq!(state.status, PeerStatus::Unknown);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.total_heartbeats, 0);
    }

    #[test]
    fn test_record_heartbeat() {
        let mut state = HeartbeatState::new(Duration::from_secs(1));
        state.record_heartbeat(Duration::from_millis(10));

        assert_eq!(state.status, PeerStatus::Healthy);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.total_heartbeats, 1);
        assert_eq!(state.last_latency, Duration::from_millis(10));
    }

    #[test]
    fn test_record_miss() {
        let mut state = HeartbeatState::new(Duration::from_secs(1));

        state.record_miss();
        assert_eq!(state.status, PeerStatus::Suspected);
        assert_eq!(state.consecutive_failures, 1);

        state.record_miss();
        state.record_miss();
        assert_eq!(state.status, PeerStatus::Failed);
        assert_eq!(state.consecutive_failures, 3);
    }

    #[test]
    fn test_success_rate() {
        let mut state = HeartbeatState::new(Duration::from_secs(1));

        state.record_heartbeat(Duration::from_millis(10));
        state.record_heartbeat(Duration::from_millis(10));
        state.record_miss();

        assert_eq!(state.success_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_heartbeat_manager() {
        let mut manager = HeartbeatManager::new(
            Duration::from_secs(1),
            Duration::from_secs(5)
        );

        let node_id = "node1".to_string();
        assert!(manager.register_peer(node_id.clone()).is_ok());
        assert!(manager.record_heartbeat(node_id.clone()).is_ok());

        let state = manager.get_peer_state(&node_id).unwrap();
        assert_eq!(state.status, PeerStatus::Healthy);
    }

    #[test]
    fn test_cluster_health_score() {
        let mut manager = HeartbeatManager::new(
            Duration::from_secs(1),
            Duration::from_secs(5)
        );

        manager.register_peer("node1".to_string()).unwrap();
        manager.register_peer("node2".to_string()).unwrap();

        manager.record_heartbeat("node1".to_string()).unwrap();
        manager.record_heartbeat("node2".to_string()).unwrap();

        let score = manager.cluster_health_score();
        assert!(score > 0.9); // Should be close to 1.0
    }
}
