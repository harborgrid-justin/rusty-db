// # Health Aggregator
//
// Aggregates multiple health check results into overall health scores
// with weighted scoring, dependency health tracking, and cascading failure detection.

#![allow(dead_code)]

use super::checker::HealthCheckResult;
use crate::common::{HealthStatus, NodeId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Health score (0.0 = unhealthy, 1.0 = perfectly healthy)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HealthScore {
    /// Overall health score
    pub score: f64,

    /// Availability score
    pub availability: f64,

    /// Performance score (based on response times)
    pub performance: f64,

    /// Reliability score (based on success rate)
    pub reliability: f64,
}

impl HealthScore {
    /// Create a perfect health score
    pub fn perfect() -> Self {
        Self {
            score: 1.0,
            availability: 1.0,
            performance: 1.0,
            reliability: 1.0,
        }
    }

    /// Create a failed health score
    pub fn failed() -> Self {
        Self {
            score: 0.0,
            availability: 0.0,
            performance: 0.0,
            reliability: 0.0,
        }
    }

    /// Get health status from score
    pub fn to_health_status(&self) -> HealthStatus {
        if self.score >= 0.9 {
            HealthStatus::Healthy
        } else if self.score >= 0.5 {
            HealthStatus::Degraded
        } else if self.score > 0.0 {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Unknown
        }
    }
}

/// Weighted health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedCheck {
    /// Check identifier
    pub check_id: String,

    /// Weight (0.0 to 1.0)
    pub weight: f64,

    /// Is this check critical (failure = complete failure)
    pub critical: bool,
}

/// Node health state
#[derive(Debug, Clone)]
struct NodeHealthState {
    /// Check results history
    check_results: HashMap<String, Vec<HealthCheckResult>>,

    /// Weighted check configurations
    weighted_checks: HashMap<String, WeightedCheck>,

    /// Current health score
    current_score: HealthScore,

    /// Last update time
    last_update: Instant,

    /// Maximum history size per check
    max_history_size: usize,

    /// Dependencies (other nodes this node depends on)
    dependencies: Vec<NodeId>,
}

impl NodeHealthState {
    fn new(max_history_size: usize) -> Self {
        Self {
            check_results: HashMap::new(),
            weighted_checks: HashMap::new(),
            current_score: HealthScore::perfect(),
            last_update: Instant::now(),
            max_history_size,
            dependencies: Vec::new(),
        }
    }

    /// Record a health check result
    fn record_result(&mut self, result: HealthCheckResult) {
        let check_type = result.check_type.clone();

        let results = self
            .check_results
            .entry(check_type)
            .or_insert_with(Vec::new);
        results.push(result);

        // Trim history
        if results.len() > self.max_history_size {
            results.remove(0);
        }

        self.last_update = Instant::now();
    }

    /// Calculate current health score
    fn calculate_score(&self) -> HealthScore {
        if self.check_results.is_empty() {
            return HealthScore::perfect();
        }

        let mut total_weight = 0.0;
        let mut weighted_availability = 0.0;
        let mut weighted_performance = 0.0;
        let mut weighted_reliability = 0.0;

        for (check_type, results) in &self.check_results {
            if results.is_empty() {
                continue;
            }

            // Get weight for this check
            let weight = self
                .weighted_checks
                .get(check_type)
                .map(|w| w.weight)
                .unwrap_or(1.0);

            // Check if critical
            let is_critical = self
                .weighted_checks
                .get(check_type)
                .map(|w| w.critical)
                .unwrap_or(false);

            // Get latest result
            let latest = &results[results.len() - 1];

            // Calculate availability (success rate)
            let success_count = results.iter().filter(|r| r.success).count();
            let availability = success_count as f64 / results.len() as f64;

            // Calculate performance (based on response times)
            let avg_response_time = if !results.is_empty() {
                let sum: Duration = results
                    .iter()
                    .filter(|r| r.success)
                    .map(|r| r.response_time)
                    .sum();
                sum.as_secs_f64() / results.len() as f64
            } else {
                0.0
            };

            // Performance score (better for lower response times)
            // Assume 100ms is perfect, 1s is acceptable, >5s is poor
            let performance = if avg_response_time < 0.1 {
                1.0
            } else if avg_response_time < 1.0 {
                1.0 - (avg_response_time - 0.1) / 0.9 * 0.3
            } else if avg_response_time < 5.0 {
                0.7 - (avg_response_time - 1.0) / 4.0 * 0.6
            } else {
                0.1
            };

            // Reliability (consistency over time)
            let reliability = self.calculate_reliability(results);

            // If check is critical and failed, score is 0
            if is_critical && !latest.success {
                return HealthScore::failed();
            }

            weighted_availability += availability * weight;
            weighted_performance += performance * weight;
            weighted_reliability += reliability * weight;
            total_weight += weight;
        }

        if total_weight == 0.0 {
            return HealthScore::perfect();
        }

        let availability = weighted_availability / total_weight;
        let performance = weighted_performance / total_weight;
        let reliability = weighted_reliability / total_weight;

        // Overall score is a weighted combination
        let score = (availability * 0.5) + (performance * 0.25) + (reliability * 0.25);

        HealthScore {
            score,
            availability,
            performance,
            reliability,
        }
    }

