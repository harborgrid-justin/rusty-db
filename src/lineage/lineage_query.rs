// # Lineage Query Module
//
// This module provides advanced lineage querying capabilities:
// - Forward lineage (impact analysis): What depends on this data?
// - Backward lineage (root cause analysis): Where did this data come from?
// - Lineage visualization export (DOT format for Graphviz)

use crate::common::{ColumnId, TableId};
use crate::error::{DbError, Result};
use crate::lineage::lineage_graph::{LineageGraph, LineageNode, NodeId, NodeType, OperationType};
use crate::lineage::lineage_tracker::LineageTracker;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Lineage query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageQueryResult {
    /// Root entity that was queried
    pub root_entity: EntityRef,

    /// Direction of lineage query
    pub direction: LineageDirection,

    /// List of affected/source entities
    pub entities: Vec<LineageEntity>,

    /// Total depth of lineage
    pub max_depth: usize,
}

/// Reference to an entity in the lineage graph
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRef {
    /// Table reference
    Table { table_id: TableId },

    /// Column reference
    Column {
        table_id: TableId,
        column_id: ColumnId,
    },

    /// Transform reference
    Transform { transform_id: u64 },
}

/// Direction of lineage query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageDirection {
    /// Forward lineage (impact analysis)
    Forward,

    /// Backward lineage (root cause analysis)
    Backward,

    /// Both directions
    Both,
}

/// Entity in lineage result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEntity {
    /// Entity reference
    pub entity: EntityRef,

    /// Entity name
    pub name: String,

    /// Depth from root (0 = root)
    pub depth: usize,

    /// How this entity is related to root
    pub relationship: String,
}

/// Lineage query builder and executor
pub struct LineageQuery<'a> {
    tracker: &'a LineageTracker,
    max_depth: Option<usize>,
    include_transforms: bool,
}

impl<'a> LineageQuery<'a> {
    /// Create a new lineage query
    pub fn new(tracker: &'a LineageTracker) -> Self {
        Self {
            tracker,
            max_depth: None,
            include_transforms: true,
        }
    }

    /// Set maximum depth for traversal
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set whether to include transform nodes
    pub fn with_transforms(mut self, include: bool) -> Self {
        self.include_transforms = include;
        self
    }

    /// Execute forward lineage query (impact analysis)
    pub fn forward(&self, entity: EntityRef) -> Result<LineageQueryResult> {
        let node_id = self.entity_to_node_id(&entity)?;
        let graph = self.tracker.get_graph();

        let node_ids = graph.traverse_forward(&node_id, self.max_depth)?;
        let entities = self.build_entities(&node_ids, &entity, 0)?;

        let max_depth = entities.iter().map(|e| e.depth).max().unwrap_or(0);

        Ok(LineageQueryResult {
            root_entity: entity,
            direction: LineageDirection::Forward,
            entities,
            max_depth,
        })
    }

    /// Execute backward lineage query (root cause analysis)
    pub fn backward(&self, entity: EntityRef) -> Result<LineageQueryResult> {
        let node_id = self.entity_to_node_id(&entity)?;
        let graph = self.tracker.get_graph();

        let node_ids = graph.traverse_backward(&node_id, self.max_depth)?;
        let entities = self.build_entities(&node_ids, &entity, 0)?;

        let max_depth = entities.iter().map(|e| e.depth).max().unwrap_or(0);

        Ok(LineageQueryResult {
            root_entity: entity,
            direction: LineageDirection::Backward,
            entities,
            max_depth,
        })
    }

    /// Execute bidirectional lineage query
    pub fn both(&self, entity: EntityRef) -> Result<LineageQueryResult> {
        let node_id = self.entity_to_node_id(&entity)?;
        let graph = self.tracker.get_graph();

        let mut all_nodes = HashSet::new();

        // Forward traversal
        let forward_nodes = graph.traverse_forward(&node_id, self.max_depth)?;
        all_nodes.extend(forward_nodes);

        // Backward traversal
        let backward_nodes = graph.traverse_backward(&node_id, self.max_depth)?;
        all_nodes.extend(backward_nodes);

        let node_vec: Vec<NodeId> = all_nodes.into_iter().collect();
        let entities = self.build_entities(&node_vec, &entity, 0)?;

        let max_depth = entities.iter().map(|e| e.depth).max().unwrap_or(0);

        Ok(LineageQueryResult {
            root_entity: entity,
            direction: LineageDirection::Both,
            entities,
            max_depth,
        })
    }

    /// Find all tables that depend on a given table
    pub fn find_dependent_tables(&self, table_id: TableId) -> Result<Vec<TableId>> {
        let entity = EntityRef::Table { table_id };
        let result = self.forward(entity)?;

        let mut tables = Vec::new();
        for entity in result.entities {
            if let EntityRef::Table { table_id } = entity.entity {
                tables.push(table_id);
            }
        }

        Ok(tables)
    }

