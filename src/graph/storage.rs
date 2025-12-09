// # Graph Storage Engine
//
// Efficient storage formats for property graphs:
// - Adjacency list storage
// - Compressed Sparse Row (CSR) format
// - Edge-centric storage for streaming
// - Vertex-centric storage for OLTP
// - Index structures for graph lookups
// - Graph compression techniques

use std::collections::HashSet;
use std::collections::{HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Read};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};
use super::property_graph::{
    PropertyGraph, VertexId, EdgeId, Vertex, Properties,
    EdgeDirection,
};

// ============================================================================
// Storage Format Types
// ============================================================================

/// Storage format for graph data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageFormat {
    /// Adjacency list (good for general purpose)
    AdjacencyList,

    /// Compressed Sparse Row (good for analytics)
    CSR,

    /// Edge list (good for streaming)
    EdgeList,

    /// Hybrid format
    Hybrid,
}

// ============================================================================
// Adjacency List Storage
// ============================================================================

/// Adjacency list representation of the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacencyList {
    /// Outgoing adjacency lists: vertex -> list of (neighbor, edge_id)
    pub outgoing: HashMap<VertexId, Vec<(VertexId, EdgeId)>>,

    /// Incoming adjacency lists: vertex -> list of (neighbor, edge_id)
    pub incoming: HashMap<VertexId, Vec<(VertexId, EdgeId)>>,

    /// Vertex data
    pub vertices: HashMap<VertexId, VertexData>,

    /// Edge data
    pub edges: HashMap<EdgeId, EdgeData>,
}

/// Vertex data for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    pub id: VertexId,
    pub labels: Vec<String>,
    pub properties: Properties,
}

/// Edge data for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: EdgeId,
    pub source: VertexId,
    pub target: VertexId,
    pub label: String,
    pub properties: Properties,
    pub direction: EdgeDirection,
    pub weight: Option<f64>,
}

impl AdjacencyList {
    /// Create a new empty adjacency list
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            vertices: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    /// Build from a property graph
    pub fn from_graph(graph: &PropertyGraph) -> Self {
        let mut adj_list = Self::new();

        // Add vertices
        for vertex in graph.vertices() {
            adj_list.vertices.insert(
                vertex.id,
                VertexData {
                    id: vertex.id,
                    labels: vertex.labels.clone(),
                    properties: vertex.properties.clone(),
                },
            );

            adj_list.outgoing.insert(vertex.id, Vec::new());
            adj_list.incoming.insert(vertex.id, Vec::new());
        }

        // Add edges
        for edge in graph.edges() {
            adj_list.edges.insert(
                edge.id,
                EdgeData {
                    id: edge.id,
                    source: edge.source,
                    target: edge.target,
                    label: edge.label.clone(),
                    properties: edge.properties.clone(),
                    direction: edge.direction,
                    weight: edge.weight,
                },
            );

            // Update adjacency lists
            adj_list.outgoing
                .entry(edge.source)
                .or_insert_with(Vec::new)
                .push((edge.target, edge.id));

            adj_list.incoming
                .entry(edge.target)
                .or_insert_with(Vec::new)
                .push((edge.source, edge.id));
        }

        adj_list
    }

    /// Get outgoing neighbors of a vertex
    pub fn get_outgoing(&self, vertex: VertexId) -> Option<&Vec<(VertexId, EdgeId)>> {
        self.outgoing.get(&vertex)
    }

    /// Get incoming neighbors of a vertex
    pub fn get_incoming(&self, vertex: VertexId) -> Option<&Vec<(VertexId, EdgeId)>> {
        self.incoming.get(&vertex)
    }

    /// Get degree of a vertex
    pub fn degree(&self, vertex: VertexId) -> (usize, usize) {
        let out_degree = self.outgoing.get(&vertex).map_or(0, |v| v.len());
        let in_degree = self.incoming.get(&vertex).map_or(0, |v| v.len());
        (in_degree, out_degree)
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| DbError::Internal(format!("Deserialization error: {}", e)))
    }
}

impl Default for AdjacencyList {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Compressed Sparse Row (CSR) Format
// ============================================================================

/// CSR (Compressed Sparse Row) format for efficient graph analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSRGraph {
    /// Number of vertices
    pub num_vertices: usize,

    /// Number of edges
    pub num_edges: usize,

    /// Row offsets: vertex i's neighbors are in neighbors[offsets[i]..offsets[i+1]]
    pub offsets: Vec<usize>,

