// # Graph Analytics Engine
//
// Advanced graph analytics capabilities:
// - Graph-relational integration
// - MATCH clause execution
// - Path enumeration
// - Temporal graph analysis
// - Graph machine learning features
// - Recommendation engine basics

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};
use crate::common::{Value, Tuple};
use super::property_graph::{PropertyGraph, VertexId, EdgeId, Properties};
use super::query_engine::{GraphQuery, QueryResult, VariableBindings, PatternMatcher};
use super::algorithms::{PageRank, PageRankConfig};

// ============================================================================
// Graph-Relational Integration
// ============================================================================

/// Graph-relational bridge for integrating graph and relational data
pub struct GraphRelationalBridge {
    /// Mapping from relational table to graph vertices
    table_vertex_map: HashMap<String, HashMap<Value, VertexId>>,

    /// Mapping from relational foreign keys to graph edges
    fk_edge_map: HashMap<(String, String), Vec<EdgeId>>,
}

impl GraphRelationalBridge {
    pub fn new() -> Self {
        Self {
            table_vertex_map: HashMap::new(),
            fk_edge_map: HashMap::new(),
        }
    }

    /// Map a relational table to graph vertices
    pub fn map_table_to_vertices(
        &mut self,
        table_name: String,
        _primary_key_col: &str,
        tuples: &[Tuple],
        graph: &mut PropertyGraph,
        label: String,
    ) -> Result<()> {
        let mut pk_vertex_map = HashMap::new();

        for (row_idx, _tuple) in tuples.iter().enumerate() {
            // Use row index as key since Value doesn't implement Hash
            let pk_value = Value::Integer(row_idx as i64);

            // Create vertex with properties
            let properties = Properties::new();

            // Create vertex
            let vertex_id = graph.add_vertex(vec![label.clone()], properties)?;
            pk_vertex_map.insert(pk_value, vertex_id);
        }

        self.table_vertex_map.insert(table_name, pk_vertex_map);
        Ok(())
    }

    /// Map foreign key relationships to edges
    pub fn map_foreign_key_to_edges(
        &mut self,
        source_table: String,
        target_table: String,
        fk_relationships: &[(Value, Value)],
        graph: &mut PropertyGraph,
        edge_label: String,
    ) -> Result<()> {
        let source_map = self.table_vertex_map.get(&source_table)
            .ok_or_else(|| DbError::Internal("Source table not mapped".to_string()))?;

        let target_map = self.table_vertex_map.get(&target_table)
            .ok_or_else(|| DbError::Internal("Target table not mapped".to_string()))?;

        let mut edge_ids = Vec::new();

        for (source_fk, target_pk) in fk_relationships {
            if let (Some(&source_vertex), Some(&target_vertex)) =
                (source_map.get(source_fk), target_map.get(target_pk))
            {
                let edge_id = graph.add_edge(
                    source_vertex,
                    target_vertex,
                    edge_label.clone(),
                    Properties::new(),
                    super::property_graph::EdgeDirection::Directed,
                )?;

                edge_ids.push(edge_id);
            }
        }

        self.fk_edge_map.insert((source_table, target_table), edge_ids);
        Ok(())
    }

    /// Execute a graph query and return results in relational format
    pub fn graph_to_relational(&self, query_result: &QueryResult) -> Vec<Tuple> {
        let mut tuples = Vec::new();

        for (row_idx, row) in query_result.rows.iter().enumerate() {
            let mut tuple_values = Vec::new();

            for value in &row.values {
                match value {
                    super::query_engine::ResultValue::Property(v) => {
                        tuple_values.push(v.clone());
                    }
                    super::query_engine::ResultValue::Vertex(id, _) => {
                        tuple_values.push(Value::Integer(*id as i64));
                    }
                    _ => {
                        tuple_values.push(Value::Null);
                    }
                }
            }

            tuples.push(Tuple::new(tuple_values, row_idx as u64));
        }

        tuples
    }

    /// Get vertex ID from table primary key
    pub fn get_vertex_from_pk(&self, table_name: &str, pk_value: &Value) -> Option<VertexId> {
        self.table_vertex_map
            .get(table_name)
            .and_then(|map| map.get(pk_value).copied())
    }
}

impl Default for GraphRelationalBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MATCH Clause Executor
// ============================================================================

/// Advanced MATCH clause executor with optimizations
pub struct MatchExecutor<'a> {
    graph: &'a PropertyGraph,
    pattern_matcher: PatternMatcher<'a>,
}

