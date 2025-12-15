// # Property Graph Implementation
//
// Provides a comprehensive property graph data structure with support for:
// - Vertices and edges with properties
// - Multi-graph (multiple edges between nodes)
// - Hypergraph extensions
// - Graph partitioning for distributed storage
// - Efficient ID generation and management

use crate::common::Value;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

// ============================================================================
// Type Aliases and Constants
// ============================================================================

// Unique identifier for vertices in the graph
pub type VertexId = u64;

// Unique identifier for edges in the graph
pub type EdgeId = u64;

// Identifier for graph partitions
pub type PartitionId = u32;

// Label type for vertices and edges
pub type Label = String;

// Property key type
pub type PropertyKey = String;

// Maximum number of partitions supported
pub const MAX_PARTITIONS: u32 = 1024;

// Default partition size (number of vertices)
pub const DEFAULT_PARTITION_SIZE: usize = 100_000;

// ============================================================================
// Property Storage
// ============================================================================

// Properties associated with vertices and edges (key-value pairs)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Properties {
    data: HashMap<PropertyKey, Value>,
}

impl Properties {
    // Create empty properties
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    // Create properties from a hashmap
    pub fn from_map(data: HashMap<PropertyKey, Value>) -> Self {
        Self { data }
    }

    // Set a property value
    pub fn set(&mut self, key: PropertyKey, value: Value) {
        self.data.insert(key, value);
    }

    // Get a property value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    // Remove a property
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    // Check if property exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    // Get all property keys
    pub fn keys(&self) -> Vec<&String> {
        self.data.keys().collect()
    }

    // Get all property values
    pub fn values(&self) -> Vec<&Value> {
        self.data.values().collect()
    }

    // Get number of properties
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // Check if properties are empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // Iterate over properties
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.data.iter()
    }

    // Merge properties from another set
    pub fn merge(&mut self, other: Properties) {
        self.data.extend(other.data);
    }
}

impl Default for Properties {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vertex Structure
// ============================================================================

// Represents a vertex (node) in the property graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    // Unique vertex identifier
    pub id: VertexId,

    // Vertex labels (can have multiple labels)
    pub labels: Vec<Label>,

    // Vertex properties
    pub properties: Properties,

    // Outgoing edge IDs
    pub outgoing_edges: HashSet<EdgeId>,

    // Incoming edge IDs
    pub incoming_edges: HashSet<EdgeId>,

    // Partition this vertex belongs to
    pub partition_id: PartitionId,

    // Degree cache for quick access
    degree_cache: Option<VertexDegree>,
}

// Vertex degree information
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VertexDegree {
    pub in_degree: usize,
    pub out_degree: usize,
    pub total_degree: usize,
}

impl Vertex {
    // Create a new vertex with given ID and labels
    pub fn new(id: VertexId, labels: Vec<Label>) -> Self {
        Self {
            id,
            labels,
            properties: Properties::new(),
            outgoing_edges: HashSet::new(),
            incoming_edges: HashSet::new(),
            partition_id: 0,
            degree_cache: None,
        }
    }

    // Create a new vertex with properties
    pub fn with_properties(id: VertexId, labels: Vec<Label>, properties: Properties) -> Self {
        Self {
            id,
            labels,
            properties,
            outgoing_edges: HashSet::new(),
            incoming_edges: HashSet::new(),
            partition_id: 0,
            degree_cache: None,
        }
    }

    // Add a label to the vertex
    pub fn add_label(&mut self, label: Label) {
        if !self.labels.contains(&label) {
            self.labels.push(label);
        }
    }

    // Remove a label from the vertex
    pub fn remove_label(&mut self, label: &str) -> bool {
        if let Some(pos) = self.labels.iter().position(|l| l == label) {
            self.labels.remove(pos);
            true
        } else {
            false
        }
    }

