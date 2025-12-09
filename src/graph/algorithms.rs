// # Graph Algorithms
//
// Comprehensive graph algorithm implementations:
// - PageRank and centrality measures
// - Connected components
// - Community detection (Louvain algorithm)
// - Triangle counting and clustering
// - Influence maximization
// - Similarity measures

use std::collections::HashSet;
use std::collections::VecDeque;
use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use crate::error::Result;
use super::property_graph::{PropertyGraph, VertexId};

// ============================================================================
// PageRank Algorithm
// ============================================================================

/// PageRank configuration
#[derive(Debug, Clone)]
pub struct PageRankConfig {
    /// Damping factor (typically 0.85)
    pub damping_factor: f64,

    /// Maximum number of iterations
    pub max_iterations: usize,

    /// Convergence threshold
    pub tolerance: f64,

    /// Personalization vector (for personalized PageRank)
    pub personalization: Option<HashMap<VertexId, f64>>,
}

impl Default for PageRankConfig {
    fn default() -> Self {
        Self {
            damping_factor: 0.85,
            max_iterations: 100,
            tolerance: 1e-6,
            personalization: None,
        }
    }
}

/// PageRank results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRankResult {
    /// PageRank scores for each vertex
    pub scores: HashMap<VertexId, f64>,

    /// Number of iterations performed
    pub iterations: usize,

    /// Whether the algorithm converged
    pub converged: bool,
}

/// PageRank algorithm implementation
pub struct PageRank;

impl PageRank {
    /// Compute PageRank scores for all vertices
    pub fn compute(graph: &PropertyGraph, config: &PageRankConfig) -> Result<PageRankResult> {
        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();
        let n = vertices.len();

        if n == 0 {
            return Ok(PageRankResult {
                scores: HashMap::new(),
                iterations: 0,
                converged: true,
            });
        }

        // Initialize scores uniformly
        let initial_score = 1.0 / n as f64;
        let mut scores: HashMap<VertexId, f64> = vertices.iter()
            .map(|&v| (v, initial_score))
            .collect();

        let mut new_scores = scores.clone();
        let teleport_prob = (1.0 - config.damping_factor) / n as f64;

        let mut iterations = 0;
        let mut converged = false;

        for iter in 0..config.max_iterations {
            iterations = iter + 1;
            let mut delta = 0.0;

            // Initialize new scores with teleportation probability
            for &vertex_id in &vertices {
                new_scores.insert(vertex_id, teleport_prob);
            }

            // Distribute scores
            for &vertex_id in &vertices {
                if let Some(vertex) = graph.get_vertex(vertex_id) {
                    let out_degree = vertex.out_degree();

                    if out_degree > 0 {
                        let score = scores.get(&vertex_id).unwrap_or(&0.0);
                        let contribution = score / out_degree as f64;

                        // Add contribution to all outgoing neighbors
                        let neighbors = graph.get_outgoing_neighbors(vertex_id)?;
                        for neighbor in neighbors {
                            *new_scores.entry(neighbor).or_insert(teleport_prob) +=
                                config.damping_factor * contribution;
                        }
                    }
                }
            }

            // Check convergence
            for &vertex_id in &vertices {
                let old = scores.get(&vertex_id).unwrap_or(&0.0);
                let new = new_scores.get(&vertex_id).unwrap_or(&0.0);
                delta += (new - old).abs();
            }

            scores = new_scores.clone();

            if delta < config.tolerance {
                converged = true;
                break;
            }
        }

        Ok(PageRankResult {
            scores,
            iterations,
            converged,
        })
    }

    /// Get top-k vertices by PageRank score
    pub fn top_k(result: &PageRankResult, k: usize) -> Vec<(VertexId, f64)> {
        let mut entries: Vec<_> = result.scores.iter()
            .map(|(&id, &score)| (id, score))
            .collect();

        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        entries.truncate(k);
        entries
    }
}

// ============================================================================
// Connected Components
// ============================================================================

/// Connected component information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedComponents {
    /// Component ID for each vertex
    pub component_map: HashMap<VertexId, usize>,

    /// Number of components
    pub num_components: usize,

    /// Size of each component
    pub component_sizes: HashMap<usize, usize>,

    /// Largest component size
    pub largest_component_size: usize,
}

/// Connected components algorithm
pub struct ConnectedComponentsAlgorithm;

