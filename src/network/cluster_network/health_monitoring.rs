// Health Monitoring Module
//
// Network health monitoring, latency tracking, and route optimization

use std::time::{Duration, SystemTime};

use super::NodeId;

// ============================================================================
// Network Health Monitor
// ============================================================================

pub struct NetworkHealthMonitor {
    check_interval: Duration,
    checks_performed: u64,
    unhealthy_nodes: Vec<NodeId>,
}

impl NetworkHealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            check_interval,
            checks_performed: 0,
            unhealthy_nodes: Vec::new(),
        }
    }

    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }

    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }

    pub fn record_check(&mut self) {
        self.checks_performed += 1;
    }

    pub fn mark_unhealthy(&mut self, node_id: NodeId) {
        if !self.unhealthy_nodes.contains(&node_id) {
            self.unhealthy_nodes.push(node_id);
        }
    }

    pub fn mark_healthy(&mut self, node_id: NodeId) {
        if let Some(pos) = self.unhealthy_nodes.iter().position(|&id| id == node_id) {
            self.unhealthy_nodes.remove(pos);
        }
    }

    pub fn unhealthy_nodes(&self) -> &[NodeId] {
        &self.unhealthy_nodes
    }

    pub fn is_healthy(&self, node_id: NodeId) -> bool {
        !self.unhealthy_nodes.contains(&node_id)
    }

    pub fn checks_performed(&self) -> u64 {
        self.checks_performed
    }
}

impl Default for NetworkHealthMonitor {
    fn default() -> Self {
        Self::new(Duration::from_secs(10))
    }
}

// ============================================================================
// Health Metrics
// ============================================================================

#[derive(Debug, Clone)]
pub struct HealthMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub node_id: NodeId,
    pub healthy: bool,
    pub message: String,
    pub checked_at: SystemTime,
}

impl HealthCheckResult {
    pub fn healthy(node_id: NodeId) -> Self {
        Self {
            node_id,
            healthy: true,
            message: "Node is healthy".to_string(),
            checked_at: SystemTime::now(),
        }
    }

    pub fn unhealthy(node_id: NodeId, message: String) -> Self {
        Self {
            node_id,
            healthy: false,
            message,
            checked_at: SystemTime::now(),
        }
    }
}

// ============================================================================
// Latency Tracker
// ============================================================================

pub struct LatencyTracker {
    samples: Vec<Duration>,
    max_samples: usize,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    pub fn with_capacity(max_samples: usize) -> Self {
        Self {
            samples: Vec::new(),
            max_samples,
        }
    }

    pub fn record_latency(&mut self, latency: Duration) {
        self.samples.push(latency);

        // Keep only the most recent samples
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }

    pub fn average_latency(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }

        let sum: Duration = self.samples.iter().sum();
        Some(sum / self.samples.len() as u32)
    }

    pub fn min_latency(&self) -> Option<Duration> {
        self.samples.iter().min().copied()
    }

    pub fn max_latency(&self) -> Option<Duration> {
        self.samples.iter().max().copied()
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Bandwidth Monitor
// ============================================================================

pub struct BandwidthMonitor {
    window_size: Duration,
    bytes_sent: u64,
    bytes_received: u64,
    window_start: SystemTime,
}

impl BandwidthMonitor {
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            bytes_sent: 0,
            bytes_received: 0,
            window_start: SystemTime::now(),
        }
    }

    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    pub fn reset_window(&mut self) {
        self.bytes_sent = 0;
        self.bytes_received = 0;
        self.window_start = SystemTime::now();
    }

    pub fn window_size(&self) -> Duration {
        self.window_size
    }
}

impl Default for BandwidthMonitor {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

// ============================================================================
// Packet Loss Detector
// ============================================================================

pub struct PacketLossDetector {
    threshold: f64,
    packets_sent: u64,
    packets_lost: u64,
}

impl PacketLossDetector {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            packets_sent: 0,
            packets_lost: 0,
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn record_sent(&mut self) {
        self.packets_sent += 1;
    }

    pub fn record_lost(&mut self) {
        self.packets_lost += 1;
    }

    pub fn loss_rate(&self) -> f64 {
        if self.packets_sent > 0 {
            self.packets_lost as f64 / self.packets_sent as f64
        } else {
            0.0
        }
    }

    pub fn is_above_threshold(&self) -> bool {
        self.loss_rate() > self.threshold
    }

    pub fn packets_sent(&self) -> u64 {
        self.packets_sent
    }

    pub fn packets_lost(&self) -> u64 {
        self.packets_lost
    }
}

// ============================================================================
// Network Quality Scorer
// ============================================================================

pub struct NetworkQualityScorer {
    scores: std::collections::HashMap<NodeId, f64>,
}

impl NetworkQualityScorer {
    pub fn new() -> Self {
        Self {
            scores: std::collections::HashMap::new(),
        }
    }

    pub fn update_score(&mut self, node_id: NodeId, score: f64) {
        self.scores.insert(node_id, score);
    }

    pub fn get_score(&self, node_id: NodeId) -> Option<f64> {
        self.scores.get(&node_id).copied()
    }

    pub fn best_node(&self) -> Option<NodeId> {
        self.scores
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&node_id, _)| node_id)
    }

    pub fn worst_node(&self) -> Option<NodeId> {
        self.scores
            .iter()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&node_id, _)| node_id)
    }
}

impl Default for NetworkQualityScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Route Optimizer
// ============================================================================

pub struct RouteOptimizer {
    optimizations_performed: u64,
}

impl RouteOptimizer {
    pub fn new() -> Self {
        Self {
            optimizations_performed: 0,
        }
    }

    pub fn record_optimization(&mut self) {
        self.optimizations_performed += 1;
    }

    pub fn optimizations_performed(&self) -> u64 {
        self.optimizations_performed
    }
}

impl Default for RouteOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RouteOptimization {
    pub from: NodeId,
    pub to: NodeId,
    pub via: Vec<NodeId>,
}

impl RouteOptimization {
    pub fn direct(from: NodeId, to: NodeId) -> Self {
        Self {
            from,
            to,
            via: Vec::new(),
        }
    }

    pub fn with_hops(from: NodeId, to: NodeId, via: Vec<NodeId>) -> Self {
        Self { from, to, via }
    }
}

// ============================================================================
// Node Network Metrics
// ============================================================================

#[derive(Debug, Clone)]
pub struct NodeNetworkMetrics {
    pub node_id: NodeId,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    pub packet_loss: f64,
}

impl NodeNetworkMetrics {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            latency_ms: 0.0,
            bandwidth_mbps: 0.0,
            packet_loss: 0.0,
        }
    }

    pub fn quality_score(&self) -> f64 {
        // Simple scoring: lower latency and packet loss are better, higher bandwidth is better
        let latency_score = 100.0 / (1.0 + self.latency_ms);
        let bandwidth_score = self.bandwidth_mbps / 10.0; // Normalize to 0-100 range
        let loss_score = 100.0 * (1.0 - self.packet_loss);

        (latency_score + bandwidth_score + loss_score) / 3.0
    }
}