    /// Find all source tables for a given table
    pub fn find_source_tables(&self, table_id: TableId) -> Result<Vec<TableId>> {
        let entity = EntityRef::Table { table_id };
        let result = self.backward(entity)?;

        let mut tables = Vec::new();
        for entity in result.entities {
            if let EntityRef::Table { table_id } = entity.entity {
                tables.push(table_id);
            }
        }

        Ok(tables)
    }

    /// Find column lineage path
    pub fn find_column_path(
        &self,
        source_table_id: TableId,
        source_column_id: ColumnId,
        target_table_id: TableId,
        target_column_id: ColumnId,
    ) -> Result<Vec<LineageEntity>> {
        // Start from source column
        let source_entity = EntityRef::Column {
            table_id: source_table_id,
            column_id: source_column_id,
        };

        let result = self.forward(source_entity)?;

        // Filter entities to find path to target
        let target_ref = EntityRef::Column {
            table_id: target_table_id,
            column_id: target_column_id,
        };

        let path: Vec<LineageEntity> = result
            .entities
            .into_iter()
            .filter(|e| {
                e.entity == target_ref || matches!(e.entity, EntityRef::Column { .. })
            })
            .collect();

        Ok(path)
    }

    /// Convert entity reference to node ID
    fn entity_to_node_id(&self, entity: &EntityRef) -> Result<NodeId> {
        match entity {
            EntityRef::Table { table_id } => Ok(format!("table_{}", table_id)),
            EntityRef::Column { table_id, column_id } => {
                Ok(format!("column_{}_{}", table_id, column_id))
            }
            EntityRef::Transform { transform_id } => Ok(format!("transform_{}", transform_id)),
        }
    }

    /// Build lineage entities from node IDs
    fn build_entities(
        &self,
        node_ids: &[NodeId],
        root: &EntityRef,
        root_depth: usize,
    ) -> Result<Vec<LineageEntity>> {
        let graph = self.tracker.get_graph();
        let mut entities = Vec::new();

        for (i, node_id) in node_ids.iter().enumerate() {
            if let Some(node) = graph.get_node(node_id)? {
                // Skip transform nodes if not included
                if !self.include_transforms && matches!(node.node_type, NodeType::Transform { .. }) {
                    continue;
                }

                let entity_ref = self.node_to_entity(&node)?;
                let name = self.get_entity_name(&node)?;

                let entity = LineageEntity {
                    entity: entity_ref.clone(),
                    name,
                    depth: i.min(root_depth + i),
                    relationship: if entity_ref == *root {
                        "self".to_string()
                    } else {
                        "derived".to_string()
                    },
                };

                entities.push(entity);
            }
        }

        Ok(entities)
    }

    /// Convert node to entity reference
    fn node_to_entity(&self, node: &LineageNode) -> Result<EntityRef> {
        match &node.node_type {
            NodeType::Source { table_id, .. } | NodeType::Target { table_id, .. } => {
                Ok(EntityRef::Table { table_id: *table_id })
            }
            NodeType::Column { table_id, column_id, .. } => Ok(EntityRef::Column {
                table_id: *table_id,
                column_id: *column_id,
            }),
            NodeType::Transform { .. } => {
                // Extract transform ID from node ID
                let id_str = node.id.strip_prefix("transform_")
                    .ok_or_else(|| DbError::Internal("Invalid transform node ID".to_string()))?;
                let transform_id = id_str.parse::<u64>()
                    .map_err(|e| DbError::Internal(format!("Invalid transform ID: {}", e)))?;
                Ok(EntityRef::Transform { transform_id })
            }
            NodeType::External { source_name, .. } => {
                Err(DbError::NotImplemented(format!("External source {} not supported", source_name)))
            }
        }
    }

    /// Get human-readable name for entity
    fn get_entity_name(&self, node: &LineageNode) -> Result<String> {
        match &node.node_type {
            NodeType::Source { table_name, .. } | NodeType::Target { table_name, .. } => {
                Ok(table_name.clone())
            }
            NodeType::Column { column_name, .. } => Ok(column_name.clone()),
            NodeType::Transform { operation, .. } => Ok(operation.clone()),
            NodeType::External { source_name, .. } => Ok(source_name.clone()),
        }
    }
}

/// Lineage visualization exporter
pub struct LineageVisualizer<'a> {
    tracker: &'a LineageTracker,
}

impl<'a> LineageVisualizer<'a> {
    /// Create a new visualizer
    pub fn new(tracker: &'a LineageTracker) -> Self {
        Self { tracker }
    }

