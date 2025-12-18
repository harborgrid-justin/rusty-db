// Load Balancing Module
//
// Cluster load balancing, routing strategies, and hotspot detection

use std::collections::HashMap;

use super::NodeId;

// ============================================================================
// Routing Strategies
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    ConsistentHash,
    Adaptive,
}

// ============================================================================
// Cluster Load Balancer
// ============================================================================

pub struct ClusterLoadBalancer {
    strategy: RoutingStrategy,
    nodes: Vec<NodeId>,
    current_index: usize,
    total_requests: u64,
    requests_per_node: HashMap<NodeId, u64>,
}

impl ClusterLoadBalancer {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            strategy,
            nodes: Vec::new(),
            current_index: 0,
            total_requests: 0,
            requests_per_node: HashMap::new(),
        }
    }

    pub fn strategy(&self) -> RoutingStrategy {
        self.strategy
    }

    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }

    pub fn add_node(&mut self, node_id: NodeId) {
        if !self.nodes.contains(&node_id) {
            self.nodes.push(node_id);
            self.requests_per_node.insert(node_id, 0);
        }
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        if let Some(pos) = self.nodes.iter().position(|&id| id == node_id) {
            self.nodes.remove(pos);
            self.requests_per_node.remove(&node_id);

            // Adjust current_index if needed
            if self.current_index >= self.nodes.len() && !self.nodes.is_empty() {
                self.current_index = 0;
            }
        }
    }

    pub fn select_node(&mut self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            return None;
        }

        let selected = match self.strategy {
            RoutingStrategy::RoundRobin => {
                let node_id = self.nodes[self.current_index];
                self.current_index = (self.current_index + 1) % self.nodes.len();
                Some(node_id)
            }
            RoutingStrategy::LeastConnections => {
                // Find node with least requests
                self.nodes
                    .iter()
                    .min_by_key(|&&node_id| {
                        self.requests_per_node.get(&node_id).copied().unwrap_or(0)
                    })
                    .copied()
            }
            RoutingStrategy::WeightedRandom => {
                // Simple random selection (could be enhanced with weights)
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hash, Hasher};

                let mut hasher = RandomState::new().build_hasher();
                self.total_requests.hash(&mut hasher);
                let index = (hasher.finish() as usize) % self.nodes.len();
                Some(self.nodes[index])
            }
            RoutingStrategy::ConsistentHash => {
                // Simplified consistent hashing
                Some(self.nodes[self.total_requests as usize % self.nodes.len()])
            }
            RoutingStrategy::Adaptive => {
                // Use least connections for adaptive
                self.nodes
                    .iter()
                    .min_by_key(|&&node_id| {
                        self.requests_per_node.get(&node_id).copied().unwrap_or(0)
                    })
                    .copied()
            }
        };

        if let Some(node_id) = selected {
            self.total_requests += 1;
            *self.requests_per_node.entry(node_id).or_insert(0) += 1;
        }

        selected
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn metrics(&self) -> LoadBalancerMetrics {
        LoadBalancerMetrics {
            total_requests: self.total_requests,
            requests_per_node: self.requests_per_node.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadBalancerMetrics {
    pub total_requests: u64,
    pub requests_per_node: HashMap<NodeId, u64>,
}

// ============================================================================
// Locality Awareness
// ============================================================================

pub struct LocalityMap {
    zones: HashMap<String, Vec<NodeId>>,
}

impl LocalityMap {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
        }
    }

    pub fn add_node_to_zone(&mut self, zone: String, node_id: NodeId) {
        self.zones
            .entry(zone)
            .or_insert_with(Vec::new)
            .push(node_id);
    }

    pub fn get_nodes_in_zone(&self, zone: &str) -> Option<&Vec<NodeId>> {
        self.zones.get(zone)
    }

    pub fn remove_node_from_zone(&mut self, zone: &str, node_id: NodeId) -> bool {
        if let Some(nodes) = self.zones.get_mut(zone) {
            if let Some(pos) = nodes.iter().position(|&id| id == node_id) {
                nodes.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn zones(&self) -> Vec<&String> {
        self.zones.keys().collect()
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }
}

impl Default for LocalityMap {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hotspot Detection
// ============================================================================

pub struct HotspotDetector {
    threshold: f64,
    node_loads: HashMap<NodeId, f64>,
}

impl HotspotDetector {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            node_loads: HashMap::new(),
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold;
    }

    pub fn update_node_load(&mut self, node_id: NodeId, load: f64) {
        self.node_loads.insert(node_id, load);
    }

    pub fn detect_hotspots(&self) -> Vec<NodeId> {
        self.node_loads
            .iter()
            .filter(|(_, &load)| load > self.threshold)
            .map(|(&node_id, _)| node_id)
            .collect()
    }

    pub fn is_hotspot(&self, node_id: NodeId) -> bool {
        self.node_loads
            .get(&node_id)
            .map(|&load| load > self.threshold)
            .unwrap_or(false)
    }

    pub fn node_load(&self, node_id: NodeId) -> Option<f64> {
        self.node_loads.get(&node_id).copied()
    }
}

// ============================================================================
// Connection Affinity
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConnectionAffinity {
    pub client_id: String,
    pub preferred_node: NodeId,
}

impl ConnectionAffinity {
    pub fn new(client_id: String, preferred_node: NodeId) -> Self {
        Self {
            client_id,
            preferred_node,
        }
    }
}
