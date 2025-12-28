// # Data Lineage Graph
//
// This module provides directed acyclic graph (DAG) representation for data lineage.
// It tracks the flow of data through tables, columns, and transformations.

use crate::common::{ColumnId, TableId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Unique identifier for lineage nodes
pub type NodeId = String;

/// Maximum number of nodes in a lineage graph to prevent memory exhaustion
const MAX_LINEAGE_NODES: usize = 100_000;

/// Maximum number of edges in a lineage graph
const MAX_LINEAGE_EDGES: usize = 500_000;

/// Type of lineage node representing different data entities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Data source (table)
    Source {
        table_id: TableId,
        table_name: String,
    },
    /// Intermediate transformation
    Transform {
        operation: String,
        description: String,
    },
    /// Data target (table)
    Target {
        table_id: TableId,
        table_name: String,
    },
    /// Column-level entity
    Column {
        table_id: TableId,
        column_id: ColumnId,
        column_name: String,
    },
    /// External data source
    External {
        source_name: String,
        source_type: String,
    },
}

/// Lineage node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Unique identifier
    pub id: NodeId,

    /// Node type
    pub node_type: NodeType,

    /// When this node was created
    pub created_at: SystemTime,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl LineageNode {
    /// Create a new lineage node
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the node
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Type of operation that created the edge
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// Direct copy/move
    Copy,
    /// SQL SELECT
    Select,
    /// SQL INSERT
    Insert,
    /// SQL UPDATE
    Update,
    /// SQL DELETE
    Delete,
    /// JOIN operation
    Join,
    /// UNION operation
    Union,
    /// Aggregation (SUM, COUNT, etc.)
    Aggregate,
    /// Filter/WHERE clause
    Filter,
    /// Transformation function
    Transform { function_name: String },
    /// ETL operation
    ETL { pipeline_name: String },
    /// Custom operation
    Custom { operation_name: String },
}