impl ConnectedComponentsAlgorithm {
    /// Find all connected components in the graph
    pub fn compute(graph: &PropertyGraph) -> Result<ConnectedComponents> {
        let mut component_map = HashMap::new();
        let mut component_sizes = HashMap::new();
        let mut visited = HashSet::new();
        let mut component_id = 0;

        for vertex in graph.vertices() {
            if !visited.contains(&vertex.id) {
                let component_size = Self::dfs_component(
                    graph,
                    vertex.id,
                    component_id,
                    &mut visited,
                    &mut component_map,
                )?;

                component_sizes.insert(component_id, component_size);
                component_id += 1;
            }
        }

        let largest_component_size = component_sizes.values()
            .max()
            .copied()
            .unwrap_or(0);

        Ok(ConnectedComponents {
            component_map,
            num_components: component_id,
            component_sizes,
            largest_component_size,
        })
    }

    fn dfs_component(
        graph: &PropertyGraph,
        start: VertexId,
        component_id: usize,
        visited: &mut HashSet<VertexId>,
        component_map: &mut HashMap<VertexId, usize>,
    ) -> Result<usize> {
        let mut stack = vec![start];
        let mut size = 0;

        while let Some(vertex_id) = stack.pop() {
            if visited.contains(&vertex_id) {
                continue;
            }

            visited.insert(vertex_id);
            component_map.insert(vertex_id, component_id);
            size += 1;

            // Add all neighbors (both incoming and outgoing for undirected interpretation)
            let neighbors = graph.get_neighbors(vertex_id)?;
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }

        Ok(size)
    }

    /// Check if two vertices are in the same component
    pub fn same_component(result: &ConnectedComponents, v1: VertexId, v2: VertexId) -> bool {
        result.component_map.get(&v1) == result.component_map.get(&v2)
    }
}

// ============================================================================
// Centrality Measures
// ============================================================================

/// Betweenness centrality results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetweennessCentrality {
    pub scores: HashMap<VertexId, f64>,
}

/// Closeness centrality results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosenessCentrality {
    pub scores: HashMap<VertexId, f64>,
}

/// Degree centrality results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegreeCentrality {
    pub in_degree: HashMap<VertexId, usize>,
    pub out_degree: HashMap<VertexId, usize>,
    pub total_degree: HashMap<VertexId, usize>,
}

/// Centrality algorithms
pub struct CentralityAlgorithms;

impl CentralityAlgorithms {
    /// Compute betweenness centrality using Brandes' algorithm
    pub fn betweenness_centrality(graph: &PropertyGraph) -> Result<BetweennessCentrality> {
        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();
        let mut scores: HashMap<VertexId, f64> = vertices.iter()
            .map(|&v| (v, 0.0))
            .collect();

        for &source in &vertices {
            // BFS from source
            let mut queue = VecDeque::new();
            let mut stack = Vec::new();
            let mut dist: HashMap<VertexId, i32> = HashMap::new();
            let mut paths: HashMap<VertexId, usize> = HashMap::new();
            let mut predecessors: HashMap<VertexId, Vec<VertexId>> = HashMap::new();

            dist.insert(source, 0);
            paths.insert(source, 1);
            queue.push_back(source);

            // BFS phase
            while let Some(v) = queue.pop_front() {
                stack.push(v);

                let neighbors = graph.get_outgoing_neighbors(v)?;
                for w in neighbors {
                    // First time visiting w?
                    if !dist.contains_key(&w) {
                        dist.insert(w, dist[&v] + 1);
                        queue.push_back(w);
                    }

                    // Shortest path to w via v?
                    if dist[&w] == dist[&v] + 1 {
                        *paths.entry(w).or_insert(0) += paths[&v];
                        predecessors.entry(w).or_insert_with(Vec::new).push(v);
                    }
                }
            }

            // Accumulation phase
            let mut dependency: HashMap<VertexId, f64> = HashMap::new();

            while let Some(w) = stack.pop() {
                if let Some(preds) = predecessors.get(&w) {
                    for &v in preds {
                        let delta = (paths[&v] as f64 / paths[&w] as f64) *
                                   (1.0 + dependency.get(&w).unwrap_or(&0.0));
                        *dependency.entry(v).or_insert(0.0) += delta;
                    }
                }

                if w != source {
                    *scores.entry(w).or_insert(0.0) += dependency.get(&w).unwrap_or(&0.0);
                }
            }
        }

        // Normalize for undirected graphs
        let n = vertices.len() as f64;
        if n > 2.0 {
            let normalization = 2.0 / ((n - 1.0) * (n - 2.0));
            for score in scores.values_mut() {
                *score *= normalization;
            }
        }

        Ok(BetweennessCentrality { scores })
    }