impl<'a> MatchExecutor<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self {
            graph,
            pattern_matcher: PatternMatcher::new(graph),
        }
    }

    /// Execute a MATCH query with cost-based optimization
    pub fn execute_match(&self, query: &GraphQuery) -> Result<Vec<VariableBindings>> {
        let mut all_bindings = vec![VariableBindings::new()];

        for match_clause in &query.match_clauses {
            for pattern in &match_clause.patterns {
                // Optimize pattern matching order
                let optimized_pattern = self.optimize_pattern(pattern)?;

                // Execute pattern matching
                let new_bindings = self.pattern_matcher.match_pattern(&optimized_pattern)?;

                // Merge with existing bindings
                all_bindings = self.merge_bindings(all_bindings, new_bindings)?;
            }
        }

        Ok(all_bindings)
    }

    /// Optimize pattern matching order based on selectivity
    fn optimize_pattern(
        &self,
        pattern: &super::query_engine::GraphPattern,
    ) -> Result<super::query_engine::GraphPattern> {
        // Simple optimization: start with most selective patterns
        // In practice, this would use statistics and cost models

        Ok(pattern.clone())
    }

    /// Merge two sets of bindings
    fn merge_bindings(
        &self,
        existing: Vec<VariableBindings>,
        new: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        let mut merged = Vec::new();

        for existing_binding in &existing {
            for new_binding in &new {
                let mut combined = existing_binding.clone();

                // Check for conflicts in variable bindings
                let mut conflict = false;
                for (var, &vertex_id) in &new_binding.vertices {
                    if let Some(&existing_id) = combined.vertices.get(var) {
                        if existing_id != vertex_id {
                            conflict = true;
                            break;
                        }
                    } else {
                        combined.vertices.insert(var.clone(), vertex_id);
                    }
                }

                if !conflict {
                    merged.push(combined);
                }
            }
        }

        Ok(merged)
    }
}

// ============================================================================
// Path Enumeration
// ============================================================================

/// Path enumeration with constraints
pub struct PathEnumerator<'a> {
    graph: &'a PropertyGraph,
}

