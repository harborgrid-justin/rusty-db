//! # Graph Database Engine
//!
//! Comprehensive property graph database implementation with PGQL-like queries.
//!
//! ## Features
//!
//! - **Property Graph Model**: Vertices and edges with rich properties
//! - **Multi-graph Support**: Multiple edges between vertices
//! - **Hypergraph Extensions**: Edges connecting multiple vertices
//! - **PGQL-like Queries**: Pattern matching, path queries, and graph traversal
//! - **Graph Algorithms**: PageRank, community detection, centrality measures
//! - **Efficient Storage**: Adjacency list, CSR format, and compression
//! - **Graph Analytics**: Temporal analysis, machine learning features, recommendations
//!
//! ## Architecture
//!
//! The graph engine is organized into several key modules:
//!
//! - `property_graph`: Core graph data structure with vertices, edges, and properties
//! - `query_engine`: PGQL-like query parsing and execution
//! - `algorithms`: Graph algorithms (PageRank, centrality, community detection)
//! - `storage`: Persistent storage formats and serialization
//! - `analytics`: Advanced analytics, temporal graphs, and ML features
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use rusty_db::graph::{PropertyGraph, Properties};
//! use rusty_db::common::Value;
//!
//! # fn example() -> rusty_db::Result<()> {
//! // Create a new graph
//! let mut graph = PropertyGraph::new();
//!
//! // Add vertices with properties
//! let mut props1 = Properties::new();
//! props1.set("name".to_string(), Value::String("Alice".to_string()));
//! props1.set("age".to_string(), Value::Integer(30));
//! let alice = graph.add_vertex(vec!["Person".to_string()], props1)?;
//!
//! let mut props2 = Properties::new();
//! props2.set("name".to_string(), Value::String("Bob".to_string()));
//! props2.set("age".to_string(), Value::Integer(25));
//! let bob = graph.add_vertex(vec!["Person".to_string()], props2)?;
//!
//! // Add an edge
//! use rusty_db::graph::property_graph::EdgeDirection;
//! let edge_props = Properties::new();
//! graph.add_edge(
//!     alice,
//!     bob,
//!     "KNOWS".to_string(),
//!     edge_props,
//!     EdgeDirection::Directed,
//! )?;
//!
//! // Query the graph
//! let neighbors = graph.get_outgoing_neighbors(alice)?;
//! println!("Alice knows {} people", neighbors.len());
//!
//! // Get graph statistics
//! let _stats = graph.get_stats();
//! println!("Graph has {} vertices and {} edges", stats.num_vertices, stats.num_edges);
//! # Ok(())
//! # }
//! ```
//!
//! ## Query Examples
//!
//! The query engine supports PGQL-like pattern matching:
//!
//! ```rust,no_run
//! use rusty_db::graph::{PropertyGraph, QueryExecutor};
//! use rusty_db::graph::query_engine::*;
//!
//! # fn example(graph: &PropertyGraph) -> rusty_db::Result<()> {
//! // Create a query executor
//! let executor = QueryExecutor::new(graph);
//!
//! // Pattern matching example (simplified)
//! // MATCH (a:Person)-[:KNOWS]->(b:Person)
//! // WHERE a.age > 25
//! // RETURN a, b
//! # Ok(())
//! # }
//! ```
//!
//! ## Graph Algorithms
//!
//! ```rust,no_run
//! use rusty_db::graph::{PropertyGraph, PageRank, PageRankConfig};
//!
//! # fn example(graph: &PropertyGraph) -> rusty_db::Result<()> {
//! // Compute PageRank
//! let config = PageRankConfig::default();
//! let _result = PageRank::compute(graph, &config)?;
//!
//! // Get top-k vertices by PageRank
//! let top_vertices = PageRank::top_k(&result, 10);
//! for (vertex_id, score) in top_vertices {
//!     println!("Vertex {}: score = {:.4}", vertex_id, score);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Storage and Persistence
//!
//! ```rust,no_run
//! use rusty_db::graph::{PropertyGraph, GraphStorageManager, StorageFormat};
//! use std::path::Path;
//!
//! # fn example(graph: &PropertyGraph) -> rusty_db::Result<()> {
//! // Create storage manager
//! let manager = GraphStorageManager::new(
//!     Path::new("./graph_data"),
//!     StorageFormat::AdjacencyList,
//! )?;
//!
//! // Save graph
//! manager.save_graph(graph, "my_graph")?;
//!
//! // Load graph
//! let loaded_graph = manager.load_graph("my_graph")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration with Relational Data
//!
//! The graph engine can integrate with relational tables:
//!
//! ```rust,no_run
//! use rusty_db::graph::{PropertyGraph, GraphRelationalBridge};
//!
//! # fn example() -> rusty_db::Result<()> {
//! let mut graph = PropertyGraph::new();
//! let mut bridge = GraphRelationalBridge::new();
//!
//! // Map relational tables to graph vertices
//! // Map foreign keys to graph edges
//! // Execute graph queries and convert results back to relational format
//! # Ok(())
//! # }
//! ```