    /// Compute closeness centrality
    pub fn closeness_centrality(graph: &PropertyGraph) -> Result<ClosenessCentrality> {
        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();
        let mut scores: HashMap<VertexId, f64> = HashMap::new();

        for &source in &vertices {
            let distances = Self::bfs_distances(graph, source)?;
            let total_distance: usize = distances.values().sum();

            let closeness = if total_distance > 0 {
                (distances.len() - 1) as f64 / total_distance as f64
            } else {
                0.0
            };

            scores.insert(source, closeness);
        }

        Ok(ClosenessCentrality { scores })
    }

    /// Compute degree centrality
    pub fn degree_centrality(graph: &PropertyGraph) -> Result<DegreeCentrality> {
        let mut in_degree = HashMap::new();
        let mut out_degree = HashMap::new();
        let mut total_degree = HashMap::new();

        for vertex in graph.vertices() {
            let in_deg = vertex.in_degree();
            let out_deg = vertex.out_degree();

            in_degree.insert(vertex.id, in_deg);
            out_degree.insert(vertex.id, out_deg);
            total_degree.insert(vertex.id, in_deg + out_deg);
        }

        Ok(DegreeCentrality {
            in_degree,
            out_degree,
            total_degree,
        })
    }

    fn bfs_distances(graph: &PropertyGraph, start: VertexId) -> Result<HashMap<VertexId, usize>> {
        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        distances.insert(start, 0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist = distances[&current];

            let neighbors = graph.get_outgoing_neighbors(current)?;
            for neighbor in neighbors {
                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, current_dist + 1);
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(distances)
    }
}

// ============================================================================
// Community Detection (Louvain Algorithm)
// ============================================================================

/// Community detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityDetectionResult {
    /// Community ID for each vertex
    pub communities: HashMap<VertexId, usize>,

    /// Number of communities found
    pub num_communities: usize,

    /// Modularity score
    pub modularity: f64,
}

/// Louvain community detection algorithm
pub struct LouvainAlgorithm;

impl LouvainAlgorithm {
    /// Detect communities using the Louvain algorithm
    pub fn detect(graph: &PropertyGraph, max_iterations: usize) -> Result<CommunityDetectionResult> {
        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();

        // Initialize each vertex in its own community
        let mut communities: HashMap<VertexId, usize> = vertices.iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        let total_edges = graph.edge_count() as f64;
        let mut best_modularity = Self::calculate_modularity(graph, &communities, total_edges)?;
        let mut improved = true;
        let mut iteration = 0;

        while improved && iteration < max_iterations {
            improved = false;
            iteration += 1;

            // Phase 1: Modularity optimization
            for &vertex in &vertices {
                let current_community = communities[&vertex];
                let neighbors = graph.get_neighbors(vertex)?;

                let mut best_community = current_community;
                let mut best_gain = 0.0;

                // Try moving vertex to neighbor communities
                let neighbor_communities: HashSet<usize> = neighbors.iter()
                    .filter_map(|&n| communities.get(&n).copied())
                    .collect();

                for candidate_community in neighbor_communities {
                    if candidate_community == current_community {
                        continue;
                    }

                    // Calculate modularity gain
                    communities.insert(vertex, candidate_community);
                    let new_modularity = Self::calculate_modularity(graph, &communities, total_edges)?;
                    let gain = new_modularity - best_modularity;

                    if gain > best_gain {
                        best_gain = gain;
                        best_community = candidate_community;
                        improved = true;
                    }

                    communities.insert(vertex, current_community); // Revert
                }

                if best_community != current_community {
                    communities.insert(vertex, best_community);
                    best_modularity += best_gain;
                }
            }
        }

        // Renumber communities to be contiguous
        let unique_communities: HashSet<usize> = communities.values().copied().collect();
        let mut community_remap: HashMap<usize, usize> = HashMap::new();
        for (new_id, &old_id) in unique_communities.iter().enumerate() {
            community_remap.insert(old_id, new_id);
        }

        let final_communities: HashMap<VertexId, usize> = communities.iter()
            .map(|(&v, &c)| (v, community_remap[&c]))
            .collect();

        Ok(CommunityDetectionResult {
            communities: final_communities,
            num_communities: unique_communities.len(),
            modularity: best_modularity,
        })
    }