    /// Column indices (neighbor vertex IDs)
    pub neighbors: Vec<VertexId>,

    /// Edge IDs corresponding to neighbors
    pub edge_ids: Vec<EdgeId>,

    /// Vertex ID mapping: index -> actual vertex ID
    pub vertex_map: Vec<VertexId>,

    /// Reverse mapping: vertex ID -> index
    pub vertex_index: HashMap<VertexId, usize>,

    /// Vertex properties
    pub vertex_properties: Vec<Properties>,

    /// Edge properties
    pub edge_properties: HashMap<EdgeId, Properties>,
}

impl CSRGraph {
    /// Build CSR representation from adjacency list
    pub fn from_adjacency_list(adj_list: &AdjacencyList) -> Self {
        let vertices: Vec<VertexId> = adj_list.vertices.keys().copied().collect());
        let num_vertices = vertices.len();

        // Create vertex mapping
        let mut vertex_map = vertices.clone();
        vertex_map.sort();

        let mut vertex_index = HashMap::new();
        for (idx, &vertex_id) in vertex_map.iter().enumerate() {
            vertex_index.insert(vertex_id, idx);
        }

        // Build CSR arrays
        let mut offsets = Vec::with_capacity(num_vertices + 1);
        let mut neighbors = Vec::new();
        let mut edge_ids = Vec::new();
        let mut vertex_properties = Vec::with_capacity(num_vertices);

        offsets.push(0);

        for &vertex_id in &vertex_map {
            // Get outgoing neighbors
            if let Some(out_neighbors) = adj_list.outgoing.get(&vertex_id) {
                for &(neighbor, edge_id) in out_neighbors {
                    neighbors.push(neighbor);
                    edge_ids.push(edge_id);
                }
            }

            offsets.push(neighbors.len());

            // Store vertex properties
            if let Some(vertex_data) = adj_list.vertices.get(&vertex_id) {
                vertex_properties.push(vertex_data.properties.clone());
            } else {
                vertex_properties.push(Properties::new());
            }
        }

        // Extract edge properties
        let mut edge_properties = HashMap::new();
        for (edge_id, edge_data) in &adj_list.edges {
            edge_properties.insert(*edge_id, edge_data.properties.clone());
        }

        CSRGraph {
            num_vertices,
            num_edges: neighbors.len(),
            offsets,
            neighbors,
            edge_ids,
            vertex_map,
            vertex_index,
            vertex_properties,
            edge_properties,
        }
    }

    /// Get neighbors of a vertex
    pub fn get_neighbors(&self, vertex: VertexId) -> Option<&[VertexId]> {
        self.vertex_index.get(&vertex).map(|&idx| {
            let start = self.offsets[idx];
            let end = self.offsets[idx + 1];
            &self.neighbors[start..end]
        })
    }

    /// Get out-degree of a vertex
    pub fn out_degree(&self, vertex: VertexId) -> Option<usize> {
        self.vertex_index.get(&vertex).map(|&idx| {
            self.offsets[idx + 1] - self.offsets[idx]
        })
    }

    /// Iterate over all edges
    pub fn edges_iter(&self) -> impl Iterator<Item = (VertexId, VertexId, EdgeId)> + '_ {
        self.vertex_map.iter().enumerate().flat_map(move |(idx, &vertex_id)| {
            let start = self.offsets[idx];
            let end = self.offsets[idx + 1];

            (start..end).map(move |i| {
                (vertex_id, self.neighbors[i], self.edge_ids[i])
            })
        })
    }

    /// Memory footprint in bytes
    pub fn memory_footprint(&self) -> usize {
        size_of::<Self>() +
        self.offsets.len() * size_of::<usize>() +
        self.neighbors.len() * size_of::<VertexId>() +
        self.edge_ids.len() * size_of::<EdgeId>()
    }
}

// ============================================================================
// Edge-Centric Storage
// ============================================================================

/// Edge-centric storage for streaming and incremental updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCentricStorage {
    /// Edges stored in insertion order
    pub edges: Vec<EdgeRecord>,

    /// Index: source vertex -> edge indices
    pub source_index: HashMap<VertexId, Vec<usize>>,

    /// Index: target vertex -> edge indices
    pub target_index: HashMap<VertexId, Vec<usize>>,

    /// Vertex metadata
    pub vertices: HashMap<VertexId, VertexData>,
}