    // Check if vertex has a specific label
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l == label)
    }

    // Add an outgoing edge
    pub(crate) fn add_outgoing_edge(&mut self, edge_id: EdgeId) {
        self.outgoing_edges.insert(edge_id);
        self.degree_cache = None; // Invalidate cache
    }

    // Add an incoming edge
    pub(crate) fn add_incoming_edge(&mut self, edge_id: EdgeId) {
        self.incoming_edges.insert(edge_id);
        self.degree_cache = None; // Invalidate cache
    }

    // Remove an outgoing edge
    pub(crate) fn remove_outgoing_edge(&mut self, edge_id: EdgeId) -> bool {
        let removed = self.outgoing_edges.remove(&edge_id);
        if removed {
            self.degree_cache = None; // Invalidate cache
        }
        removed
    }

    // Remove an incoming edge
    pub(crate) fn remove_incoming_edge(&mut self, edge_id: EdgeId) -> bool {
        let removed = self.incoming_edges.remove(&edge_id);
        if removed {
            self.degree_cache = None; // Invalidate cache
        }
        removed
    }

    // Get degree information
    pub fn degree(&mut self) -> VertexDegree {
        if let Some(degree) = self.degree_cache {
            degree
        } else {
            let degree = VertexDegree {
                in_degree: self.incoming_edges.len(),
                out_degree: self.outgoing_edges.len(),
                total_degree: self.incoming_edges.len() + self.outgoing_edges.len(),
            };
            self.degree_cache = Some(degree);
            degree
        }
    }

    // Get in-degree (number of incoming edges)
    pub fn in_degree(&self) -> usize {
        self.incoming_edges.len()
    }

    // Get out-degree (number of outgoing edges)
    pub fn out_degree(&self) -> usize {
        self.outgoing_edges.len()
    }

    // Check if vertex is isolated (no edges)
    pub fn is_isolated(&self) -> bool {
        self.incoming_edges.is_empty() && self.outgoing_edges.is_empty()
    }
}

// ============================================================================
// Edge Structure
// ============================================================================

// Edge direction enumeration
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
pub enum EdgeDirection {
    // Directed edge from source to target
    Directed,

    // Undirected edge (bidirectional)
    Undirected,
}

// Represents an edge in the property graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    // Unique edge identifier
    pub id: EdgeId,

    // Source vertex ID
    pub source: VertexId,

    // Target vertex ID
    pub target: VertexId,

    // Edge label (relationship type)
    pub label: Label,

    // Edge properties
    pub properties: Properties,

    // Edge direction
    pub direction: EdgeDirection,

    // Weight (if applicable)
    pub weight: Option<f64>,

    // Timestamp for temporal graphs
    pub timestamp: Option<i64>,
}

impl Edge {
    // Create a new directed edge
    pub fn new(id: EdgeId, source: VertexId, target: VertexId, label: Label) -> Self {
        Self {
            id,
            source,
            target,
            label,
            properties: Properties::new(),
            direction: EdgeDirection::Directed,
            weight: None,
            timestamp: None,
        }
    }

    // Create a new edge with properties
    pub fn with_properties(
        id: EdgeId,
        source: VertexId,
        target: VertexId,
        label: Label,
        properties: Properties,
    ) -> Self {
        Self {
            id,
            source,
            target,
            label,
            properties,
            direction: EdgeDirection::Directed,
            weight: None,
            timestamp: None,
        }
    }

    // Create an undirected edge
    pub fn undirected(id: EdgeId, source: VertexId, target: VertexId, label: Label) -> Self {
        Self {
            id,
            source,
            target,
            label,
            properties: Properties::new(),
            direction: EdgeDirection::Undirected,
            weight: None,
            timestamp: None,
        }
    }

    // Set edge weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    // Set edge timestamp
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    // Check if edge is directed
    pub fn is_directed(&self) -> bool {
        self.direction == EdgeDirection::Directed
    }

    // Check if edge is undirected
    pub fn is_undirected(&self) -> bool {
        self.direction == EdgeDirection::Undirected
    }

    // Check if edge is a self-loop
    pub fn is_self_loop(&self) -> bool {
        self.source == self.target
    }

    // Get the other vertex in the edge (given one endpoint)
    pub fn other_vertex(&self, vertex_id: VertexId) -> Option<VertexId> {
        if self.source == vertex_id {
            Some(self.target)
        } else if self.target == vertex_id {
            Some(self.source)
        } else {
            None
        }
    }
}

// ============================================================================
// Hypergraph Support
// ============================================================================