    fn calculate_modularity(
        graph: &PropertyGraph,
        communities: &HashMap<VertexId, usize>,
        total_edges: f64,
    ) -> Result<f64> {
        if total_edges == 0.0 {
            return Ok(0.0);
        }

        let mut modularity = 0.0;
        let m = 2.0 * total_edges; // Total degree

        for vertex1 in graph.vertices() {
            let community1 = communities.get(&vertex1.id).unwrap();
            let degree1 = vertex1.in_degree() + vertex1.out_degree();

            for vertex2 in graph.vertices() {
                if vertex1.id >= vertex2.id {
                    continue;
                }

                let community2 = communities.get(&vertex2.id).unwrap();
                if community1 != community2 {
                    continue;
                }

                let degree2 = vertex2.in_degree() + vertex2.out_degree();

                // Check if there's an edge between vertex1 and vertex2
                let edge_weight = if Self::has_edge(graph, vertex1.id, vertex2.id) {
                    1.0
                } else {
                    0.0
                };

                let expected = (degree1 * degree2) as f64 / m;
                modularity += edge_weight - expected;
            }
        }

        modularity /= m;
        Ok(modularity)
    }

    fn has_edge(graph: &PropertyGraph, v1: VertexId, v2: VertexId) -> bool {
        if let Ok(neighbors) = graph.get_outgoing_neighbors(v1) {
            neighbors.contains(&v2)
        } else {
            false
        }
    }
}

// ============================================================================
// Triangle Counting
// ============================================================================

/// Triangle counting results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangleCountResult {
    /// Total number of triangles in the graph
    pub total_triangles: usize,

    /// Number of triangles each vertex participates in
    pub vertex_triangles: HashMap<VertexId, usize>,
}

/// Triangle counting algorithm
pub struct TriangleCounting;