    /// Calculate reliability score from results
    fn calculate_reliability(&self, results: &[HealthCheckResult]) -> f64 {
        if results.len() < 2 {
            return 1.0;
        }

        // Calculate variance in success/failure pattern
        let mut transitions = 0;
        for i in 1..results.len() {
            if results[i].success != results[i - 1].success {
                transitions += 1;
            }
        }

        // Fewer transitions = more reliable
        let transition_rate = transitions as f64 / (results.len() - 1) as f64;
        1.0 - transition_rate
    }
}

/// Health aggregator
pub struct HealthAggregator {
    /// Per-node health states
    nodes: HashMap<NodeId, NodeHealthState>,

    /// Maximum history size
    max_history_size: usize,

    /// Enable dependency tracking
    track_dependencies: bool,
}

impl HealthAggregator {
    /// Create a new health aggregator
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            max_history_size: 100,
            track_dependencies: true,
        }
    }

    /// Create with custom history size
    pub fn with_history_size(max_history_size: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            max_history_size,
            track_dependencies: true,
        }
    }

    /// Record a health check result for a node
    pub async fn record_check_result(
        &mut self,
        node_id: NodeId,
        result: HealthCheckResult,
    ) -> Result<()> {
        let state = self
            .nodes
            .entry(node_id.clone())
            .or_insert_with(|| NodeHealthState::new(self.max_history_size));

        state.record_result(result);

        // Recalculate score
        state.current_score = state.calculate_score();

        Ok(())
    }

    /// Add a weighted check configuration
    pub async fn add_weighted_check(
        &mut self,
        node_id: NodeId,
        weighted_check: WeightedCheck,
    ) -> Result<()> {
        let state = self
            .nodes
            .entry(node_id)
            .or_insert_with(|| NodeHealthState::new(self.max_history_size));

        state
            .weighted_checks
            .insert(weighted_check.check_id.clone(), weighted_check);
        Ok(())
    }

    /// Add a dependency between nodes
    pub async fn add_dependency(&mut self, node_id: NodeId, depends_on: NodeId) -> Result<()> {
        let state = self
            .nodes
            .entry(node_id)
            .or_insert_with(|| NodeHealthState::new(self.max_history_size));

        if !state.dependencies.contains(&depends_on) {
            state.dependencies.push(depends_on);
        }

        Ok(())
    }

    /// Get health score for a node
    pub async fn get_health_score(&self, node_id: &NodeId) -> Result<HealthScore> {
        let state = self
            .nodes
            .get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        let mut score = state.current_score;

        // Factor in dependency health
        if self.track_dependencies {
            score = self.adjust_for_dependencies(node_id, score).await?;
        }

        Ok(score)
    }

    /// Get health status for a node
    pub async fn get_node_status(&self, node_id: &NodeId) -> Result<HealthStatus> {
        let score = self.get_health_score(node_id).await?;
        Ok(score.to_health_status())
    }

    /// Get all unhealthy nodes
    pub async fn get_unhealthy_nodes(&self) -> Vec<NodeId> {
        let mut unhealthy = Vec::new();

        for (node_id, state) in &self.nodes {
            if state.current_score.score < 0.5 {
                unhealthy.push(node_id.clone());
            }
        }

        unhealthy
    }

    /// Get cluster-wide health score
    pub async fn get_cluster_health_score(&self) -> HealthScore {
        if self.nodes.is_empty() {
            return HealthScore::perfect();
        }

        let total: HealthScore = self.nodes.values().map(|state| state.current_score).fold(
            HealthScore {
                score: 0.0,
                availability: 0.0,
                performance: 0.0,
                reliability: 0.0,
            },
            |acc, score| HealthScore {
                score: acc.score + score.score,
                availability: acc.availability + score.availability,
                performance: acc.performance + score.performance,
                reliability: acc.reliability + score.reliability,
            },
        );

        let count = self.nodes.len() as f64;
        HealthScore {
            score: total.score / count,
            availability: total.availability / count,
            performance: total.performance / count,
            reliability: total.reliability / count,
        }
    }

    /// Detect cascading failures
    pub async fn detect_cascading_failures(&self) -> Vec<Vec<NodeId>> {
        let mut cascades = Vec::new();

        // Find nodes that are failed
        let failed_nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, state)| state.current_score.score < 0.3)
            .map(|(id, _)| id.clone())
            .collect();

        // Find dependent nodes
        for failed_node in &failed_nodes {
            let mut cascade = vec![failed_node.clone()];

            for (node_id, state) in &self.nodes {
                if state.dependencies.contains(failed_node) {
                    cascade.push(node_id.clone());
                }
            }

            if cascade.len() > 1 {
                cascades.push(cascade);
            }
        }

        cascades
    }

    /// Adjust score based on dependencies
    async fn adjust_for_dependencies(
        &self,
        node_id: &NodeId,
        mut score: HealthScore,
    ) -> Result<HealthScore> {
        let state = self
            .nodes
            .get(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;

        if state.dependencies.is_empty() {
            return Ok(score);
        }

        // Calculate dependency health
        let mut dep_health_sum = 0.0;
        let mut dep_count = 0;

        for dep_id in &state.dependencies {
            if let Some(dep_state) = self.nodes.get(dep_id) {
                dep_health_sum += dep_state.current_score.score;
                dep_count += 1;
            }
        }

        if dep_count > 0 {
            let dep_health = dep_health_sum / dep_count as f64;

            // If dependencies are unhealthy, reduce this node's score
            let dependency_factor = dep_health * 0.3 + 0.7; // 70-100% based on deps
            score.score *= dependency_factor;
        }

        Ok(score)
    }

    /// Remove a node from tracking
    pub async fn remove_node(&mut self, node_id: &NodeId) -> Result<()> {
        self.nodes
            .remove(node_id)
            .ok_or_else(|| DbError::NotFound(format!("Node {} not found", node_id)))?;
        Ok(())
    }
}