impl<'a> PathEnumerator<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph }
    }

    /// Enumerate all simple paths between two vertices
    pub fn enumerate_simple_paths(
        &self,
        start: VertexId,
        end: VertexId,
        max_length: usize,
    ) -> Result<Vec<Vec<VertexId>>> {
        let mut paths = Vec::new();
        let mut current_path = vec![start];
        let mut visited = HashSet::new();
        visited.insert(start);

        self.dfs_enumerate(
            start,
            end,
            max_length,
            &mut current_path,
            &mut visited,
            &mut paths,
        )?;

        Ok(paths)
    }

    fn dfs_enumerate(
        &self,
        current: VertexId,
        end: VertexId,
        max_length: usize,
        current_path: &mut Vec<VertexId>,
        visited: &mut HashSet<VertexId>,
        paths: &mut Vec<Vec<VertexId>>,
    ) -> Result<()> {
        if current == end {
            paths.push(current_path.clone());
            return Ok(());
        }

        if current_path.len() >= max_length {
            return Ok(());
        }

        let neighbors = self.graph.get_outgoing_neighbors(current)?;
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                current_path.push(neighbor);

                self.dfs_enumerate(neighbor, end, max_length, current_path, visited, paths)?;

                current_path.pop();
                visited.remove(&neighbor);
            }
        }

        Ok(())
    }

    /// Enumerate k-shortest paths
    pub fn enumerate_k_shortest_paths(
        &self,
        start: VertexId,
        end: VertexId,
        k: usize,
    ) -> Result<Vec<(Vec<VertexId>, f64)>> {
        // Yen's algorithm for k-shortest paths
        let mut paths = Vec::new();

        // Find shortest path
        if let Some((path, cost)) = self.dijkstra(start, end, &HashSet::new())? {
            paths.push((path, cost));
        } else {
            return Ok(paths);
        }

        for k_idx in 1..k {
            let mut candidates = Vec::new();

            let (prev_path, _) = &paths[k_idx - 1];

            for _i in 0..(prev_path.len() - 1) {
                let spur_node = prev_path[i];
                let root_path = &prev_path[0..=i];

                // Find edges to exclude
                let mut excluded_edges = HashSet::new();
                for (path, _) in &paths {
                    if path.len() > i && &path[0..=i] == root_path {
                        if path.len() > i + 1 {
                            excluded_edges.insert((path[i], path[i + 1]));
                        }
                    }
                }

                // Find spur path
                if let Some((spur_path, spur_cost)) =
                    self.dijkstra(spur_node, end, &excluded_edges)?
                {
                    let mut total_path = root_path.to_vec();
                    total_path.extend(&spur_path[1..]);

                    let total_cost = self.path_cost(&total_path)? + spur_cost;
                    candidates.push((total_path, total_cost));
                }
            }

            if candidates.is_empty() {
                break;
            }

            // Sort candidates and add the best one
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            paths.push(candidates[0].clone());
        }

        Ok(paths)
    }

    fn dijkstra(
        &self,
        start: VertexId,
        end: VertexId,
        excluded_edges: &HashSet<(VertexId, VertexId)>,
    ) -> Result<Option<(Vec<VertexId>, f64)>> {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

        #[derive(Copy, Clone)]
        struct State {
            vertex: VertexId,
            cost: f64,
        }

        impl Eq for State {}
        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.cost == other.cost && self.vertex == other.vertex
            }
        }
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut distances: HashMap<VertexId, f64> = HashMap::new();
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push(State { vertex: start, cost: 0.0 });

        while let Some(State { vertex, cost }) = heap.pop() {
            if vertex == end {
                let mut path = vec![end];
                let mut current = end;
                while current != start {
                    if let Some(&prev) = parent.get(&current) {
                        path.push(prev);
                        current = prev;
                    } else {
                        break;
                    }
                }
                path.reverse();
                return Ok(Some((path, cost)));
            }

            if cost > *distances.get(&vertex).unwrap_or(&f64::INFINITY) {
                continue;
            }

            let neighbors = self.graph.get_outgoing_neighbors(vertex)?;
            for next in neighbors {
                if excluded_edges.contains(&(vertex, next)) {
                    continue;
                }

                let edge_weight = 1.0; // Simplified
                let next_cost = cost + edge_weight;

                if next_cost < *distances.get(&next).unwrap_or(&f64::INFINITY) {
                    distances.insert(next, next_cost);
                    parent.insert(next, vertex);
                    heap.push(State { vertex: next, cost: next_cost });
                }
            }
        }

        Ok(None)
    }

    fn path_cost(&self, path: &[VertexId]) -> Result<f64> {
        let mut cost = 0.0;
        for _ in 0..(path.len() - 1) {
            cost += 1.0; // Simplified: assume unit weights
        }
        Ok(cost)
    }
}

// ============================================================================
// Temporal Graph Analysis
// ============================================================================

/// Temporal graph for time-varying networks
#[derive(Debug, Clone)]
pub struct TemporalGraph {
    /// Snapshots of the graph at different time points
    snapshots: BTreeMap<i64, PropertyGraph>,

    /// Event log for incremental updates
    event_log: Vec<TemporalEvent>,
}

/// Temporal event (vertex/edge addition or removal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalEvent {
    AddVertex { timestamp: i64, vertex_id: VertexId, labels: Vec<String> },
    RemoveVertex { timestamp: i64, vertex_id: VertexId },
    AddEdge { timestamp: i64, edge_id: EdgeId, source: VertexId, target: VertexId, label: String },
    RemoveEdge { timestamp: i64, edge_id: EdgeId },
}

impl TemporalGraph {
    pub fn new() -> Self {
        Self {
            snapshots: BTreeMap::new(),
            event_log: Vec::new(),
        }
    }

    /// Add a snapshot at a specific time
    pub fn add_snapshot(&mut self, timestamp: i64, graph: PropertyGraph) {
        self.snapshots.insert(timestamp, graph);
    }

    /// Get snapshot at a specific time
    pub fn get_snapshot(&self, timestamp: i64) -> Option<&PropertyGraph> {
        self.snapshots.get(&timestamp)
    }

    /// Get snapshot closest to a timestamp
    pub fn get_closest_snapshot(&self, timestamp: i64) -> Option<&PropertyGraph> {
        self.snapshots.range(..=timestamp).next_back().map(|(_, graph)| graph)
    }

    /// Record a temporal event
    pub fn record_event(&mut self, event: TemporalEvent) {
        self.event_log.push(event);
    }