impl TriangleCounting {
    /// Count all triangles in the graph
    pub fn count(graph: &PropertyGraph) -> Result<TriangleCountResult> {
        let mut total_triangles = 0;
        let mut vertex_triangles: HashMap<VertexId, usize> = HashMap::new();

        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();

        for &v1 in &vertices {
            let neighbors_v1 = graph.get_outgoing_neighbors(v1)?;
            let neighbors_v1_set: HashSet<VertexId> = neighbors_v1.iter().copied().collect();

            for &v2 in &neighbors_v1 {
                if v2 <= v1 {
                    continue; // Avoid double counting
                }

                let neighbors_v2 = graph.get_outgoing_neighbors(v2)?;

                for &v3 in &neighbors_v2 {
                    if v3 <= v2 {
                        continue;
                    }

                    // Check if v3 is also a neighbor of v1
                    if neighbors_v1_set.contains(&v3) {
                        total_triangles += 1;

                        *vertex_triangles.entry(v1).or_insert(0) += 1;
                        *vertex_triangles.entry(v2).or_insert(0) += 1;
                        *vertex_triangles.entry(v3).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok(TriangleCountResult {
            total_triangles,
            vertex_triangles,
        })
    }
}

// ============================================================================
// Clustering Coefficient
// ============================================================================

/// Clustering coefficient results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringCoefficient {
    /// Global clustering coefficient
    pub global_coefficient: f64,

    /// Local clustering coefficient for each vertex
    pub local_coefficients: HashMap<VertexId, f64>,

    /// Average clustering coefficient
    pub average_coefficient: f64,
}

/// Clustering coefficient algorithm
pub struct ClusteringCoefficientAlgorithm;

impl ClusteringCoefficientAlgorithm {
    /// Compute clustering coefficients
    pub fn compute(graph: &PropertyGraph) -> Result<ClusteringCoefficient> {
        let mut local_coefficients = HashMap::new();
        let mut sum_coefficients = 0.0;
        let mut count = 0;

        for vertex in graph.vertices() {
            let coefficient = Self::local_clustering_coefficient(graph, vertex.id)?;
            local_coefficients.insert(vertex.id, coefficient);
            sum_coefficients += coefficient;
            count += 1;
        }

        let average_coefficient = if count > 0 {
            sum_coefficients / count as f64
        } else {
            0.0
        };

        // Global clustering coefficient (ratio of triangles to connected triples)
        let triangle_result = TriangleCounting::count(graph)?;
        let total_triangles = triangle_result.total_triangles as f64;

        let total_triples = Self::count_connected_triples(graph)?;
        let global_coefficient = if total_triples > 0 {
            (3.0 * total_triangles) / total_triples as f64
        } else {
            0.0
        };

        Ok(ClusteringCoefficient {
            global_coefficient,
            local_coefficients,
            average_coefficient,
        })
    }

    fn local_clustering_coefficient(graph: &PropertyGraph, vertex: VertexId) -> Result<f64> {
        let neighbors = graph.get_outgoing_neighbors(vertex)?;
        let k = neighbors.len();

        if k < 2 {
            return Ok(0.0);
        }

        let neighbors_set: HashSet<VertexId> = neighbors.iter().copied().collect();
        let mut edges_between_neighbors = 0;

        for &n1 in &neighbors {
            let n1_neighbors = graph.get_outgoing_neighbors(n1)?;
            for &n2 in &n1_neighbors {
                if neighbors_set.contains(&n2) {
                    edges_between_neighbors += 1;
                }
            }
        }

        let max_edges = k * (k - 1);
        Ok(edges_between_neighbors as f64 / max_edges as f64)
    }

    fn count_connected_triples(graph: &PropertyGraph) -> Result<usize> {
        let mut triples = 0;

        for vertex in graph.vertices() {
            let degree = vertex.out_degree();
            if degree >= 2 {
                triples += degree * (degree - 1) / 2;
            }
        }

        Ok(triples)
    }
}

// ============================================================================
// Similarity Measures
// ============================================================================

/// Jaccard similarity between two vertices
pub fn jaccard_similarity(graph: &PropertyGraph, v1: VertexId, v2: VertexId) -> Result<f64> {
    let neighbors1: HashSet<VertexId> = graph.get_outgoing_neighbors(v1)?
        .into_iter()
        .collect();

    let neighbors2: HashSet<VertexId> = graph.get_outgoing_neighbors(v2)?
        .into_iter()
        .collect();

    let intersection = neighbors1.intersection(&neighbors2).count();
    let union = neighbors1.union(&neighbors2).count();

    if union == 0 {
        Ok(0.0)
    } else {
        Ok(intersection as f64 / union as f64)
    }
}

/// Cosine similarity between two vertices (based on neighbor sets)
pub fn cosine_similarity(graph: &PropertyGraph, v1: VertexId, v2: VertexId) -> Result<f64> {
    let neighbors1: HashSet<VertexId> = graph.get_outgoing_neighbors(v1)?
        .into_iter()
        .collect();

    let neighbors2: HashSet<VertexId> = graph.get_outgoing_neighbors(v2)?
        .into_iter()
        .collect();

    let intersection = neighbors1.intersection(&neighbors2).count();
    let size1 = neighbors1.len();
    let size2 = neighbors2.len();

    if size1 == 0 || size2 == 0 {
        Ok(0.0)
    } else {
        Ok(intersection as f64 / ((size1 * size2) as f64).sqrt())
    }
}

/// Common neighbor count
pub fn common_neighbors(graph: &PropertyGraph, v1: VertexId, v2: VertexId) -> Result<usize> {
    let neighbors1: HashSet<VertexId> = graph.get_outgoing_neighbors(v1)?
        .into_iter()
        .collect();

    let neighbors2: HashSet<VertexId> = graph.get_outgoing_neighbors(v2)?
        .into_iter()
        .collect();

    Ok(neighbors1.intersection(&neighbors2).count())
}

// ============================================================================
// Influence Maximization
// ============================================================================

/// Influence propagation model
#[derive(Debug, Clone, Copy)]
pub enum InfluenceModel {
    /// Independent Cascade model
    IndependentCascade,

    /// Linear Threshold model
    LinearThreshold,
}

/// Influence maximization using greedy algorithm
pub struct InfluenceMaximization;

impl InfluenceMaximization {
    /// Find k most influential vertices using greedy algorithm
    pub fn greedy_selection(
        graph: &PropertyGraph,
        k: usize,
        model: InfluenceModel,
        monte_carlo_runs: usize,
    ) -> Result<Vec<VertexId>> {
        let mut seed_set = Vec::new();
        let vertices: Vec<VertexId> = graph.vertices().map(|v| v.id).collect();

        for _ in 0..k {
            let mut best_vertex = None;
            let mut best_influence = 0.0;

            for &candidate in &vertices {
                if seed_set.contains(&candidate) {
                    continue;
                }

                let mut test_set = seed_set.clone();
                test_set.push(candidate);

                let influence = Self::estimate_influence(
                    graph,
                    &test_set,
                    model,
                    monte_carlo_runs,
                )?;

                if influence > best_influence {
                    best_influence = influence;
                    best_vertex = Some(candidate);
                }
            }

            if let Some(vertex) = best_vertex {
                seed_set.push(vertex);
            } else {
                break;
            }
        }

        Ok(seed_set)
    }

    fn estimate_influence(
        graph: &PropertyGraph,
        seed_set: &[VertexId],
        model: InfluenceModel,
        runs: usize,
    ) -> Result<f64> {
        let mut total_influenced = 0;

        for _ in 0..runs {
            let influenced = match model {
                InfluenceModel::IndependentCascade => {
                    Self::independent_cascade_simulation(graph, seed_set)?
                }
                InfluenceModel::LinearThreshold => {
                    Self::linear_threshold_simulation(graph, seed_set)?
                }
            };

            total_influenced += influenced;
        }

        Ok(total_influenced as f64 / runs as f64)
    }

    fn independent_cascade_simulation(
        graph: &PropertyGraph,
        seed_set: &[VertexId],
    ) -> Result<usize> {
        let mut influenced = HashSet::new();
        let mut queue = VecDeque::new();

        for &seed in seed_set {
            influenced.insert(seed);
            queue.push_back(seed);
        }

        let propagation_prob = 0.1; // Simplified: constant probability

        while let Some(current) = queue.pop_front() {
            let neighbors = graph.get_outgoing_neighbors(current)?;

            for neighbor in neighbors {
                if !influenced.contains(&neighbor) {
                    // Simulate propagation with probability
                    if rand::random::<f64>() < propagation_prob {
                        influenced.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        Ok(influenced.len())
    }

    fn linear_threshold_simulation(
        graph: &PropertyGraph,
        seed_set: &[VertexId],
    ) -> Result<usize> {
        let mut influenced = HashSet::new();
        let mut thresholds: HashMap<VertexId, f64> = HashMap::new();

        // Initialize random thresholds
        for vertex in graph.vertices() {
            thresholds.insert(vertex.id, rand::random::<f64>());
        }

        for &seed in seed_set {
            influenced.insert(seed);
        }

        let mut changed = true;
        while changed {
            changed = false;

            for vertex in graph.vertices() {
                if influenced.contains(&vertex.id) {
                    continue;
                }

                let neighbors = graph.get_incoming_neighbors(vertex.id)?;
                let influenced_neighbors = neighbors.iter()
                    .filter(|n| influenced.contains(n))
                    .count();

                let degree = vertex.in_degree();
                if degree > 0 {
                    let influence_fraction = influenced_neighbors as f64 / degree as f64;
                    if influence_fraction >= thresholds[&vertex.id] {
                        influenced.insert(vertex.id);
                        changed = true;
                    }
                }
            }
        }

        Ok(influenced.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::property_graph::{Properties, EdgeDirection};

    #[test]
    fn test_pagerank() {
        let mut graph = PropertyGraph::new();
        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v2, v3, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v3, v1, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let config = PageRankConfig::default();
        let result = PageRank::compute(&graph, &config).unwrap();

        assert!(result.converged);
        assert_eq!(result.scores.len(), 3);

        // All vertices should have similar scores in this symmetric graph
        let scores: Vec<f64> = result.scores.values().copied().collect();
        let avg = scores.iter().sum::<f64>() / scores.len() as f64;
        assert!((avg - 0.333).abs() < 0.1);
    }

    #[test]
    fn test_connected_components() {
        let mut graph = PropertyGraph::new();
        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v4 = graph.add_vertex(vec![], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "link".to_string(), Properties::new(), EdgeDirection::Undirected).unwrap();
        graph.add_edge(v3, v4, "link".to_string(), Properties::new(), EdgeDirection::Undirected).unwrap();

        let result = ConnectedComponentsAlgorithm::compute(&graph).unwrap();

        assert_eq!(result.num_components, 2);
        assert!(!ConnectedComponentsAlgorithm::same_component(&result, v1, v3));
        assert!(ConnectedComponentsAlgorithm::same_component(&result, v1, v2));
    }
}