    /// Export lineage graph to DOT format (Graphviz)
    pub fn export_to_dot(&self, entity: Option<EntityRef>, max_depth: Option<usize>) -> Result<String> {
        let graph = self.tracker.get_graph();
        let mut dot = String::from("digraph lineage {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Get nodes to include
        let node_ids: Vec<NodeId> = if let Some(entity_ref) = entity {
            let query = LineageQuery::new(self.tracker)
                .with_max_depth(max_depth.unwrap_or(10));
            let result = query.both(entity_ref)?;
            result.entities.iter()
                .map(|e| match &e.entity {
                    EntityRef::Table { table_id } => format!("table_{}", table_id),
                    EntityRef::Column { table_id, column_id } => {
                        format!("column_{}_{}", table_id, column_id)
                    }
                    EntityRef::Transform { transform_id } => format!("transform_{}", transform_id),
                })
                .collect()
        } else {
            graph.get_all_nodes()?
                .iter()
                .map(|n| n.id.clone())
                .collect()
        };

        // Add nodes
        let mut node_labels: HashMap<NodeId, String> = HashMap::new();
        for node_id in &node_ids {
            if let Some(node) = graph.get_node(node_id)? {
                let (label, color) = self.format_node(&node)?;
                node_labels.insert(node_id.clone(), label.clone());
                dot.push_str(&format!(
                    "  \"{}\" [label=\"{}\", fillcolor=\"{}\", style=filled];\n",
                    node_id, label, color
                ));
            }
        }

        dot.push('\n');

        // Add edges
        for node_id in &node_ids {
            let edges = graph.get_outgoing_edges(node_id)?;
            for edge in edges {
                if node_ids.contains(&edge.to) {
                    let label = self.format_operation(&edge.operation);
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                        edge.from, edge.to, label
                    ));
                }
            }
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    /// Format node for visualization
    fn format_node(&self, node: &LineageNode) -> Result<(String, String)> {
        match &node.node_type {
            NodeType::Source { table_name, .. } => {
                Ok((format!("Source: {}", table_name), "lightblue".to_string()))
            }
            NodeType::Target { table_name, .. } => {
                Ok((format!("Target: {}", table_name), "lightgreen".to_string()))
            }
            NodeType::Column { column_name, .. } => {
                Ok((format!("Column: {}", column_name), "lightyellow".to_string()))
            }
            NodeType::Transform { operation, .. } => {
                Ok((format!("Transform: {}", operation), "orange".to_string()))
            }
            NodeType::External { source_name, .. } => {
                Ok((format!("External: {}", source_name), "pink".to_string()))
            }
        }
    }

    /// Format operation type for edge label
    fn format_operation(&self, op: &OperationType) -> String {
        match op {
            OperationType::Copy => "COPY".to_string(),
            OperationType::Select => "SELECT".to_string(),
            OperationType::Insert => "INSERT".to_string(),
            OperationType::Update => "UPDATE".to_string(),
            OperationType::Delete => "DELETE".to_string(),
            OperationType::Join => "JOIN".to_string(),
            OperationType::Union => "UNION".to_string(),
            OperationType::Aggregate => "AGG".to_string(),
            OperationType::Filter => "FILTER".to_string(),
            OperationType::Transform { function_name } => format!("TRANSFORM: {}", function_name),
            OperationType::ETL { pipeline_name } => format!("ETL: {}", pipeline_name),
            OperationType::Custom { operation_name } => operation_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineage::lineage_graph::OperationType;

    #[test]
    fn test_forward_lineage_query() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_table(2, "user_summary".to_string()).unwrap();

        tracker.track_table_lineage(
            1, 2,
            OperationType::Insert,
            Some("INSERT INTO user_summary SELECT * FROM users".to_string()),
            None,
        ).unwrap();

        let query = LineageQuery::new(&tracker);
        let result = query.forward(EntityRef::Table { table_id: 1 }).unwrap();

        assert_eq!(result.direction, LineageDirection::Forward);
        assert!(result.entities.len() >= 1);
    }

    #[test]
    fn test_backward_lineage_query() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_table(2, "user_summary".to_string()).unwrap();

        tracker.track_table_lineage(
            1, 2,
            OperationType::Insert,
            None,
            None,
        ).unwrap();

        let query = LineageQuery::new(&tracker);
        let result = query.backward(EntityRef::Table { table_id: 2 }).unwrap();

        assert_eq!(result.direction, LineageDirection::Backward);
    }

    #[test]
    fn test_find_dependent_tables() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "source".to_string()).unwrap();
        tracker.register_table(2, "target1".to_string()).unwrap();
        tracker.register_table(3, "target2".to_string()).unwrap();

        tracker.track_table_lineage(1, 2, OperationType::Insert, None, None).unwrap();
        tracker.track_table_lineage(1, 3, OperationType::Insert, None, None).unwrap();

        let query = LineageQuery::new(&tracker);
        let dependents = query.find_dependent_tables(1).unwrap();

        assert!(dependents.contains(&2));
        assert!(dependents.contains(&3));
    }

    #[test]
    fn test_visualizer_dot_export() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_table(2, "user_summary".to_string()).unwrap();

        tracker.track_table_lineage(
            1, 2,
            OperationType::Insert,
            None,
            None,
        ).unwrap();

        let visualizer = LineageVisualizer::new(&tracker);
        let dot = visualizer.export_to_dot(None, None).unwrap();

        assert!(dot.contains("digraph lineage"));
        assert!(dot.contains("users"));
        assert!(dot.contains("user_summary"));
    }
}