/// Edge in the lineage graph representing data flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Source node ID
    pub from: NodeId,

    /// Target node ID
    pub to: NodeId,

    /// Type of operation
    pub operation: OperationType,

    /// When this edge was created
    pub created_at: SystemTime,

    /// SQL query or transformation logic
    pub transformation_logic: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl LineageEdge {
    /// Create a new lineage edge
    pub fn new(from: NodeId, to: NodeId, operation: OperationType) -> Self {
        Self {
            from,
            to,
            operation,
            created_at: SystemTime::now(),
            transformation_logic: None,
            metadata: HashMap::new(),
        }
    }

    /// Set transformation logic
    pub fn with_logic(mut self, logic: String) -> Self {
        self.transformation_logic = Some(logic);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Lineage graph storing nodes and edges
#[derive(Debug, Clone)]
pub struct LineageGraph {
    /// All nodes in the graph
    nodes: Arc<RwLock<HashMap<NodeId, LineageNode>>>,

    /// Adjacency list: from -> [to]
    edges: Arc<RwLock<HashMap<NodeId, Vec<LineageEdge>>>>,

    /// Reverse adjacency list: to -> [from] for backward traversal
    reverse_edges: Arc<RwLock<HashMap<NodeId, Vec<NodeId>>>>,
}

impl LineageGraph {
    /// Create a new empty lineage graph
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            edges: Arc::new(RwLock::new(HashMap::new())),
            reverse_edges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&self, node: LineageNode) -> Result<()> {
        let mut nodes = self.nodes.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if nodes.len() >= MAX_LINEAGE_NODES {
            return Err(DbError::LimitExceeded(
                format!("Maximum lineage nodes ({}) exceeded", MAX_LINEAGE_NODES)
            ));
        }

        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Add an edge to the graph
    pub fn add_edge(&self, edge: LineageEdge) -> Result<()> {
        // Validate nodes exist
        {
            let nodes = self.nodes.read()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            if !nodes.contains_key(&edge.from) {
                return Err(DbError::NotFound(
                    format!("Source node {} not found", edge.from)
                ));
            }

            if !nodes.contains_key(&edge.to) {
                return Err(DbError::NotFound(
                    format!("Target node {} not found", edge.to)
                ));
            }
        }

        // Check edge limit
        {
            let edges = self.edges.read()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            let total_edges: usize = edges.values().map(|v| v.len()).sum();
            if total_edges >= MAX_LINEAGE_EDGES {
                return Err(DbError::LimitExceeded(
                    format!("Maximum lineage edges ({}) exceeded", MAX_LINEAGE_EDGES)
                ));
            }
        }

        // Add forward edge
        {
            let mut edges = self.edges.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            edges.entry(edge.from.clone())
                .or_insert_with(Vec::new)
                .push(edge.clone());
        }

        // Add reverse edge
        {
            let mut reverse_edges = self.reverse_edges.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            reverse_edges.entry(edge.to.clone())
                .or_insert_with(Vec::new)
                .push(edge.from.clone());
        }

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &NodeId) -> Result<Option<LineageNode>> {
        let nodes = self.nodes.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(nodes.get(node_id).cloned())
    }

    /// Get all outgoing edges from a node
    pub fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<LineageEdge>> {
        let edges = self.edges.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(edges.get(node_id).cloned().unwrap_or_default())
    }

    /// Get all incoming edges to a node
    pub fn get_incoming_nodes(&self, node_id: &NodeId) -> Result<Vec<NodeId>> {
        let reverse_edges = self.reverse_edges.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(reverse_edges.get(node_id).cloned().unwrap_or_default())
    }

    /// Perform forward traversal (impact analysis) from a starting node
    pub fn traverse_forward(&self, start_node: &NodeId, max_depth: Option<usize>) -> Result<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start_node.clone(), 0));
        visited.insert(start_node.clone());

        let max_d = max_depth.unwrap_or(usize::MAX);

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_d {
                continue;
            }

            result.push(current.clone());

            let outgoing = self.get_outgoing_edges(&current)?;
            for edge in outgoing {
                if !visited.contains(&edge.to) {
                    visited.insert(edge.to.clone());
                    queue.push_back((edge.to.clone(), depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Perform backward traversal (root cause analysis) from a starting node
    pub fn traverse_backward(&self, start_node: &NodeId, max_depth: Option<usize>) -> Result<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start_node.clone(), 0));
        visited.insert(start_node.clone());

        let max_d = max_depth.unwrap_or(usize::MAX);

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_d {
                continue;
            }

            result.push(current.clone());

            let incoming = self.get_incoming_nodes(&current)?;
            for from_node in incoming {
                if !visited.contains(&from_node) {
                    visited.insert(from_node.clone());
                    queue.push_back((from_node.clone(), depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Get all nodes in the graph
    pub fn get_all_nodes(&self) -> Result<Vec<LineageNode>> {
        let nodes = self.nodes.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(nodes.values().cloned().collect())
    }

    /// Get number of nodes
    pub fn node_count(&self) -> Result<usize> {
        let nodes = self.nodes.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(nodes.len())
    }

    /// Get number of edges
    pub fn edge_count(&self) -> Result<usize> {
        let edges = self.edges.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let total: usize = edges.values().map(|v| v.len()).sum();
        Ok(total)
    }

    /// Clear the graph
    pub fn clear(&self) -> Result<()> {
        {
            let mut nodes = self.nodes.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            nodes.clear();
        }

        {
            let mut edges = self.edges.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            edges.clear();
        }

        {
            let mut reverse_edges = self.reverse_edges.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            reverse_edges.clear();
        }

        Ok(())
    }
}

impl Default for LineageGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_graph() {
        let graph = LineageGraph::new();
        assert_eq!(graph.node_count().unwrap(), 0);
        assert_eq!(graph.edge_count().unwrap(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let graph = LineageGraph::new();

        let node1 = LineageNode::new(
            "table1".to_string(),
            NodeType::Source {
                table_id: 1,
                table_name: "users".to_string(),
            },
        );

        let node2 = LineageNode::new(
            "table2".to_string(),
            NodeType::Target {
                table_id: 2,
                table_name: "user_summary".to_string(),
            },
        );

        graph.add_node(node1).unwrap();
        graph.add_node(node2).unwrap();

        assert_eq!(graph.node_count().unwrap(), 2);
    }

    #[test]
    fn test_add_edge() {
        let graph = LineageGraph::new();

        let node1 = LineageNode::new(
            "table1".to_string(),
            NodeType::Source {
                table_id: 1,
                table_name: "users".to_string(),
            },
        );

        let node2 = LineageNode::new(
            "table2".to_string(),
            NodeType::Target {
                table_id: 2,
                table_name: "user_summary".to_string(),
            },
        );

        graph.add_node(node1).unwrap();
        graph.add_node(node2).unwrap();

        let edge = LineageEdge::new(
            "table1".to_string(),
            "table2".to_string(),
            OperationType::Insert,
        );

        graph.add_edge(edge).unwrap();

        assert_eq!(graph.edge_count().unwrap(), 1);
    }

    #[test]
    fn test_traverse_forward() {
        let graph = LineageGraph::new();

        // Create chain: A -> B -> C
        let node_a = LineageNode::new("A".to_string(), NodeType::Source {
            table_id: 1,
            table_name: "A".to_string(),
        });
        let node_b = LineageNode::new("B".to_string(), NodeType::Transform {
            operation: "transform".to_string(),
            description: "B".to_string(),
        });
        let node_c = LineageNode::new("C".to_string(), NodeType::Target {
            table_id: 3,
            table_name: "C".to_string(),
        });

        graph.add_node(node_a).unwrap();
        graph.add_node(node_b).unwrap();
        graph.add_node(node_c).unwrap();

        graph.add_edge(LineageEdge::new("A".to_string(), "B".to_string(), OperationType::Select)).unwrap();
        graph.add_edge(LineageEdge::new("B".to_string(), "C".to_string(), OperationType::Insert)).unwrap();

        let result = graph.traverse_forward(&"A".to_string(), None).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"A".to_string()));
        assert!(result.contains(&"B".to_string()));
        assert!(result.contains(&"C".to_string()));
    }
}
