// # Dependency Graph Resolution
//
// This module provides dependency graph resolution with cycle detection
// for managing complex service dependencies in RustyDB.
//
// ## Features
//
// - **Dependency Tracking**: Build and maintain dependency graphs
// - **Cycle Detection**: Detect and prevent circular dependencies
// - **Topological Sorting**: Determine correct initialization order
// - **Dependency Validation**: Verify all dependencies can be satisfied
// - **Impact Analysis**: Determine which services are affected by changes
// - **Visualization**: Generate dependency graph representations
//
// ## Architecture
//
// ```text
// ┌────────────────────────────────────────────────┐
// │        Dependency Graph                        │
// ├────────────────────────────────────────────────┤
// │                                                │
// │  Service A → [Service B, Service C]           │
// │  Service B → [Service D]                      │
// │  Service C → [Service D]                      │
// │  Service D → []                               │
// │                                                │
// │  Initialization Order: D, B, C, A             │
// │                                                │
// └────────────────────────────────────────────────┘
// ```

use std::collections::VecDeque;
use std::fmt;
use std::collections::HashSet;
use std::collections::{HashMap};


use tracing::{debug, info};

use crate::error::{Result, DbError};

// Node in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyNode {
    // Node identifier (service name)
    pub id: String,
    // Display name
    pub name: String,
    // Node metadata
    pub metadata: HashMap<String, String>,
}

impl DependencyNode {
    // Create a new dependency node
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            metadata: HashMap::new(),
        }
    }

    // Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

// Edge in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    // Source node (dependent)
    pub from: String,
    // Target node (dependency)
    pub to: String,
    // Edge type
    pub edge_type: DependencyType,
    // Whether this is a required dependency
    pub required: bool,
}

impl DependencyEdge {
    // Create a new dependency edge
    pub fn new(from: String, to: String, edge_type: DependencyType) -> Self {
        Self {
            from,
            to,
            edge_type,
            required: true,
        }
    }

    // Mark as optional dependency
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

// Type of dependency relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    // Hard dependency (must be initialized first)
    Hard,
    // Soft dependency (preferred order but not required)
    Soft,
    // Runtime dependency (can be resolved later)
    Runtime,
}

// Dependency graph for tracking service dependencies
pub struct DependencyGraph {
    // Nodes in the graph
    nodes: HashMap<String, DependencyNode>,
    // Adjacency list (node -> dependencies)
    edges: HashMap<String, Vec<DependencyEdge>>,
    // Reverse adjacency list (node -> dependents)
    reverse_edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    // Create a new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    // Add a node to the graph
    pub fn add_node(&mut self, node: DependencyNode) -> Result<()> {
        let id = node.id.clone();
        if self.nodes.contains_key(&id) {
            return Err(DbError::Internal(format!("Node already exists: {}", id)));
        }

        self.nodes.insert(id.clone(), node);
        self.edges.insert(id.clone(), Vec::new());
        self.reverse_edges.insert(id, Vec::new());

        Ok(())
    }

    // Add an edge to the graph
    pub fn add_edge(&mut self, edge: DependencyEdge) -> Result<()> {
        // Verify both nodes exist
        if !self.nodes.contains_key(&edge.from) {
            return Err(DbError::Internal(format!("Node not found: {}", edge.from)));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(DbError::Internal(format!("Node not found: {}", edge.to)));
        }

        // Add to adjacency list
        self.edges
            .entry(edge.from.clone())
            .or_insert_with(Vec::new)
            .push(edge.clone());

        // Add to reverse adjacency list
        self.reverse_edges
            .entry(edge.to.clone())
            .or_insert_with(Vec::new)
            .push(edge.from.clone());

        debug!("Added dependency: {} -> {}", edge.from, edge.to);

        Ok(())
    }

    // Remove a node and all its edges
    pub fn remove_node(&mut self, id: &str) -> Result<()> {
        if !self.nodes.contains_key(id) {
            return Err(DbError::Internal(format!("Node not found: {}", id)));
        }

        // Remove node
        self.nodes.remove(id);

        // Remove outgoing edges
        self.edges.remove(id);

        // Remove incoming edges
        self.reverse_edges.remove(id);

        // Remove references from other nodes
        for edges in self.edges.values_mut() {
            edges.retain(|e| e.to != id);
        }

        for dependents in self.reverse_edges.values_mut() {
            dependents.retain(|d| d != id);
        }

        debug!("Removed node: {}", id);

        Ok(())
    }

    // Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&DependencyNode> {
        self.nodes.get(id)
    }

    // Get dependencies of a node
    pub fn get_dependencies(&self, id: &str) -> Vec<&DependencyEdge> {
        self.edges
            .get(id)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    // Get dependents of a node (nodes that depend on this one)
    pub fn get_dependents(&self, id: &str) -> Vec<&String> {
        self.reverse_edges
            .get(id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    // Check if the graph has cycles
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if self.has_cycle_util(node_id, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    // Utility function for cycle detection using DFS
    fn has_cycle_util(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(edges) = self.edges.get(node) {
            for edge in edges {
                // Only check hard dependencies for cycles
                if edge.edge_type != DependencyType::Hard {
                    continue;
                }

                if !visited.contains(&edge.to) {
                    if self.has_cycle_util(&edge.to, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&edge.to) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    // Find a cycle in the graph (if one exists)
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if let Some(cycle) = self.find_cycle_util(
                    node_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    // Utility function for finding a cycle
    fn find_cycle_util(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(edges) = self.edges.get(node) {
            for edge in edges {
                if edge.edge_type != DependencyType::Hard {
                    continue;
                }

                if !visited.contains(&edge.to) {
                    if let Some(cycle) =
                        self.find_cycle_util(&edge.to, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&edge.to) {
                    // Found a cycle - extract it from the path
                    if let Some(start_idx) = path.iter().position(|n| n == &edge.to) {
                        let mut cycle = path[start_idx..].to_vec();
                        cycle.push(edge.to.clone());
                        return Some(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    // Perform topological sort to get initialization order
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // Check for cycles first
        if let Some(cycle) = self.find_cycle() {
            return Err(DbError::Internal(format!(
                "Circular dependency detected: {}",
                cycle.join(" -> ")
            )));
        }

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Initialize in-degree for all nodes
        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
        }

        // Calculate in-degree (only for hard dependencies)
        for edges in self.edges.values() {
            for edge in edges {
                if edge.edge_type == DependencyType::Hard {
                    *in_degree.entry(edge.from.clone()).or_insert(0) += 0;
                    *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
                }
            }
        }

        // Add nodes with no dependencies to queue
        for (node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id.clone());
            }
        }

        // Process nodes
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(edges) = self.edges.get(&node) {
                for edge in edges {
                    if edge.edge_type == DependencyType::Hard {
                        let to = &edge.to;
                        if let Some(degree) = in_degree.get_mut(to) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(to.clone());
                            }
                        }
                    }
                }
            }
        }

        // Verify all nodes were processed
        if result.len() != self.nodes.len() {
            return Err(DbError::Internal(
                "Failed to compute topological sort - possible cycle".into(),
            ));
        }

        // Reverse to get dependency-first order
        result.reverse();

        info!("Computed initialization order: {}", result.join(" -> "));

        Ok(result)
    }

    // Get all nodes that depend on the given node (transitive closure)
    pub fn get_impact_set(&self, node_id: &str) -> HashSet<String> {
        let mut impact = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(node_id.to_string());

        while let Some(node) = queue.pop_front() {
            if let Some(dependents) = self.reverse_edges.get(&node) {
                for dependent in dependents {
                    if impact.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        impact
    }

    // Validate that all dependencies can be satisfied
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for (node_id, edges) in &self.edges {
            for edge in edges {
                if edge.required && !self.nodes.contains_key(&edge.to) {
                    errors.push(format!(
                        "Node '{}' depends on missing node '{}'",
                        node_id, edge.to
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(DbError::Internal(format!(
                "Dependency validation failed: {}",
                errors.join(", ")
            )))
        }
    }

    // Get graph statistics
    pub fn statistics(&self) -> GraphStatistics {
        let total_edges: usize = self.edges.values().map(|e| e.len()).sum();
        let hard_edges = self
            .edges
            .values()
            .flatten()
            .filter(|e| e.edge_type == DependencyType::Hard)
            .count();
        let soft_edges = self
            .edges
            .values()
            .flatten()
            .filter(|e| e.edge_type == DependencyType::Soft)
            .count();

        GraphStatistics {
            total_nodes: self.nodes.len(),
            total_edges,
            hard_edges,
            soft_edges,
            has_cycles: self.has_cycles(),
        }
    }

    // Generate a DOT representation of the graph for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph Dependencies {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Add nodes
        for node in self.nodes.values() {
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", node.id, node.name));
        }

        dot.push('\n');

        // Add edges
        for edges in self.edges.values() {
            for edge in edges {
                let style = match edge.edge_type {
                    DependencyType::Hard => "solid",
                    DependencyType::Soft => "dashed",
                    DependencyType::Runtime => "dotted",
                };
                let color = if edge.required { "black" } else { "gray" };

                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\" [style={}, color={}];\n",
                    edge.from, edge.to, style, color
                ));
            }
        }

        dot.push_str("}\n");
        dot
    }

    // Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|e| e.len()).sum()
    }

    // Clear the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.reverse_edges.clear();
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DependencyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependencyGraph")
            .field("nodes", &self.nodes.len())
            .field("edges", &self.edge_count())
            .finish()
    }
}

// Statistics about a dependency graph
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    // Total number of nodes
    pub total_nodes: usize,
    // Total number of edges
    pub total_edges: usize,
    // Number of hard dependency edges
    pub hard_edges: usize,
    // Number of soft dependency edges
    pub soft_edges: usize,
    // Whether the graph has cycles
    pub has_cycles: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();
        let node = DependencyNode::new("service1".into(), "Service 1".into());

        assert!(graph.add_node(node).is_ok());
        assert!(graph.get_node("service1").is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node(DependencyNode::new("a".into(), "A".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("b".into(), "B".into()))
            .unwrap();

        let edge = DependencyEdge::new("a".into(), "b".into(), DependencyType::Hard);
        assert!(graph.add_edge(edge).is_ok());

        let deps = graph.get_dependencies("a");
        assert_eq!(deps.len(), 1);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node(DependencyNode::new("a".into(), "A".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("b".into(), "B".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("c".into(), "C".into()))
            .unwrap();

        // Create a cycle: a -> b -> c -> a
        graph
            .add_edge(DependencyEdge::new("a".into(), "b".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("b".into(), "c".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("c".into(), "a".into(), DependencyType::Hard))
            .unwrap();

        assert!(graph.has_cycles());

        let cycle = graph.find_cycle();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();

        // Create a DAG: d -> b -> a, d -> c -> a
        graph
            .add_node(DependencyNode::new("a".into(), "A".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("b".into(), "B".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("c".into(), "C".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("d".into(), "D".into()))
            .unwrap();

        graph
            .add_edge(DependencyEdge::new("a".into(), "b".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("a".into(), "c".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("b".into(), "d".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("c".into(), "d".into(), DependencyType::Hard))
            .unwrap();

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 4);

        // D should come before B and C
        let d_pos = order.iter().position(|n| n == "d").unwrap();
        let b_pos = order.iter().position(|n| n == "b").unwrap();
        let c_pos = order.iter().position(|n| n == "c").unwrap();
        assert!(d_pos < b_pos);
        assert!(d_pos < c_pos);
    }

    #[test]
    fn test_impact_set() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node(DependencyNode::new("a".into(), "A".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("b".into(), "B".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("c".into(), "C".into()))
            .unwrap();

        graph
            .add_edge(DependencyEdge::new("a".into(), "c".into(), DependencyType::Hard))
            .unwrap();
        graph
            .add_edge(DependencyEdge::new("b".into(), "c".into(), DependencyType::Hard))
            .unwrap();

        let impact = graph.get_impact_set("c");
        assert_eq!(impact.len(), 2);
        assert!(impact.contains("a"));
        assert!(impact.contains("b"));
    }

    #[test]
    fn test_graph_statistics() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node(DependencyNode::new("a".into(), "A".into()))
            .unwrap();
        graph
            .add_node(DependencyNode::new("b".into(), "B".into()))
            .unwrap();

        graph
            .add_edge(DependencyEdge::new("a".into(), "b".into(), DependencyType::Hard))
            .unwrap();

        let stats = graph.statistics();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_edges, 1);
        assert_eq!(stats.hard_edges, 1);
        assert!(!stats.has_cycles);
    }
}