/// Edge record for edge-centric storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub edge_id: EdgeId,
    pub source: VertexId,
    pub target: VertexId,
    pub label: String,
    pub properties: Properties,
    pub timestamp: Option<i64>,
    pub weight: Option<f64>,
}

impl EdgeCentricStorage {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            source_index: HashMap::new(),
            target_index: HashMap::new(),
            vertices: HashMap::new(),
        }
    }

    /// Add an edge to the storage
    pub fn add_edge(&mut self, edge: EdgeRecord) {
        let edge_idx = self.edges.len();

        self.source_index
            .entry(edge.source)
            .or_insert_with(Vec::new)
            .push(edge_idx);

        self.target_index
            .entry(edge.target)
            .or_insert_with(Vec::new)
            .push(edge_idx);

        self.edges.push(edge);
    }

    /// Get edges with a specific source
    pub fn edges_from(&self, source: VertexId) -> Vec<&EdgeRecord> {
        self.source_index
            .get(&source)
            .map(|indices| {
                indices.iter()
                    .filter_map(|&idx| self.edges.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get edges with a specific target
    pub fn edges_to(&self, target: VertexId) -> Vec<&EdgeRecord> {
        self.target_index
            .get(&target)
            .map(|indices| {
                indices.iter()
                    .filter_map(|&idx| self.edges.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get edges in a time range (for temporal graphs)
    pub fn edges_in_range(&self, start_time: i64, end_time: i64) -> Vec<&EdgeRecord> {
        self.edges.iter()
            .filter(|edge| {
                if let Some(ts) = edge.timestamp {
                    ts >= start_time && ts <= end_time
                } else {
                    false
                }
            })
            .collect()
    }
}

impl Default for EdgeCentricStorage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Graph Compression
// ============================================================================

/// Graph compression utilities
pub struct GraphCompression;

impl GraphCompression {
    /// Compress vertex IDs using gap encoding
    pub fn compress_vertex_ids(ids: &[VertexId]) -> Vec<u64> {
        if ids.is_empty() {
            return Vec::new();
        }

        let mut sorted_ids = ids.to_vec();
        sorted_ids.sort_unstable();

        let mut compressed = Vec::with_capacity(sorted_ids.len());
        compressed.push(sorted_ids[0]);

        for i in 1..sorted_ids.len() {
            let gap = sorted_ids[i] - sorted_ids[i - 1];
            compressed.push(gap);
        }

        compressed
    }

    /// Decompress vertex IDs from gap encoding
    pub fn decompress_vertex_ids(compressed: &[u64]) -> Vec<VertexId> {
        if compressed.is_empty() {
            return Vec::new();
        }

        let mut ids = Vec::with_capacity(compressed.len());
        ids.push(compressed[0]);

        for i in 1..compressed.len() {
            ids.push(ids[i - 1] + compressed[i]);
        }

        ids
    }

    /// Estimate compression ratio
    pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
        if original_size == 0 {
            return 0.0;
        }
        compressed_size as f64 / original_size as f64
    }
}

// ============================================================================
// Persistent Storage Manager
// ============================================================================

/// Graph storage manager for persistence
pub struct GraphStorageManager {
    /// Base directory for graph storage
    base_dir: PathBuf,

    /// Current storage format
    format: StorageFormat,
}

impl GraphStorageManager {
    /// Create a new storage manager
    pub fn new<P: AsRef<Path>>(base_dir: P, format: StorageFormat) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| DbError::Internal(format!("Failed to create directory: {}", e)))?);
        }

        Ok(Self { base_dir, format })
    }

    /// Save a graph to disk
    pub fn save_graph(&self, graph: &PropertyGraph, name: &str) -> Result<()> {
        let file_path = self.base_dir.join(format!("{}.graph", name)));

        match self.format {
            StorageFormat::AdjacencyList => {
                let adj_list = AdjacencyList::from_graph(graph);
                let data = adj_list.serialize()?;
                self.write_file(&file_path, &data)?;
            }
            StorageFormat::CSR => {
                let adj_list = AdjacencyList::from_graph(graph);
                let csr = CSRGraph::from_adjacency_list(&adj_list);
                let data = bincode::serialize(&csr)
                    .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))?);
                self.write_file(&file_path, &data)?;
            }
            _ => {
                return Err(DbError::Internal("Unsupported storage format".to_string()));
            }
        }

        Ok(())
    }

    /// Load a graph from disk
    pub fn load_graph(&self, name: &str) -> Result<PropertyGraph> {
        let file_path = self.base_dir.join(format!("{}.graph", name)));

        let data = self.read_file(&file_path)?;

        match self.format {
            StorageFormat::AdjacencyList => {
                let adj_list = AdjacencyList::deserialize(&data)?;
                Ok(Self::adjacency_list_to_graph(&adj_list)?)
            }
            StorageFormat::CSR => {
                let csr: CSRGraph = bincode::deserialize(&data)
                    .map_err(|e| DbError::Internal(format!("Deserialization error: {}", e)))?);
                Ok(Self::csr_to_graph(&csr)?)
            }
            _ => {
                Err(DbError::Internal("Unsupported storage format".to_string()))
            }
        }
    }

    /// Write data to file
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let mut file = File::create(path)
            .map_err(|e| DbError::Internal(format!("Failed to create file: {}", e)))?);

        file.write_all(data)
            .map_err(|e| DbError::Internal(format!("Failed to write file: {}", e)))?);

        Ok(())
    }

    /// Read data from file
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let mut file = File::open(path)
            .map_err(|e| DbError::Internal(format!("Failed to open file: {}", e)))?);

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| DbError::Internal(format!("Failed to read file: {}", e)))?);

        Ok(data)
    }

    /// Convert adjacency list to property graph
    fn adjacency_list_to_graph(adj_list: &AdjacencyList) -> Result<PropertyGraph> {
        let mut graph = PropertyGraph::new();

        // Add vertices
        for (vertex_id, vertex_data) in &adj_list.vertices {
            // Note: This is a simplified conversion; in practice, we'd need to handle ID generation
            let vertex = Vertex::with_properties(
                *vertex_id,
                vertex_data.labels.clone(),
                vertex_data.properties.clone(),
            );
            // We can't directly insert into the graph's internal structure
            // This is a limitation of the current API design
        }

        // Add edges
        for (_edge_id, edge_data) in &adj_list.edges {
            graph.add_edge(
                edge_data.source,
                edge_data.target,
                edge_data.label.clone(),
                edge_data.properties.clone(),
                edge_data.direction,
            )?;
        }

        Ok(graph)
    }

    /// Convert CSR to property graph
    fn csr_to_graph(csr: &CSRGraph) -> Result<PropertyGraph> {
        let mut graph = PropertyGraph::new();

        // Add vertices
        for (idx, &_vertex_id) in csr.vertex_map.iter().enumerate() {
            let properties = csr.vertex_properties.get(idx)
                .cloned()
                .unwrap_or_else(Properties::new);
            // Simplified conversion
        }

        // Add edges
        for (source, target, edge_id) in csr.edges_iter() {
            let properties = csr.edge_properties.get(&edge_id)
                .cloned()
                .unwrap_or_else(Properties::new);

            graph.add_edge(
                source,
                target,
                "edge".to_string(),
                properties,
                EdgeDirection::Directed,
            )?;
        }

        Ok(graph)
    }
}