// ============================================================================
// Module Declarations
// ============================================================================

/// Property graph data structure
pub mod property_graph;

/// Query engine with PGQL-like support
pub mod query_engine;

/// Graph algorithms
pub mod algorithms;

/// Storage formats and persistence
pub mod storage;

/// Advanced analytics and ML features
pub mod analytics;

// ============================================================================
// Re-exports for Convenience
// ============================================================================

// Property Graph
pub use property_graph::{
    PropertyGraph,
    Vertex,
    Edge,
    Properties,
    VertexId,
    EdgeId,
    EdgeDirection,
    HyperEdge,
    GraphStats,
    PartitioningStrategy,
};

// Query Engine
pub use query_engine::{
    QueryExecutor,
    PatternMatcher,
    PathFinder,
    GraphTraversal,
    GraphQuery,
    QueryResult,
    MatchClause,
    GraphPattern,
    VertexPattern,
    EdgePattern,
    PropertyConstraint,
    WhereClause,
    ReturnClause,
    OrderByClause,
};

// Algorithms
pub use algorithms::{
    PageRank,
    PageRankConfig,
    PageRankResult,
    ConnectedComponentsAlgorithm,
    ConnectedComponents,
    CentralityAlgorithms,
    BetweennessCentrality,
    ClosenessCentrality,
    DegreeCentrality,
    LouvainAlgorithm,
    CommunityDetectionResult,
    TriangleCounting,
    TriangleCountResult,
    ClusteringCoefficientAlgorithm,
    ClusteringCoefficient,
    InfluenceMaximization,
    InfluenceModel,
    jaccard_similarity,
    cosine_similarity,
    common_neighbors,
};

// Storage
pub use storage::{
    StorageFormat,
    AdjacencyList,
    CSRGraph,
    EdgeCentricStorage,
    GraphStorageManager,
    GraphIndex,
    GraphCompression,
    MemoryMappedGraph,
};

// Analytics
pub use analytics::{
    GraphRelationalBridge,
    MatchExecutor,
    PathEnumerator,
    TemporalGraph,
    TemporalEvent,
    TemporalMetrics,
    GraphEmbedding,
    RecommendationEngine,
};

// ============================================================================
// Module-level Functions
// ============================================================================

/// Create a new property graph instance
pub fn new_graph() -> PropertyGraph {
    PropertyGraph::new()
}

/// Create a property graph with partitioning
pub fn new_partitioned_graph(
    strategy: PartitioningStrategy,
    num_partitions: u32,
) -> PropertyGraph {
    PropertyGraph::with_partitioning(strategy, num_partitions)
}

// ============================================================================
// Version Information
// ============================================================================

/// Graph database engine version
pub const VERSION: &str = "1.0.0";

/// Feature flags
pub mod features {
    /// Multi-graph support enabled
    pub const MULTI_GRAPH: bool = true;

    /// Hypergraph support enabled
    pub const HYPERGRAPH: bool = true;

    /// Temporal graph support enabled
    pub const TEMPORAL: bool = true;

    /// Graph partitioning enabled
    pub const PARTITIONING: bool = true;

    /// Graph compression enabled
    pub const COMPRESSION: bool = true;

    /// Machine learning features enabled
    pub const ML_FEATURES: bool = true;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Value;

