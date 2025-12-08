//! Network Analysis
//!
//! Oracle Spatial Network Data Model compatible operations:
//! - Road network modeling
//! - Shortest path algorithms (Dijkstra, A*)
//! - Traveling salesman problem
//! - Service area computation
//! - Network routing and optimization

use crate::error::Result;
use crate::spatial::geometry::Coordinate;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::f64;

/// Network node
#[derive(Debug, Clone)]
pub struct Node {
    pub id: u64,
    pub coord: Coordinate,
    pub properties: HashMap<String, String>,
}

impl Node {
    pub fn new(id: u64, coord: Coordinate) -> Self {
        Self {
            id,
            coord,
            properties: HashMap::new(),
        }
    }
}

/// Network edge (link)
#[derive(Debug, Clone)]
pub struct Edge {
    pub id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub cost: f64,
    pub reverse_cost: Option<f64>, // For one-way streets
    pub geometry: Vec<Coordinate>,
    pub properties: HashMap<String, String>,
}

impl Edge {
    pub fn new(id: u64, from_node: u64, to_node: u64, cost: f64) -> Self {
        Self {
            id,
            from_node,
            to_node,
            cost,
            reverse_cost: Some(cost), // Bidirectional by default
            geometry: Vec::new(),
            properties: HashMap::new(),
        }
    }

    pub fn one_way(id: u64, from_node: u64, to_node: u64, cost: f64) -> Self {
        Self {
            id,
            from_node,
            to_node,
            cost,
            reverse_cost: None, // One-way
            geometry: Vec::new(),
            properties: HashMap::new(),
        }
    }

    pub fn is_bidirectional(&self) -> bool {
        self.reverse_cost.is_some()
    }
}

/// Network graph
pub struct Network {
    pub nodes: HashMap<u64, Node>,
    pub edges: HashMap<u64, Edge>,
    adjacency: HashMap<u64, Vec<u64>>, // node_id -> edge_ids
}

impl Network {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a node to the network
    pub fn add_node(&mut self, node: Node) {
        let node_id = node.id;
        self.nodes.insert(node_id, node);
        self.adjacency.entry(node_id).or_insert_with(Vec::new);
    }

    /// Add an edge to the network
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
        if !self.nodes.contains_key(&edge.from_node) {
            return Err(DbError::InvalidInput(format!(
                "From node {} not found",
                edge.from_node
            )));
        }

        if !self.nodes.contains_key(&edge.to_node) {
            return Err(DbError::InvalidInput(format!(
                "To node {} not found",
                edge.to_node
            )));
        }

        let edge_id = edge.id;
        let from_node = edge.from_node;
        let _to_node = edge.to_node;
        let is_bidirectional = edge.is_bidirectional();

        self.edges.insert(edge_id, edge);

        self.adjacency
            .entry(from_node)
            .or_insert_with(Vec::new)
            .push(edge_id);

        if is_bidirectional {
            self.adjacency
                .entry(to_node)
                .or_insert_with(Vec::new)
                .push(edge_id);
        }

        Ok(())
    }

    /// Get outgoing edges from a node
    pub fn get_outgoing_edges(&self, node_id: u64) -> Vec<&Edge> {
        self.adjacency
            .get(&node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|edge_id| self.edges.get(edge_id))
                    .filter(|edge| edge.from_node == node_id || edge.is_bidirectional())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the cost of traversing an edge from a specific node
    pub fn get_edge_cost(&self, edge: &Edge, from_node: u64) -> Option<f64> {
        if edge.from_node == from_node {
            Some(edge.cost)
        } else if edge.to_node == from_node && edge.is_bidirectional() {
            edge.reverse_cost
        } else {
            None
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

/// Dijkstra's shortest path algorithm
pub struct DijkstraRouter<'a> {
    network: &'a Network,
}

#[derive(Clone)]
struct DijkstraState {
    node_id: u64,
    cost: f64,
}

impl Eq for DijkstraState {}

impl PartialEq for DijkstraState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node_id == other.node_id
    }
}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<u64>,
    pub edges: Vec<u64>,
    pub total_cost: f64,
}