// ============================================================================
// Index Structures
// ============================================================================

/// Graph index for fast lookups
#[derive(Debug, Clone)]
pub struct GraphIndex {
    /// Label index: label -> vertices
    pub vertex_label_index: HashMap<String<VertexId>>,

    /// Property index: property_key -> vertices (simplified to avoid Value as key)
    pub vertex_property_index: HashMap<String<VertexId>>,

    /// Edge label index: label -> edges
    pub edge_label_index: HashMap<String<EdgeId>>,

    /// Two-hop index for faster path queries
    pub two_hop_index: HashMap<VertexId<VertexId>>,
}

impl GraphIndex {
    pub fn new() -> Self {
        Self {
            vertex_label_index: HashMap::new(),
            vertex_property_index: HashMap::new(),
            edge_label_index: HashMap::new(),
            two_hop_index: HashMap::new(),
        }
    }

    /// Build index from a graph
    pub fn build_from_graph(graph: &PropertyGraph) -> Self {
        let mut index = Self::new();

        // Build vertex label index
        for vertex in graph.vertices() {
            for label in &vertex.labels {
                index.vertex_label_index
                    .entry(label.clone())
                    .or_insert_with(HashSet::new)
                    .insert(vertex.id);
            }

            // Build property index (only by key, not value)
            for key in vertex.properties.keys() {
                index.vertex_property_index
                    .entry(key.clone())
                    .or_insert_with(HashSet::new)
                    .insert(vertex.id);
            }
        }

        // Build edge label index
        for edge in graph.edges() {
            index.edge_label_index
                .entry(edge.label.clone())
                .or_insert_with(HashSet::new)
                .insert(edge.id);
        }

        // Build two-hop index
        for vertex in graph.vertices() {
            let mut two_hop_neighbors = HashSet::new();

            if let Ok(neighbors) = graph.get_outgoing_neighbors(vertex.id) {
                for neighbor in neighbors {
                    if let Ok(second_neighbors) = graph.get_outgoing_neighbors(neighbor) {
                        two_hop_neighbors.extend(second_neighbors);
                    }
                }
            }

            index.two_hop_index.insert(vertex.id, two_hop_neighbors);
        }

        index
    }