// Hyperedge connects multiple vertices (generalization of edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperEdge {
    // Unique hyperedge identifier
    pub id: EdgeId,

    // Set of vertices in this hyperedge
    pub vertices: HashSet<VertexId>,

    // Hyperedge label
    pub label: Label,

    // Hyperedge properties
    pub properties: Properties,

    // Cardinality (number of vertices)
    pub cardinality: usize,
}

impl HyperEdge {
    // Create a new hyperedge
    pub fn new(id: EdgeId, vertices: HashSet<VertexId>, label: Label) -> Self {
        let cardinality = vertices.len();
        Self {
            id,
            vertices,
            label,
            properties: Properties::new(),
            cardinality,
        }
    }

    // Add a vertex to the hyperedge
    pub fn add_vertex(&mut self, vertex_id: VertexId) {
        self.vertices.insert(vertex_id);
        self.cardinality = self.vertices.len();
    }

    // Remove a vertex from the hyperedge
    pub fn remove_vertex(&mut self, vertex_id: VertexId) -> bool {
        let removed = self.vertices.remove(&vertex_id);
        if removed {
            self.cardinality = self.vertices.len();
        }
        removed
    }

    // Check if vertex is in hyperedge
    pub fn contains_vertex(&self, vertex_id: VertexId) -> bool {
        self.vertices.contains(&vertex_id)
    }
}

// ============================================================================
// Graph Partitioning
// ============================================================================

// Graph partition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPartition {
    // Partition identifier
    pub id: PartitionId,

    // Vertices in this partition
    pub vertices: HashSet<VertexId>,

    // Edges in this partition
    pub edges: HashSet<EdgeId>,

    // Cut edges (edges connecting to other partitions)
    pub cut_edges: HashSet<EdgeId>,

    // Partition size in bytes (estimated)
    pub size_bytes: usize,
}

impl GraphPartition {
    // Create a new partition
    pub fn new(id: PartitionId) -> Self {
        Self {
            id,
            vertices: HashSet::new(),
            edges: HashSet::new(),
            cut_edges: HashSet::new(),
            size_bytes: 0,
        }
    }

    // Add a vertex to the partition
    pub fn add_vertex(&mut self, vertex_id: VertexId) {
        self.vertices.insert(vertex_id);
        self.size_bytes += size_of::<VertexId>();
    }

    // Add an edge to the partition
    pub fn add_edge(&mut self, edge_id: EdgeId, is_cut: bool) {
        self.edges.insert(edge_id);
        if is_cut {
            self.cut_edges.insert(edge_id);
        }
        self.size_bytes += size_of::<EdgeId>();
    }

    // Get partition load (number of vertices)
    pub fn load(&self) -> usize {
        self.vertices.len()
    }

    // Get cut ratio (proportion of cut edges)
    pub fn cut_ratio(&self) -> f64 {
        if self.edges.is_empty() {
            0.0
        } else {
            self.cut_edges.len() as f64 / self.edges.len() as f64
        }
    }
}

// Partitioning strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitioningStrategy {
    // Hash-based partitioning
    Hash,

    // Range-based partitioning
    Range,

    // Edge-cut minimization (METIS-like)
    EdgeCut,

    // Vertex-cut minimization
    VertexCut,
}

// Graph partitioner
#[derive(Clone)]
pub struct GraphPartitioner {
    strategy: PartitioningStrategy,
    num_partitions: u32,
    partitions: Vec<GraphPartition>,
}

impl GraphPartitioner {
    // Create a new partitioner
    pub fn new(strategy: PartitioningStrategy, num_partitions: u32) -> Self {
        let mut partitions = Vec::with_capacity(num_partitions as usize);
        for i in 0..num_partitions {
            partitions.push(GraphPartition::new(i));
        }

        Self {
            strategy,
            num_partitions,
            partitions,
        }
    }

    // Assign a vertex to a partition
    pub fn assign_vertex(&mut self, vertex_id: VertexId) -> PartitionId {
        let partition_id = match self.strategy {
            PartitioningStrategy::Hash => (vertex_id % self.num_partitions as u64) as PartitionId,
            PartitioningStrategy::Range => {
                // Simple range partitioning
                ((vertex_id / DEFAULT_PARTITION_SIZE as u64) % self.num_partitions as u64)
                    as PartitionId
            }
            _ => {
                // Default to hash for other strategies (would need graph structure for advanced partitioning)
                (vertex_id % self.num_partitions as u64) as PartitionId
            }
        };

        self.partitions[partition_id as usize].add_vertex(vertex_id);
        partition_id
    }