impl<'a> DijkstraRouter<'a> {
    pub fn new(network: &'a Network) -> Self {
        Self { network }
    }

    /// Find shortest path from start to end node
    pub fn shortest_path(&self, start: u64, end: u64) -> Result<Path> {
        if !self.network.nodes.contains_key(&start) {
            return Err(DbError::InvalidInput(format!("Start node {} not found", start)));
        }

        if !self.network.nodes.contains_key(&end) {
            return Err(DbError::InvalidInput(format!("End node {} not found", end)));
        }

        let mut distances: HashMap<u64, f64> = HashMap::new();
        let mut previous: HashMap<u64, (u64, u64)> = HashMap::new(); // node_id -> (prev_node, edge_id)
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push(DijkstraState {
            node_id: start,
            cost: 0.0,
        });

        while let Some(DijkstraState { node_id, cost }) = heap.pop() {
            if node_id == end {
                return Ok(self.reconstruct_path(start, end, &previous, cost));
            }

            if cost > *distances.get(&node_id).unwrap_or(&f64::INFINITY) {
                continue;
            }

            for edge in self.network.get_outgoing_edges(node_id) {
                let next_node = if edge.from_node == node_id {
                    edge.to_node
                } else {
                    edge.from_node
                };

                let edge_cost = self.network.get_edge_cost(edge, node_id).unwrap_or(f64::INFINITY);
                let next_cost = cost + edge_cost;

                if next_cost < *distances.get(&next_node).unwrap_or(&f64::INFINITY) {
                    distances.insert(next_node, next_cost);
                    previous.insert(next_node, (node_id, edge.id));
                    heap.push(DijkstraState {
                        node_id: next_node,
                        cost: next_cost,
                    });
                }
            }
        }

        Err(DbError::InvalidInput("No path found".to_string()))
    }

    fn reconstruct_path(
        &self,
        start: u64,
        end: u64,
        previous: &HashMap<u64, (u64, u64)>,
        total_cost: f64,
    ) -> Path {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut current = end;

        while current != start {
            nodes.push(current);
            if let Some(&(prev_node, edge_id)) = previous.get(&current) {
                edges.push(edge_id);
                current = prev_node;
            } else {
                break;
            }
        }

        nodes.push(start);
        nodes.reverse();
        edges.reverse();

        Path {
            nodes,
            edges,
            total_cost,
        }
    }

    /// Find shortest paths from start to all reachable nodes
    pub fn shortest_path_tree(&self, start: u64) -> HashMap<u64, f64> {
        let mut distances: HashMap<u64, f64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push(DijkstraState {
            node_id: start,
            cost: 0.0,
        });

        while let Some(DijkstraState { node_id, cost }) = heap.pop() {
            if cost > *distances.get(&node_id).unwrap_or(&f64::INFINITY) {
                continue;
            }

            for edge in self.network.get_outgoing_edges(node_id) {
                let next_node = if edge.from_node == node_id {
                    edge.to_node
                } else {
                    edge.from_node
                };

                let edge_cost = self.network.get_edge_cost(edge, node_id).unwrap_or(f64::INFINITY);
                let next_cost = cost + edge_cost;

                if next_cost < *distances.get(&next_node).unwrap_or(&f64::INFINITY) {
                    distances.insert(next_node, next_cost);
                    heap.push(DijkstraState {
                        node_id: next_node,
                        cost: next_cost,
                    });
                }
            }
        }

        distances
    }
}

/// A* shortest path algorithm with heuristic
pub struct AStarRouter<'a> {
    network: &'a Network,
}

#[derive(Clone)]
struct AStarState {
    node_id: u64,
    cost: f64,
    heuristic: f64,
}

impl Eq for AStarState {}

impl PartialEq for AStarState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node_id == other.node_id
    }
}

impl Ord for AStarState {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_f = self.cost + self.heuristic;
        let other_f = other.cost + other.heuristic;

        other_f
            .partial_cmp(&self_f)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> AStarRouter<'a> {
    pub fn new(network: &'a Network) -> Self {
        Self { network }
    }

