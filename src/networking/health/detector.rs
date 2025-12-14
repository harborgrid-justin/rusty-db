// # Failure Detector
//
// Implements Phi Accrual failure detection algorithm with adaptive thresholds
// and historical analysis for accurate distributed failure detection.

use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Failure detector trait
pub trait FailureDetector: Send + Sync {
    /// Record a heartbeat from a node
    fn record_heartbeat(&mut self, node_id: NodeId, timestamp: Instant) -> Result<()>;

    /// Get suspicion level for a node (phi value)
    fn get_phi(&self, node_id: &NodeId) -> Result<f64>;

    /// Check if a node is suspected
    fn is_suspected(&self, node_id: &NodeId) -> bool;

    /// Get all suspected nodes
    fn get_suspected_nodes(&self) -> Vec<NodeId>;
}

/// Suspicion level
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SuspicionLevel {
    /// No suspicion
    None,

    /// Low suspicion
    Low,

    /// Medium suspicion
    Medium,

    /// High suspicion (likely failed)
    High,

    /// Critical (definitely failed)
    Critical,
}

impl SuspicionLevel {
    /// Get suspicion level from phi value
    pub fn from_phi(phi: f64) -> Self {
        if phi < 3.0 {
            SuspicionLevel::None
        } else if phi < 5.0 {
            SuspicionLevel::Low
        } else if phi < 8.0 {
            SuspicionLevel::Medium
        } else if phi < 12.0 {
            SuspicionLevel::High
        } else {
            SuspicionLevel::Critical
        }
    }
}

/// Phi Accrual failure detector state for a single node
#[derive(Debug, Clone)]
struct PhiAccrualState {
    /// History of inter-arrival times between heartbeats
    heartbeat_history: VecDeque<Duration>,

    /// Timestamp of last heartbeat
    last_heartbeat: Option<Instant>,

    /// Mean of inter-arrival times
    mean: f64,

    /// Variance of inter-arrival times
    variance: f64,

    /// Maximum window size for history
    max_window_size: usize,

    /// Number of samples received
    sample_count: u64,
}

impl PhiAccrualState {
    fn new(max_window_size: usize) -> Self {
        Self {
            heartbeat_history: VecDeque::with_capacity(max_window_size),
            last_heartbeat: None,
            mean: 0.0,
            variance: 0.0,
            max_window_size,
            sample_count: 0,
        }
    }

    /// Record a new heartbeat
    fn record_heartbeat(&mut self, timestamp: Instant) {
        if let Some(last) = self.last_heartbeat {
            let interval = timestamp.duration_since(last);

            // Add to history
            self.heartbeat_history.push_back(interval);
            if self.heartbeat_history.len() > self.max_window_size {
                self.heartbeat_history.pop_front();
            }

            // Update statistics
            self.update_statistics();
            self.sample_count += 1;
        }

        self.last_heartbeat = Some(timestamp);
    }

    /// Calculate phi value (suspicion level)
    fn calculate_phi(&self, now: Instant) -> f64 {
        let last = match self.last_heartbeat {
            Some(t) => t,
            None => return 0.0,
        };

        // Time since last heartbeat
        let elapsed = now.duration_since(last).as_secs_f64();

        // Need at least a few samples for accurate calculation
        if self.sample_count < 2 {
            return 0.0;
        }

        // Calculate phi using normal distribution
        // phi = -log10(P(t > elapsed))
        let p = self.cumulative_probability(elapsed);

        if p <= 0.0 {
            return 0.0;
        }

        -p.log10()
    }