    #[test]
    fn test_graph_creation() {
        let graph = new_graph();
        assert_eq!(graph.vertex_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_basic_graph_operations() {
        let mut graph = new_graph();

        // Add vertices
        let mut props = Properties::new();
        props.set("name".to_string(), Value::String("Alice".to_string()));
        let v1 = graph.add_vertex(vec!["Person".to_string()], props).unwrap();

        let mut props = Properties::new();
        props.set("name".to_string(), Value::String("Bob".to_string()));
        let v2 = graph.add_vertex(vec!["Person".to_string()], props).unwrap();

        assert_eq!(graph.vertex_count(), 2);

        // Add edge
        let _edge = graph.add_edge(
            v1,
            v2,
            "KNOWS".to_string(),
            Properties::new(),
            EdgeDirection::Directed,
        ).unwrap();

        assert_eq!(graph.edge_count(), 1);

        // Check neighbors
        let neighbors = graph.get_outgoing_neighbors(v1).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], v2);
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = new_graph();

        let v1 = graph.add_vertex(vec!["Person".to_string()], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec!["Person".to_string()], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec!["Person".to_string()], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "KNOWS".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v2, v3, "KNOWS".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let _stats = graph.get_stats();
        assert_eq!(stats.num_vertices, 3);
        assert_eq!(stats.num_edges, 2);
        assert!(stats.avg_degree > 0.0);
    }

    #[test]
    fn test_pagerank() {
        let mut graph = new_graph();

        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v2, v3, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v3, v1, "link".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let config = PageRankConfig::default();
        let _result = PageRank::compute(&graph, &config).unwrap();

        assert!(result.converged);
        assert_eq!(result.scores.len(), 3);

        // All vertices should have similar scores in this symmetric graph
        let scores: Vec<f64> = result.scores.values().copied().collect();
        let sum: f64 = scores.iter().sum();
        assert!((sum - 1.0).abs() < 0.01); // PageRank scores sum to 1
    }

    #[test]
    fn test_connected_components() {
        let mut graph = new_graph();

        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v4 = graph.add_vertex(vec![], Properties::new()).unwrap();

        // Create two separate components
        graph.add_edge(v1, v2, "link".to_string(), Properties::new(), EdgeDirection::Undirected).unwrap();
        graph.add_edge(v3, v4, "link".to_string(), Properties::new(), EdgeDirection::Undirected).unwrap();

        let _result = ConnectedComponentsAlgorithm::compute(&graph).unwrap();

        assert_eq!(result.num_components, 2);
        assert!(ConnectedComponentsAlgorithm::same_component(&result, v1, v2));
        assert!(ConnectedComponentsAlgorithm::same_component(&result, v3, v4));
        assert!(!ConnectedComponentsAlgorithm::same_component(&result, v1, v3));
    }

    #[test]
    fn test_adjacency_list_storage() {
        let mut graph = new_graph();

        let v1 = graph.add_vertex(vec!["A".to_string()], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec!["B".to_string()], Properties::new()).unwrap();
        graph.add_edge(v1, v2, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let adj_list = AdjacencyList::from_graph(&graph);

        assert_eq!(adj_list.vertices.len(), 2);
        assert_eq!(adj_list.edges.len(), 1);
        assert!(adj_list.outgoing.contains_key(&v1));

        // Test serialization
        let serialized = adj_list.serialize().unwrap();
        let deserialized = AdjacencyList::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.vertices.len(), 2);
        assert_eq!(deserialized.edges.len(), 1);
    }

    #[test]
    fn test_csr_format() {
        let mut graph = new_graph();

        let v1 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec![], Properties::new()).unwrap();
        let v3 = graph.add_vertex(vec![], Properties::new()).unwrap();

        graph.add_edge(v1, v2, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();
        graph.add_edge(v1, v3, "E".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let adj_list = AdjacencyList::from_graph(&graph);
        let csr = CSRGraph::from_adjacency_list(&adj_list);

        assert_eq!(csr.num_vertices, 3);
        assert_eq!(csr.num_edges, 2);

        // Vertex v1 should have 2 outgoing edges
        let out_degree = csr.out_degree(v1).unwrap();
        assert_eq!(out_degree, 2);
    }
}