    /// Find shortest path using A* with Euclidean distance heuristic
    pub fn shortest_path(&self, start: u64, end: u64) -> Result<Path> {
        if !self.network.nodes.contains_key(&start) {
            return Err(DbError::InvalidInput(format!("Start node {} not found", start)));
        }

        if !self.network.nodes.contains_key(&end) {
            return Err(DbError::InvalidInput(format!("End node {} not found", end)));
        }

        let end_coord = &self.network.nodes[&end].coord;

        let mut g_scores: HashMap<u64, f64> = HashMap::new();
        let mut previous: HashMap<u64, (u64, u64)> = HashMap::new();
        let mut heap = BinaryHeap::new();

        g_scores.insert(start, 0.0);
        heap.push(AStarState {
            node_id: start,
            cost: 0.0,
            heuristic: self.heuristic(start, end_coord),
        });

        while let Some(AStarState { node_id, cost, .. }) = heap.pop() {
            if node_id == end {
                return Ok(self.reconstruct_path(start, end, &previous, cost));
            }

            if cost > *g_scores.get(&node_id).unwrap_or(&f64::INFINITY) {
                continue;
            }

            for edge in self.network.get_outgoing_edges(node_id) {
                let next_node = if edge.from_node == node_id {
                    edge.to_node
                } else {
                    edge.from_node
                };

                let edge_cost = self.network.get_edge_cost(edge, node_id).unwrap_or(f64::INFINITY);
                let next_cost = cost + edge_cost;

                if next_cost < *g_scores.get(&next_node).unwrap_or(&f64::INFINITY) {
                    g_scores.insert(next_node, next_cost);
                    previous.insert(next_node, (node_id, edge.id));

                    heap.push(AStarState {
                        node_id: next_node,
                        cost: next_cost,
                        heuristic: self.heuristic(next_node, end_coord),
                    });
                }
            }
        }

        Err(DbError::InvalidInput("No path found".to_string()))
    }

    fn heuristic(&self, node_id: u64, target_coord: &Coordinate) -> f64 {
        if let Some(node) = self.network.nodes.get(&node_id) {
            node.coord.distance_2d(target_coord)
        } else {
            0.0
        }
    }

    fn reconstruct_path(
        &self,
        start: u64,
        end: u64,
        previous: &HashMap<u64, (u64, u64)>,
        total_cost: f64,
    ) -> Path {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut current = end;

        while current != start {
            nodes.push(current);
            if let Some(&(prev_node, edge_id)) = previous.get(&current) {
                edges.push(edge_id);
                current = prev_node;
            } else {
                break;
            }
        }

        nodes.push(start);
        nodes.reverse();
        edges.reverse();

        Path {
            nodes,
            edges,
            total_cost,
        }
    }
}

/// Service area analysis (isochrones)
pub struct ServiceAreaAnalyzer<'a> {
    network: &'a Network,
}

#[derive(Debug, Clone)]
pub struct ServiceArea {
    pub center: u64,
    pub max_cost: f64,
    pub nodes: Vec<u64>,
    pub boundary_nodes: Vec<u64>,
}

impl<'a> ServiceAreaAnalyzer<'a> {
    pub fn new(network: &'a Network) -> Self {
        Self { network }
    }

    /// Calculate service area from a center point with max cost
    pub fn calculate(&self, center: u64, max_cost: f64) -> Result<ServiceArea> {
        if !self.network.nodes.contains_key(&center) {
            return Err(DbError::InvalidInput(format!("Center node {} not found", center)));
        }

        let router = DijkstraRouter::new(self.network);
        let distances = router.shortest_path_tree(center);

        let mut reachable_nodes = Vec::new();
        let mut boundary_nodes = Vec::new();

        for (&node_id, &distance) in &distances {
            if distance <= max_cost {
                reachable_nodes.push(node_id);

                // Check if this is a boundary node
                let is_boundary = self
                    .network
                    .get_outgoing_edges(node_id)
                    .iter()
                    .any(|edge| {
                        let next_node = if edge.from_node == node_id {
                            edge.to_node
                        } else {
                            edge.from_node
                        };

                        distances.get(&next_node).map_or(true, |&d| d > max_cost)
                    });

                if is_boundary {
                    boundary_nodes.push(node_id);
                }
            }
        }

        Ok(ServiceArea {
            center,
            max_cost,
            nodes: reachable_nodes,
            boundary_nodes,
        })
    }