    // Get partition for a vertex
    pub fn get_partition(&self, vertex_id: VertexId) -> PartitionId {
        match self.strategy {
            PartitioningStrategy::Hash => (vertex_id % self.num_partitions as u64) as PartitionId,
            PartitioningStrategy::Range => {
                ((vertex_id / DEFAULT_PARTITION_SIZE as u64) % self.num_partitions as u64)
                    as PartitionId
            }
            _ => (vertex_id % self.num_partitions as u64) as PartitionId,
        }
    }

    // Get partition statistics
    pub fn get_partition_stats(&self, partition_id: PartitionId) -> Option<&GraphPartition> {
        self.partitions.get(partition_id as usize)
    }

    // Balance partitions (simple greedy rebalancing)
    pub fn rebalance(&mut self) -> Result<()> {
        // Calculate average load
        let total_vertices: usize = self.partitions.iter().map(|p| p.load()).sum();
        let avg_load = total_vertices / self.num_partitions as usize;

        // Find overloaded and underloaded partitions
        let mut overloaded: Vec<PartitionId> = Vec::new();
        let mut underloaded: Vec<PartitionId> = Vec::new();

        for partition in &self.partitions {
            if partition.load() > avg_load * 11 / 10 {
                // 10% tolerance
                overloaded.push(partition.id);
            } else if partition.load() < avg_load * 9 / 10 {
                underloaded.push(partition.id);
            }
        }

        // Simple rebalancing would require moving vertices
        // This is a placeholder for the actual rebalancing logic

        Ok(())
    }
}

// ============================================================================
// Graph Statistics
// ============================================================================

// Graph statistics and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    // Total number of vertices
    pub num_vertices: u64,

    // Total number of edges
    pub num_edges: u64,

    // Number of hyperedges
    pub num_hyperedges: u64,

    // Graph density
    pub density: f64,

    // Average degree
    pub avg_degree: f64,

    // Maximum degree
    pub max_degree: usize,

    // Minimum degree
    pub min_degree: usize,

    // Number of connected components
    pub num_components: Option<usize>,

    // Graph diameter
    pub diameter: Option<usize>,

    // Clustering coefficient
    pub clustering_coefficient: Option<f64>,

    // Memory usage in bytes
    pub memory_bytes: usize,

    // Last updated timestamp
    pub last_updated: i64,
}

impl GraphStats {
    // Create empty statistics
    pub fn new() -> Self {
        Self {
            num_vertices: 0,
            num_edges: 0,
            num_hyperedges: 0,
            density: 0.0,
            avg_degree: 0.0,
            max_degree: 0,
            min_degree: 0,
            num_components: None,
            diameter: None,
            clustering_coefficient: None,
            memory_bytes: 0,
            last_updated: 0,
        }
    }

    // Update basic statistics
    pub fn update_basic(&mut self, num_vertices: u64, num_edges: u64) {
        self.num_vertices = num_vertices;
        self.num_edges = num_edges;

        // Calculate density: edges / (vertices * (vertices - 1) / 2)
        if num_vertices > 1 {
            let max_edges = num_vertices * (num_vertices - 1) / 2;
            self.density = num_edges as f64 / max_edges as f64;
            self.avg_degree = (2.0 * num_edges as f64) / num_vertices as f64;
        }

        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }

    // Update degree statistics
    pub fn update_degree_stats(&mut self, degrees: &[usize]) {
        if degrees.is_empty() {
            return;
        }

        self.max_degree = *degrees.iter().max().unwrap();
        self.min_degree = *degrees.iter().min().unwrap();

        let sum: usize = degrees.iter().sum();
        self.avg_degree = sum as f64 / degrees.len() as f64;
    }
}

impl Default for GraphStats {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Property Graph
// ============================================================================

// Main property graph structure
pub struct PropertyGraph {
    // All vertices in the graph
    vertices: HashMap<VertexId, Vertex>,

    // All edges in the graph
    edges: HashMap<EdgeId, Edge>,

    // Hyperedges (for hypergraph support)
    hyperedges: HashMap<EdgeId, HyperEdge>,

    // Vertex ID generator
    next_vertex_id: AtomicU64,