    /// Calculate cumulative probability using normal distribution
    fn cumulative_probability(&self, elapsed: f64) -> f64 {
        if self.mean == 0.0 || self.variance == 0.0 {
            return 1.0;
        }

        // Z-score
        let std_dev = self.variance.sqrt();
        let z = (elapsed - self.mean) / std_dev;

        // Approximate CDF of standard normal distribution
        // Using error function approximation
        let t = 1.0 / (1.0 + 0.2316419 * z.abs());
        let d = 0.3989423 * (-z * z / 2.0).exp();
        let p = d
            * t
            * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));

        if z > 0.0 {
            1.0 - p
        } else {
            p
        }
    }

    /// Update mean and variance
    fn update_statistics(&mut self) {
        if self.heartbeat_history.is_empty() {
            return;
        }

        // Calculate mean
        let sum: Duration = self.heartbeat_history.iter().copied().sum();
        self.mean = sum.as_secs_f64() / self.heartbeat_history.len() as f64;

        // Calculate variance
        let variance_sum: f64 = self
            .heartbeat_history
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - self.mean;
                diff * diff
            })
            .sum();

        self.variance = variance_sum / self.heartbeat_history.len() as f64;
    }
}

/// Phi Accrual failure detector
pub struct PhiAccrualDetector {
    /// Phi threshold for suspicion
    threshold: f64,

    /// Maximum window size for heartbeat history
    window_size: usize,

    /// Per-node states
    states: HashMap<NodeId, PhiAccrualState>,

    /// Adaptive threshold enabled
    adaptive_threshold: bool,

    /// Minimum threshold
    min_threshold: f64,

    /// Maximum threshold
    max_threshold: f64,
}

impl PhiAccrualDetector {
    /// Create a new Phi Accrual detector
    pub fn new(threshold: f64, window_size: usize) -> Self {
        Self {
            threshold,
            window_size,
            states: HashMap::new(),
            adaptive_threshold: false,
            min_threshold: 3.0,
            max_threshold: 16.0,
        }
    }

    /// Create with adaptive threshold
    pub fn with_adaptive_threshold(threshold: f64, window_size: usize) -> Self {
        Self {
            threshold,
            window_size,
            states: HashMap::new(),
            adaptive_threshold: true,
            min_threshold: 3.0,
            max_threshold: 16.0,
        }
    }

    /// Set threshold
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.clamp(self.min_threshold, self.max_threshold);
    }

    /// Get current threshold for a node
    pub fn get_threshold(&self, node_id: &NodeId) -> f64 {
        if self.adaptive_threshold {
            // Adapt based on network stability
            self.calculate_adaptive_threshold(node_id)
        } else {
            self.threshold
        }
    }

    /// Calculate adaptive threshold based on network conditions
    fn calculate_adaptive_threshold(&self, node_id: &NodeId) -> f64 {
        let state = match self.states.get(node_id) {
            Some(s) => s,
            None => return self.threshold,
        };

        if state.sample_count < 10 {
            return self.threshold;
        }

        // If variance is low (stable network), we can be more sensitive (lower threshold)
        // If variance is high (unstable network), we should be more tolerant (higher threshold)
        let std_dev = state.variance.sqrt();
        let coefficient_of_variation = if state.mean > 0.0 {
            std_dev / state.mean
        } else {
            1.0
        };

        // Adjust threshold based on stability
        let adjustment = coefficient_of_variation * 2.0;
        let adaptive = self.threshold * (1.0 + adjustment);

        adaptive.clamp(self.min_threshold, self.max_threshold)
    }

    /// Get suspicion level for a node
    pub fn get_suspicion_level(&self, node_id: &NodeId) -> Result<SuspicionLevel> {
        let phi = self.get_phi(node_id)?;
        Ok(SuspicionLevel::from_phi(phi))
    }

    /// Remove a node from tracking
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.states.remove(node_id);
    }

    /// Get all tracked nodes
    pub fn get_tracked_nodes(&self) -> Vec<NodeId> {
        self.states.keys().cloned().collect()
    }

    /// Get statistics for a node
    pub fn get_statistics(&self, node_id: &NodeId) -> Option<DetectorStatistics> {
        let state = self.states.get(node_id)?;

        Some(DetectorStatistics {
            mean_interval: Duration::from_secs_f64(state.mean),
            variance: state.variance,
            sample_count: state.sample_count,
            last_heartbeat: state
                .last_heartbeat
                .map(|instant| instant.elapsed().as_millis() as u64),
            current_phi: self.get_phi(node_id).unwrap_or(0.0),
            suspicion_level: SuspicionLevel::from_phi(self.get_phi(node_id).unwrap_or(0.0)),
        })
    }
}