    /// Get events in a time range
    pub fn get_events(&self, start: i64, end: i64) -> Vec<&TemporalEvent> {
        self.event_log.iter()
            .filter(|event| {
                let ts = match event {
                    TemporalEvent::AddVertex { timestamp, .. } => *timestamp,
                    TemporalEvent::RemoveVertex { timestamp, .. } => *timestamp,
                    TemporalEvent::AddEdge { timestamp, .. } => *timestamp,
                    TemporalEvent::RemoveEdge { timestamp, .. } => *timestamp,
                };
                ts >= start && ts <= end
            })
            .collect()
    }

    /// Compute temporal metrics
    pub fn compute_temporal_metrics(&self, vertex: VertexId) -> Result<TemporalMetrics> {
        let mut appearance_count = 0;
        let mut first_appearance = None;
        let mut last_appearance = None;

        for (timestamp, graph) in &self.snapshots {
            if graph.get_vertex(vertex).is_some() {
                appearance_count += 1;
                if first_appearance.is_none() {
                    first_appearance = Some(*timestamp);
                }
                last_appearance = Some(*timestamp);
            }
        }

        Ok(TemporalMetrics {
            vertex_id: vertex,
            appearance_count,
            first_appearance,
            last_appearance,
            lifespan: if let (Some(first), Some(last)) = (first_appearance, last_appearance) {
                Some(last - first)
            } else {
                None
            },
        })
    }
}

impl Default for TemporalGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporal metrics for a vertex
#[derive(Debug, Clone)]
pub struct TemporalMetrics {
    pub vertex_id: VertexId,
    pub appearance_count: usize,
    pub first_appearance: Option<i64>,
    pub last_appearance: Option<i64>,
    pub lifespan: Option<i64>,
}

// ============================================================================
// Graph Machine Learning Features
// ============================================================================

/// Graph embedding generator
pub struct GraphEmbedding;

impl GraphEmbedding {
    /// Generate simple degree-based features
    pub fn degree_features(graph: &PropertyGraph) -> HashMap<VertexId, Vec<f64>> {
        let mut features = HashMap::new();

        for vertex in graph.vertices() {
            let in_degree = vertex.in_degree() as f64;
            let out_degree = vertex.out_degree() as f64;
            let total_degree = in_degree + out_degree;

            features.insert(vertex.id, vec![in_degree, out_degree, total_degree]);
        }

        features
    }

    /// Generate PageRank-based features
    pub fn pagerank_features(graph: &PropertyGraph) -> Result<HashMap<VertexId, Vec<f64>>> {
        let config = PageRankConfig::default();
        let _result = PageRank::compute(graph, &config)?;

        let mut features = HashMap::new();
        for (vertex_id, score) in result.scores {
            features.insert(vertex_id, vec![score]);
        }

        Ok(features)
    }

    /// Generate local clustering coefficient features
    pub fn clustering_features(graph: &PropertyGraph) -> HashMap<VertexId, Vec<f64>> {
        let mut features = HashMap::new();

        for vertex in graph.vertices() {
            let coefficient = Self::local_clustering_coefficient(graph, vertex.id)
                .unwrap_or(0.0);
            features.insert(vertex.id, vec![coefficient]);
        }

        features
    }

    fn local_clustering_coefficient(graph: &PropertyGraph, vertex: VertexId) -> Result<f64> {
        let neighbors = graph.get_outgoing_neighbors(vertex)?;
        let k = neighbors.len();

        if k < 2 {
            return Ok(0.0);
        }

        let neighbors_set: HashSet<VertexId> = neighbors.iter().copied().collect();
        let mut edges_between = 0;

        for &n1 in &neighbors {
            let n1_neighbors = graph.get_outgoing_neighbors(n1)?;
            for &n2 in &n1_neighbors {
                if neighbors_set.contains(&n2) {
                    edges_between += 1;
                }
            }
        }

        Ok(edges_between as f64 / (k * (k - 1)) as f64)
    }
}

// ============================================================================
// Recommendation Engine
// ============================================================================

/// Graph-based recommendation engine
pub struct RecommendationEngine<'a> {
    graph: &'a PropertyGraph,
}