    /// Calculate multiple service areas (isochrones)
    pub fn isochrones(&self, center: u64, cost_intervals: &[f64]) -> Result<Vec<ServiceArea>> {
        let mut areas = Vec::new();

        for &max_cost in cost_intervals {
            areas.push(self.calculate(center, max_cost)?);
        }

        Ok(areas)
    }
}

/// Traveling Salesman Problem (TSP) solver
pub struct TspSolver<'a> {
    network: &'a Network,
}

impl<'a> TspSolver<'a> {
    pub fn new(network: &'a Network) -> Self {
        Self { network }
    }

    /// Solve TSP using nearest neighbor heuristic
    pub fn solve_nearest_neighbor(&self, nodes: &[u64]) -> Result<Path> {
        if nodes.is_empty() {
            return Err(DbError::InvalidInput("No nodes provided".to_string()));
        }

        let router = DijkstraRouter::new(self.network);
        let mut unvisited: HashSet<u64> = nodes.iter().copied().collect();
        let mut tour = Vec::new();
        let mut tour_edges = Vec::new();
        let mut total_cost = 0.0;

        let mut current = nodes[0];
        tour.push(current);
        unvisited.remove(&current);

        while !unvisited.is_empty() {
            let mut nearest_node = None;
            let mut min_cost = f64::INFINITY;
            let mut best_path = None;

            // Find nearest unvisited node
            for &node in &unvisited {
                if let Ok(path) = router.shortest_path(current, node) {
                    if path.total_cost < min_cost {
                        min_cost = path.total_cost;
                        nearest_node = Some(node);
                        best_path = Some(path);
                    }
                }
            }

            if let (Some(next_node), Some(path)) = (nearest_node, best_path) {
                tour.extend(path.nodes.iter().skip(1));
                tour_edges.extend(path.edges);
                total_cost += path.total_cost;
                current = next_node;
                unvisited.remove(&next_node);
            } else {
                return Err(DbError::InvalidInput("Cannot complete tour".to_string()));
            }
        }

        // Return to start
        if let Ok(return_path) = router.shortest_path(current, nodes[0]) {
            tour.extend(return_path.nodes.iter().skip(1));
            tour_edges.extend(return_path.edges);
            total_cost += return_path.total_cost;
        }

        Ok(Path {
            nodes: tour,
            edges: tour_edges,
            total_cost,
        })
    }

    /// Improve tour using 2-opt local search
    pub fn two_opt_improve(&self, mut path: Path, max_iterations: usize) -> Result<Path> {
        let router = DijkstraRouter::new(self.network);
        let mut improved = true;
        let mut iteration = 0;

        while improved && iteration < max_iterations {
            improved = false;
            iteration += 1;

            for _i in 1..path.nodes.len() - 2 {
                for j in i + 1..path.nodes.len() - 1 {
                    // Calculate cost of current edges
                    let current_cost = router.shortest_path(path.nodes[i - 1], path.nodes[i])
                        .map(|p| p.total_cost)
                        .unwrap_or(f64::INFINITY)
                        + router.shortest_path(path.nodes[j], path.nodes[j + 1])
                            .map(|p| p.total_cost)
                            .unwrap_or(f64::INFINITY);

                    // Calculate cost of swapped edges
                    let new_cost = router.shortest_path(path.nodes[i - 1], path.nodes[j])
                        .map(|p| p.total_cost)
                        .unwrap_or(f64::INFINITY)
                        + router.shortest_path(path.nodes[i], path.nodes[j + 1])
                            .map(|p| p.total_cost)
                            .unwrap_or(f64::INFINITY);

                    if new_cost < current_cost {
                        // Reverse the segment between i and j
                        path.nodes[i..=j].reverse();
                        improved = true;
                    }
                }
            }
        }

        // Recalculate total cost
        let mut total_cost = 0.0;
        let mut edges = Vec::new();

        for _i in 0..path.nodes.len() - 1 {
            if let Ok(segment) = router.shortest_path(path.nodes[i], path.nodes[i + 1]) {
                total_cost += segment.total_cost;
                edges.extend(segment.edges);
            }
        }

        path.total_cost = total_cost;
        path.edges = edges;

        Ok(path)
    }
}