    /// Find vertices by label
    pub fn find_by_label(&self, label: &str) -> Option<&HashSet<VertexId>> {
        self.vertex_label_index.get(label)
    }

    /// Find vertices by property key (not value due to Value not implementing Hash)
    pub fn find_by_property(&self, key: &str) -> Option<&HashSet<VertexId>> {
        self.vertex_property_index.get(key)
    }

    /// Find edges by label
    pub fn find_edges_by_label(&self, label: &str) -> Option<&HashSet<EdgeId>> {
        self.edge_label_index.get(label)
    }

    /// Check if there's a two-hop path between vertices
    pub fn has_two_hop_path(&self, source: VertexId, target: VertexId) -> bool {
        self.two_hop_index
            .get(&source)
            .map(|neighbors| neighbors.contains(&target))
            .unwrap_or(false)
    }
}

impl Default for GraphIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory-Mapped Storage
// ============================================================================

/// Memory-mapped graph storage for large graphs
pub struct MemoryMappedGraph {
    /// Path to the memory-mapped file
    file_path: PathBuf,

    /// File size
    file_size: usize,
}

impl MemoryMappedGraph {
    /// Create a new memory-mapped graph
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        Ok(Self {
            file_path,
            file_size: 0,
        })
    }

    /// Map a graph to memory
    pub fn map_graph(&mut self, graph: &PropertyGraph) -> Result<()> {
        // Serialize graph
        let adj_list = AdjacencyList::from_graph(graph);
        let data = adj_list.serialize()?;

        // Write to file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.file_path)
            .map_err(|e| DbError::Internal(format!("Failed to open file: {}", e)))?);

        file.write_all(&data)
            .map_err(|e| DbError::Internal(format!("Failed to write: {}", e)))?);

        self.file_size = data.len();

        Ok(())
    }

    /// Get file size
    pub fn size(&self) -> usize {
        self.file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::property_graph::EdgeDirection;

    #[test]
    fn test_adjacency_list() {
        let mut graph = PropertyGraph::new();
        let v1 = graph.add_vertex(vec!["A".to_string()], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec!["B".to_string()], Properties::new()).unwrap();
        graph.add_edge(v1, v2, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let adj_list = AdjacencyList::from_graph(&graph);

        assert_eq!(adj_list.vertices.len(), 2);
        assert_eq!(adj_list.edges.len(), 1);
        assert!(adj_list.outgoing.contains_key(&v1));
    }

    #[test]
    fn test_csr_format() {
        let mut adj_list = AdjacencyList::new();

        let v1 = 1;
        let v2 = 2;

        adj_list.vertices.insert(v1, VertexData {
            id: v1,
            labels: vec![],
            properties: Properties::new(),
        });

        adj_list.vertices.insert(v2, VertexData {
            id: v2,
            labels: vec![],
            properties: Properties::new(),
        });

        adj_list.outgoing.insert(v1, vec![(v2, 1)]);
        adj_list.outgoing.insert(v2, vec![]);

        let csr = CSRGraph::from_adjacency_list(&adj_list);

        assert_eq!(csr.num_vertices, 2);
        assert_eq!(csr.num_edges, 1);
        assert_eq!(csr.offsets.len(), 3);
    }

    #[test]
    fn test_compression() {
        let ids = vec![1, 5, 6, 10, 15];
        let compressed = GraphCompression::compress_vertex_ids(&ids);
        let decompressed = GraphCompression::decompress_vertex_ids(&compressed);

        assert_eq!(ids, decompressed);
    }
}