impl<'a> RecommendationEngine<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph }
    }

    /// Collaborative filtering recommendations
    pub fn collaborative_filtering(
        &self,
        user_vertex: VertexId,
        top_k: usize,
    ) -> Result<Vec<(VertexId, f64)>> {
        // Find similar users based on common neighbors
        let user_neighbors = self.graph.get_outgoing_neighbors(user_vertex)?;
        let user_neighbors_set: HashSet<VertexId> = user_neighbors.iter().copied().collect();

        let mut candidate_scores: HashMap<VertexId, f64> = HashMap::new();

        // For each item the user hasn't interacted with
        for vertex in self.graph.vertices() {
            if vertex.id == user_vertex || user_neighbors_set.contains(&vertex.id) {
                continue;
            }

            // Calculate score based on common neighbors
            let vertex_neighbors = self.graph.get_incoming_neighbors(vertex.id)?;
            let common = vertex_neighbors.iter()
                .filter(|n| user_neighbors_set.contains(n))
                .count();

            if common > 0 {
                candidate_scores.insert(vertex.id, common as f64);
            }
        }

        // Sort by score and return top-k
        let mut recommendations: Vec<(VertexId, f64)> = candidate_scores.into_iter().collect();
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recommendations.truncate(top_k);

        Ok(recommendations)
    }

    /// Content-based recommendations using property similarity
    pub fn content_based(
        &self,
        item_vertex: VertexId,
        top_k: usize,
    ) -> Result<Vec<(VertexId, f64)>> {
        let item = self.graph.get_vertex(item_vertex)
            .ok_or_else(|| DbError::Internal("Vertex not found".to_string()))?;

        let mut similarities: HashMap<VertexId, f64> = HashMap::new();

        for vertex in self.graph.vertices() {
            if vertex.id == item_vertex {
                continue;
            }

            // Calculate property-based similarity
            let similarity = self.property_similarity(&item.properties, &vertex.properties);
            if similarity > 0.0 {
                similarities.insert(vertex.id, similarity);
            }
        }

        let mut recommendations: Vec<(VertexId, f64)> = similarities.into_iter().collect();
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recommendations.truncate(top_k);

        Ok(recommendations)
    }

    /// Random walk with restart for recommendations
    pub fn random_walk_with_restart(
        &self,
        start_vertex: VertexId,
        restart_prob: f64,
        num_iterations: usize,
        top_k: usize,
    ) -> Result<Vec<(VertexId, f64)>> {
        let mut scores: HashMap<VertexId, f64> = HashMap::new();

        for _ in 0..num_iterations {
            let mut current = start_vertex;

            for _ in 0..100 {
                // Max walk length
                *scores.entry(current).or_insert(0.0) += 1.0;

                if rand::random::<f64>() < restart_prob {
                    current = start_vertex;
                } else {
                    let neighbors = self.graph.get_outgoing_neighbors(current)?;
                    if neighbors.is_empty() {
                        break;
                    }
                    let idx = (rand::random::<f64>() * neighbors.len() as f64) as usize % neighbors.len();
                    current = neighbors[idx];
                }
            }
        }

        // Normalize scores
        let total: f64 = scores.values().sum();
        for score in scores.values_mut() {
            *score /= total;
        }

        // Remove start vertex and sort
        scores.remove(&start_vertex);
        let mut recommendations: Vec<(VertexId, f64)> = scores.into_iter().collect();
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recommendations.truncate(top_k);

        Ok(recommendations)
    }

    fn property_similarity(&self, props1: &Properties, props2: &Properties) -> f64 {
        let keys1: HashSet<&String> = props1.keys().iter().copied().collect();
        let keys2: HashSet<&String> = props2.keys().iter().copied().collect();

        let common_keys: Vec<&&String> = keys1.intersection(&keys2).collect();
        if common_keys.is_empty() {
            return 0.0;
        }

        let mut matching = 0;
        for &&key in &common_keys {
            if props1.get(key) == props2.get(key) {
                matching += 1;
            }
        }

        matching as f64 / common_keys.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::property_graph::EdgeDirection;

    #[test]
    fn test_path_enumeration() {
        let mut graph = PropertyGraph::new();
        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v2, v3, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let enumerator = PathEnumerator::new(&graph);
        let paths = enumerator.enumerate_simple_paths(v1, v3, 10).unwrap();

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], vec![v1, v2, v3]);
    }

    #[test]
    fn test_temporal_graph() {
        let mut temporal = TemporalGraph::new();

        let mut graph1 = PropertyGraph::new();
        let _v1 = graph1.add_vertex(vec![], Properties::new()).unwrap();

        temporal.add_snapshot(100, graph1);

        assert!(temporal.get_snapshot(100).is_some());
        assert!(temporal.get_closest_snapshot(150).is_some());
    }
}