/// Turn restrictions for routing
#[derive(Debug, Clone)]
pub struct TurnRestriction {
    pub from_edge: u64,
    pub via_node: u64,
    pub to_edge: u64,
    pub restriction_type: RestrictionType,
    pub cost: f64, // Additional cost for non-prohibited turns
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestrictionType {
    Prohibited,     // No turn allowed
    Mandatory,      // Only this turn allowed
    Discouraged,    // Allowed but with additional cost
}

/// Network with turn restrictions
pub struct RestrictedNetwork {
    pub network: Network,
    pub restrictions: Vec<TurnRestriction>,
}

impl RestrictedNetwork {
    pub fn new(network: Network) -> Self {
        Self {
            network,
            restrictions: Vec::new(),
        }
    }

    pub fn add_restriction(&mut self, restriction: TurnRestriction) {
        self.restrictions.push(restriction);
    }

    /// Check if a turn is allowed
    pub fn is_turn_allowed(&self, from_edge: u64, via_node: u64, to_edge: u64) -> bool {
        for restriction in &self.restrictions {
            if restriction.from_edge == from_edge
                && restriction.via_node == via_node
                && restriction.to_edge == to_edge
            {
                return restriction.restriction_type != RestrictionType::Prohibited;
            }
        }
        true
    }

    /// Get additional cost for a turn
    pub fn get_turn_cost(&self, from_edge: u64, via_node: u64, to_edge: u64) -> f64 {
        for restriction in &self.restrictions {
            if restriction.from_edge == from_edge
                && restriction.via_node == via_node
                && restriction.to_edge == to_edge
            {
                if restriction.restriction_type == RestrictionType::Discouraged {
                    return restriction.cost;
                }
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_creation() {
        let mut network = Network::new();

        network.add_node(Node::new(1, Coordinate::new(0.0, 0.0)));
        network.add_node(Node::new(2, Coordinate::new(1.0, 0.0)));
        network.add_node(Node::new(3, Coordinate::new(2.0, 0.0)));

        assert_eq!(network.nodes.len(), 3);
    }

    #[test]
    fn test_dijkstra_shortest_path() {
        let mut network = Network::new();

        network.add_node(Node::new(1, Coordinate::new(0.0, 0.0)));
        network.add_node(Node::new(2, Coordinate::new(1.0, 0.0)));
        network.add_node(Node::new(3, Coordinate::new(2.0, 0.0)));

        network.add_edge(Edge::new(1, 1, 2, 1.0)).unwrap();
        network.add_edge(Edge::new(2, 2, 3, 1.0)).unwrap();
        network.add_edge(Edge::new(3, 1, 3, 5.0)).unwrap();

        let router = DijkstraRouter::new(&network);
        let path = router.shortest_path(1, 3).unwrap();

        assert_eq!(path.total_cost, 2.0);
        assert_eq!(path.nodes, vec![1, 2, 3]);
    }

    #[test]
    fn test_service_area() {
        let mut network = Network::new();

        network.add_node(Node::new(1, Coordinate::new(0.0, 0.0)));
        network.add_node(Node::new(2, Coordinate::new(1.0, 0.0)));
        network.add_node(Node::new(3, Coordinate::new(2.0, 0.0)));
        network.add_node(Node::new(4, Coordinate::new(3.0, 0.0)));

        network.add_edge(Edge::new(1, 1, 2, 1.0)).unwrap();
        network.add_edge(Edge::new(2, 2, 3, 1.0)).unwrap();
        network.add_edge(Edge::new(3, 3, 4, 1.0)).unwrap();

        let analyzer = ServiceAreaAnalyzer::new(&network);
        let area = analyzer.calculate(1, 2.5).unwrap();

        assert!(area.nodes.contains(&1));
        assert!(area.nodes.contains(&2));
        assert!(area.nodes.contains(&3));
        assert!(!area.nodes.contains(&4));
    }
}