impl Default for HealthAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_health_score() {
        let perfect = HealthScore::perfect();
        assert_eq!(perfect.score, 1.0);
        assert_eq!(perfect.to_health_status(), HealthStatus::Healthy);

        let failed = HealthScore::failed();
        assert_eq!(failed.score, 0.0);
    }

    #[tokio::test]
    async fn test_aggregator_record_result() {
        let mut agg = HealthAggregator::new();
        let node_id = "node1".to_string();

        let result = HealthCheckResult::success("test".to_string(), Duration::from_millis(10));

        assert!(agg
            .record_check_result(node_id.clone(), result)
            .await
            .is_ok());
        assert!(agg.get_health_score(&node_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_weighted_check() {
        let mut agg = HealthAggregator::new();
        let node_id = "node1".to_string();

        let weighted = WeightedCheck {
            check_id: "test".to_string(),
            weight: 2.0,
            critical: false,
        };

        assert!(agg.add_weighted_check(node_id, weighted).await.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_health_score() {
        let mut agg = HealthAggregator::new();

        // Add results for multiple nodes
        for i in 0..3 {
            let node_id = format!("node{}", i);
            let result = HealthCheckResult::success("test".to_string(), Duration::from_millis(10));
            agg.record_check_result(node_id, result).await.unwrap();
        }

        let cluster_score = agg.get_cluster_health_score().await;
        assert!(cluster_score.score > 0.9);
    }
}