impl FailureDetector for PhiAccrualDetector {
    fn record_heartbeat(&mut self, node_id: NodeId, timestamp: Instant) -> Result<()> {
        let state = self
            .states
            .entry(node_id)
            .or_insert_with(|| PhiAccrualState::new(self.window_size));

        state.record_heartbeat(timestamp);
        Ok(())
    }

    fn get_phi(&self, node_id: &NodeId) -> Result<f64> {
        let state = self
            .states
            .get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        Ok(state.calculate_phi(Instant::now()))
    }

    fn is_suspected(&self, node_id: &NodeId) -> bool {
        let threshold = self.get_threshold(node_id);
        self.get_phi(node_id)
            .map(|phi| phi > threshold)
            .unwrap_or(false)
    }

    fn get_suspected_nodes(&self) -> Vec<NodeId> {
        self.states
            .keys()
            .filter(|node_id| self.is_suspected(node_id))
            .cloned()
            .collect()
    }
}

/// Detector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorStatistics {
    /// Mean interval between heartbeats
    pub mean_interval: Duration,

    /// Variance of intervals
    pub variance: f64,

    /// Number of samples collected
    pub sample_count: u64,

    /// Timestamp of last heartbeat (milliseconds since UNIX_EPOCH)
    pub last_heartbeat: Option<u64>,

    /// Current phi value
    pub current_phi: f64,

    /// Current suspicion level
    pub suspicion_level: SuspicionLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_accrual_state_creation() {
        let state = PhiAccrualState::new(100);
        assert_eq!(state.sample_count, 0);
        assert_eq!(state.mean, 0.0);
        assert_eq!(state.variance, 0.0);
    }

    #[test]
    fn test_record_heartbeat() {
        let mut state = PhiAccrualState::new(100);
        let now = Instant::now();

        state.record_heartbeat(now);
        assert!(state.last_heartbeat.is_some());
        assert_eq!(state.sample_count, 0); // First heartbeat doesn't create an interval

        std::thread::sleep(Duration::from_millis(10));
        let later = Instant::now();
        state.record_heartbeat(later);
        assert_eq!(state.sample_count, 1);
    }

    #[test]
    fn test_phi_calculation() {
        let mut detector = PhiAccrualDetector::new(8.0, 100);
        let node_id = "node1".to_string();
        let now = Instant::now();

        // Record several heartbeats at regular intervals
        for i in 0..10 {
            let timestamp = now + Duration::from_millis(i * 100);
            detector
                .record_heartbeat(node_id.clone(), timestamp)
                .unwrap();
        }

        // Immediately after, phi should be low
        let phi = detector.get_phi(&node_id).unwrap();
        assert!(phi < 1.0, "Phi should be low immediately after heartbeats");
    }

    #[test]
    fn test_suspicion_level() {
        assert_eq!(SuspicionLevel::from_phi(2.0), SuspicionLevel::None);
        assert_eq!(SuspicionLevel::from_phi(4.0), SuspicionLevel::Low);
        assert_eq!(SuspicionLevel::from_phi(6.0), SuspicionLevel::Medium);
        assert_eq!(SuspicionLevel::from_phi(10.0), SuspicionLevel::High);
        assert_eq!(SuspicionLevel::from_phi(15.0), SuspicionLevel::Critical);
    }

    #[test]
    fn test_detector_interface() {
        let mut detector = PhiAccrualDetector::new(8.0, 100);
        let node_id = "node1".to_string();

        assert!(detector
            .record_heartbeat(node_id.clone(), Instant::now())
            .is_ok());
        assert!(!detector.is_suspected(&node_id));
        assert_eq!(detector.get_suspected_nodes().len(), 0);
    }
}