    // Edge ID generator
    next_edge_id: AtomicU64,

    // Graph partitioner
    partitioner: Option<GraphPartitioner>,

    // Graph statistics
    stats: Arc<RwLock<GraphStats>>,

    // Index: label -> vertex IDs
    vertex_label_index: HashMap<Label, HashSet<VertexId>>,

    // Index: label -> edge IDs
    edge_label_index: HashMap<Label, HashSet<EdgeId>>,

    // Index: property key -> vertex IDs
    vertex_property_index: HashMap<PropertyKey, HashSet<VertexId>>,
}

impl PropertyGraph {
    // Create a new empty property graph
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: HashMap::new(),
            hyperedges: HashMap::new(),
            next_vertex_id: AtomicU64::new(1),
            next_edge_id: AtomicU64::new(1),
            partitioner: None,
            stats: Arc::new(RwLock::new(GraphStats::new())),
            vertex_label_index: HashMap::new(),
            edge_label_index: HashMap::new(),
            vertex_property_index: HashMap::new(),
        }
    }

    // Create a graph with partitioning
    pub fn with_partitioning(strategy: PartitioningStrategy, num_partitions: u32) -> Self {
        let mut graph = Self::new();
        graph.partitioner = Some(GraphPartitioner::new(strategy, num_partitions));
        graph
    }

    // Generate next vertex ID
    fn generate_vertex_id(&self) -> VertexId {
        self.next_vertex_id.fetch_add(1, Ordering::SeqCst)
    }

    // Generate next edge ID
    fn generate_edge_id(&self) -> EdgeId {
        self.next_edge_id.fetch_add(1, Ordering::SeqCst)
    }

    // Add a vertex to the graph
    pub fn add_vertex(&mut self, labels: Vec<Label>, properties: Properties) -> Result<VertexId> {
        let id = self.generate_vertex_id();
        let mut vertex = Vertex::with_properties(id, labels.clone(), properties);

        // Assign partition
        if let Some(ref mut partitioner) = self.partitioner {
            vertex.partition_id = partitioner.assign_vertex(id);
        }

        // Update label index
        for label in &labels {
            self.vertex_label_index
                .entry(label.clone())
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        // Update property index
        for key in vertex.properties.keys() {
            self.vertex_property_index
                .entry(key.clone())
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        self.vertices.insert(id, vertex);

        // Update statistics
        self.update_stats();

        Ok(id)
    }

    // Get a vertex by ID
    pub fn get_vertex(&self, id: VertexId) -> Option<&Vertex> {
        self.vertices.get(&id)
    }

    // Get a mutable vertex by ID
    pub fn get_vertex_mut(&mut self, id: VertexId) -> Option<&mut Vertex> {
        self.vertices.get_mut(&id)
    }

    // Remove a vertex and all its edges
    pub fn remove_vertex(&mut self, id: VertexId) -> Result<()> {
        if let Some(vertex) = self.vertices.remove(&id) {
            // Remove all connected edges
            let edge_ids: Vec<EdgeId> = vertex
                .outgoing_edges
                .iter()
                .chain(vertex.incoming_edges.iter())
                .copied()
                .collect();

            for edge_id in edge_ids {
                self.remove_edge(edge_id)?;
            }

            // Update label index
            for label in &vertex.labels {
                if let Some(set) = self.vertex_label_index.get_mut(label) {
                    set.remove(&id);
                }
            }

            // Update property index
            for key in vertex.properties.keys() {
                if let Some(set) = self.vertex_property_index.get_mut(key) {
                    set.remove(&id);
                }
            }

            self.update_stats();
            Ok(())
        } else {
            Err(DbError::Internal(format!("Vertex {} not found", id)))
        }
    }

    // Add an edge between two vertices
    pub fn add_edge(
        &mut self,
        source: VertexId,
        target: VertexId,
        label: Label,
        properties: Properties,
        direction: EdgeDirection,
    ) -> Result<EdgeId> {
        // Verify vertices exist
        if !self.vertices.contains_key(&source) {
            return Err(DbError::Internal(format!(
                "Source vertex {} not found",
                source
            )));
        }
        if !self.vertices.contains_key(&target) {
            return Err(DbError::Internal(format!(
                "Target vertex {} not found",
                target
            )));
        }

        let id = self.generate_edge_id();
        let mut edge = Edge::with_properties(id, source, target, label.clone(), properties);
        edge.direction = direction;

        // Update vertex edge lists
        if let Some(v) = self.vertices.get_mut(&source) {
            v.add_outgoing_edge(id);
        }

        if let Some(v) = self.vertices.get_mut(&target) {
            v.add_incoming_edge(id);
        }

        // For undirected edges, also add reverse references
        if direction == EdgeDirection::Undirected {
            if let Some(v) = self.vertices.get_mut(&source) {
                v.add_incoming_edge(id);
            }
            if let Some(v) = self.vertices.get_mut(&target) {
                v.add_outgoing_edge(id);
            }
        }

        // Update edge label index
        self.edge_label_index
            .entry(label)
            .or_insert_with(HashSet::new)
            .insert(id);

        self.edges.insert(id, edge);
        self.update_stats();

        Ok(id)
    }

    // Get an edge by ID
    pub fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    // Remove an edge
    pub fn remove_edge(&mut self, id: EdgeId) -> Result<()> {
        if let Some(edge) = self.edges.remove(&id) {
            // Update vertex edge lists
            if let Some(v) = self.vertices.get_mut(&edge.source) {
                v.remove_outgoing_edge(id);
            }

            if let Some(v) = self.vertices.get_mut(&edge.target) {
                v.remove_incoming_edge(id);
            }

            // For undirected edges, also remove reverse references
            if edge.direction == EdgeDirection::Undirected {
                if let Some(v) = self.vertices.get_mut(&edge.source) {
                    v.remove_incoming_edge(id);
                }
                if let Some(v) = self.vertices.get_mut(&edge.target) {
                    v.remove_outgoing_edge(id);
                }
            }

            // Update edge label index
            if let Some(set) = self.edge_label_index.get_mut(&edge.label) {
                set.remove(&id);
            }

            self.update_stats();
            Ok(())
        } else {
            Err(DbError::Internal(format!("Edge {} not found", id)))
        }
    }

    // Add a hyperedge
    pub fn add_hyperedge(
        &mut self,
        vertices: HashSet<VertexId>,
        label: Label,
        properties: Properties,
    ) -> Result<EdgeId> {
        // Verify all vertices exist
        for &vertex_id in &vertices {
            if !self.vertices.contains_key(&vertex_id) {
                return Err(DbError::Internal(format!("Vertex {} not found", vertex_id)));
            }
        }

        let id = self.generate_edge_id();
        let mut hyperedge = HyperEdge::new(id, vertices, label);
        hyperedge.properties = properties;

        self.hyperedges.insert(id, hyperedge);
        self.update_stats();

        Ok(id)
    }

    // Get vertices by label
    pub fn get_vertices_by_label(&self, label: &str) -> Vec<&Vertex> {
        if let Some(vertex_ids) = self.vertex_label_index.get(label) {
            vertex_ids
                .iter()
                .filter_map(|id| self.vertices.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    // Get edges by label
    pub fn get_edges_by_label(&self, label: &str) -> Vec<&Edge> {
        if let Some(edge_ids) = self.edge_label_index.get(label) {
            edge_ids
                .iter()
                .filter_map(|id| self.edges.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    // Get neighbors of a vertex
    pub fn get_neighbors(&self, vertex_id: VertexId) -> Result<Vec<VertexId>> {
        if let Some(vertex) = self.vertices.get(&vertex_id) {
            let mut neighbors = HashSet::new();

            // Add targets of outgoing edges
            for &edge_id in &vertex.outgoing_edges {
                if let Some(edge) = self.edges.get(&edge_id) {
                    neighbors.insert(edge.target);
                }
            }

            // Add sources of incoming edges
            for &edge_id in &vertex.incoming_edges {
                if let Some(edge) = self.edges.get(&edge_id) {
                    neighbors.insert(edge.source);
                }
            }

            Ok(neighbors.into_iter().collect())
        } else {
            Err(DbError::Internal(format!("Vertex {} not found", vertex_id)))
        }
    }

    // Get outgoing neighbors
    pub fn get_outgoing_neighbors(&self, vertex_id: VertexId) -> Result<Vec<VertexId>> {
        if let Some(vertex) = self.vertices.get(&vertex_id) {
            let neighbors: Vec<VertexId> = vertex
                .outgoing_edges
                .iter()
                .filter_map(|&edge_id| self.edges.get(&edge_id))
                .map(|edge| edge.target)
                .collect();
            Ok(neighbors)
        } else {
            Err(DbError::Internal(format!("Vertex {} not found", vertex_id)))
        }
    }

    // Get incoming neighbors
    pub fn get_incoming_neighbors(&self, vertex_id: VertexId) -> Result<Vec<VertexId>> {
        if let Some(vertex) = self.vertices.get(&vertex_id) {
            let neighbors: Vec<VertexId> = vertex
                .incoming_edges
                .iter()
                .filter_map(|&edge_id| self.edges.get(&edge_id))
                .map(|edge| edge.source)
                .collect();
            Ok(neighbors)
        } else {
            Err(DbError::Internal(format!("Vertex {} not found", vertex_id)))
        }
    }

    // Get all vertices
    pub fn vertices(&self) -> impl Iterator<Item = &Vertex> {
        self.vertices.values()
    }

    // Get all edges
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values()
    }

    // Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    // Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    // Update graph statistics
    fn update_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.update_basic(self.vertices.len() as u64, self.edges.len() as u64);
            stats.num_hyperedges = self.hyperedges.len() as u64;

            let degrees: Vec<usize> = self
                .vertices
                .values()
                .map(|v| v.in_degree() + v.out_degree())
                .collect();

            if !degrees.is_empty() {
                stats.update_degree_stats(&degrees);
            }

            // Estimate memory usage
            stats.memory_bytes = self.vertices.len() * size_of::<Vertex>()
                + self.edges.len() * size_of::<Edge>()
                + self.hyperedges.len() * size_of::<HyperEdge>();
        }
    }

    // Get graph statistics
    pub fn get_stats(&self) -> GraphStats {
        self.stats.read().unwrap().clone()
    }

    // Clear the entire graph
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.hyperedges.clear();
        self.vertex_label_index.clear();
        self.edge_label_index.clear();
        self.vertex_property_index.clear();
        self.update_stats();
    }
}

impl Default for PropertyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for PropertyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyGraph")
            .field("vertices", &self.vertices)
            .field("edges", &self.edges)
            .field("hyperedges", &self.hyperedges)
            .field(
                "next_vertex_id",
                &self.next_vertex_id.load(Ordering::SeqCst),
            )
            .field("next_edge_id", &self.next_edge_id.load(Ordering::SeqCst))
            .field("stats", &self.stats)
            .finish()
    }
}

impl Clone for PropertyGraph {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            hyperedges: self.hyperedges.clone(),
            next_vertex_id: AtomicU64::new(self.next_vertex_id.load(Ordering::SeqCst)),
            next_edge_id: AtomicU64::new(self.next_edge_id.load(Ordering::SeqCst)),
            partitioner: self.partitioner.clone(),
            stats: Arc::new(RwLock::new(self.stats.read().unwrap().clone())),
            vertex_label_index: self.vertex_label_index.clone(),
            edge_label_index: self.edge_label_index.clone(),
            vertex_property_index: self.vertex_property_index.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_graph_basic() {
        let mut graph = PropertyGraph::new();

        // Add vertices
        let v1 = graph
            .add_vertex(vec!["Person".to_string()], Properties::new())
            .unwrap();
        let v2 = graph
            .add_vertex(vec!["Person".to_string()], Properties::new())
            .unwrap();

        // Add edge
        let _e1 = graph
            .add_edge(
                v1,
                v2,
                "KNOWS".to_string(),
                Properties::new(),
                EdgeDirection::Directed,
            )
            .unwrap();

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // Check neighbors
        let neighbors = graph.get_outgoing_neighbors(v1).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], v2);
    }

    #[test]
    fn test_vertex_properties() {
        let mut props = Properties::new();
        props.set("name".to_string(), Value::String("Alice".to_string()));
        props.set("age".to_string(), Value::Integer(30));

        assert_eq!(props.len(), 2);
        assert!(props.contains_key("name"));

        if let Some(Value::String(name)) = props.get("name") {
            assert_eq!(name, "Alice");
        } else {
            panic!("Expected string value");
        }
    }
}
