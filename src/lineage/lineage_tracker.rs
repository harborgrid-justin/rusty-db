// # Data Lineage Tracker
//
// This module provides core lineage tracking functionality including:
// - Table-level lineage tracking
// - Column-level lineage tracking
// - Transform tracking (ETL operations)
// - Lineage graph construction

use crate::common::{ColumnId, TableId, TransactionId};
use crate::error::{DbError, Result};
use crate::lineage::lineage_graph::{
    LineageEdge, LineageGraph, LineageNode, NodeId, NodeType, OperationType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Unique identifier for lineage records
pub type LineageId = u64;

/// Column lineage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnLineage {
    /// Source table
    pub source_table_id: TableId,

    /// Source column
    pub source_column_id: ColumnId,

    /// Target table
    pub target_table_id: TableId,

    /// Target column
    pub target_column_id: ColumnId,

    /// Transformation applied
    pub transformation: Option<String>,

    /// When this lineage was created
    pub created_at: SystemTime,
}

/// Table lineage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableLineage {
    /// Source table
    pub source_table_id: TableId,

    /// Target table
    pub target_table_id: TableId,

    /// Type of operation
    pub operation_type: OperationType,

    /// SQL query that created this lineage
    pub sql_query: Option<String>,

    /// Transaction that created this lineage
    pub transaction_id: Option<TransactionId>,

    /// When this lineage was created
    pub created_at: SystemTime,
}

/// ETL/Transform operation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOperation {
    /// Unique identifier
    pub id: LineageId,

    /// Operation name
    pub name: String,

    /// Input tables
    pub input_tables: Vec<TableId>,

    /// Output tables
    pub output_tables: Vec<TableId>,

    /// Transformation logic
    pub logic: String,

    /// Operation status
    pub status: TransformStatus,

    /// When operation started
    pub started_at: SystemTime,

    /// When operation completed
    pub completed_at: Option<SystemTime>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Status of a transform operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformStatus {
    /// Operation is running
    Running,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed { error: String },
    /// Operation was cancelled
    Cancelled,
}

/// Main lineage tracker
pub struct LineageTracker {
    /// Lineage graph
    graph: LineageGraph,

    /// Column lineage records
    column_lineage: Arc<RwLock<Vec<ColumnLineage>>>,

    /// Table lineage records
    table_lineage: Arc<RwLock<Vec<TableLineage>>>,

    /// Transform operations
    transforms: Arc<RwLock<HashMap<LineageId, TransformOperation>>>,

    /// Table metadata (ID -> name)
    table_metadata: Arc<RwLock<HashMap<TableId, String>>>,

    /// Column metadata (Table ID, Column ID -> name)
    column_metadata: Arc<RwLock<HashMap<(TableId, ColumnId), String>>>,

    /// Next lineage ID
    next_id: Arc<RwLock<LineageId>>,
}

impl LineageTracker {
    /// Create a new lineage tracker
    pub fn new() -> Self {
        Self {
            graph: LineageGraph::new(),
            column_lineage: Arc::new(RwLock::new(Vec::new())),
            table_lineage: Arc::new(RwLock::new(Vec::new())),
            transforms: Arc::new(RwLock::new(HashMap::new())),
            table_metadata: Arc::new(RwLock::new(HashMap::new())),
            column_metadata: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Register table metadata
    pub fn register_table(&self, table_id: TableId, table_name: String) -> Result<()> {
        let mut metadata = self.table_metadata.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        metadata.insert(table_id, table_name.clone());

        // Add node to graph
        let node = LineageNode::new(
            format!("table_{}", table_id),
            NodeType::Source {
                table_id,
                table_name,
            },
        );
        self.graph.add_node(node)?;

        Ok(())
    }

    /// Register column metadata
    pub fn register_column(
        &self,
        table_id: TableId,
        column_id: ColumnId,
        column_name: String,
    ) -> Result<()> {
        let mut metadata = self.column_metadata.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        metadata.insert((table_id, column_id), column_name.clone());

        // Add column node to graph
        let node = LineageNode::new(
            format!("column_{}_{}", table_id, column_id),
            NodeType::Column {
                table_id,
                column_id,
                column_name,
            },
        );
        self.graph.add_node(node)?;

        Ok(())
    }

    /// Track table-level lineage
    pub fn track_table_lineage(
        &self,
        source_table_id: TableId,
        target_table_id: TableId,
        operation_type: OperationType,
        sql_query: Option<String>,
        transaction_id: Option<TransactionId>,
    ) -> Result<()> {
        // Create table lineage record
        let lineage = TableLineage {
            source_table_id,
            target_table_id,
            operation_type: operation_type.clone(),
            sql_query: sql_query.clone(),
            transaction_id,
            created_at: SystemTime::now(),
        };

        {
            let mut table_lineage = self.table_lineage.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            table_lineage.push(lineage);
        }

        // Add edge to graph
        let edge = LineageEdge::new(
            format!("table_{}", source_table_id),
            format!("table_{}", target_table_id),
            operation_type,
        ).with_logic(sql_query.unwrap_or_default());

        self.graph.add_edge(edge)?;

        Ok(())
    }

    /// Track column-level lineage
    pub fn track_column_lineage(
        &self,
        source_table_id: TableId,
        source_column_id: ColumnId,
        target_table_id: TableId,
        target_column_id: ColumnId,
        transformation: Option<String>,
    ) -> Result<()> {
        // Create column lineage record
        let lineage = ColumnLineage {
            source_table_id,
            source_column_id,
            target_table_id,
            target_column_id,
            transformation: transformation.clone(),
            created_at: SystemTime::now(),
        };

        {
            let mut column_lineage = self.column_lineage.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            column_lineage.push(lineage);
        }

        // Add edge to graph
        let operation = if let Some(transform) = transformation {
            OperationType::Transform {
                function_name: transform,
            }
        } else {
            OperationType::Copy
        };

        let edge = LineageEdge::new(
            format!("column_{}_{}", source_table_id, source_column_id),
            format!("column_{}_{}", target_table_id, target_column_id),
            operation,
        );

        self.graph.add_edge(edge)?;

        Ok(())
    }

    /// Start tracking a transform operation
    pub fn start_transform(
        &self,
        name: String,
        input_tables: Vec<TableId>,
        output_tables: Vec<TableId>,
        logic: String,
    ) -> Result<LineageId> {
        let id = {
            let mut next_id = self.next_id.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let transform = TransformOperation {
            id,
            name: name.clone(),
            input_tables: input_tables.clone(),
            output_tables: output_tables.clone(),
            logic: logic.clone(),
            status: TransformStatus::Running,
            started_at: SystemTime::now(),
            completed_at: None,
            metadata: HashMap::new(),
        };

        // Add transform node to graph
        let node = LineageNode::new(
            format!("transform_{}", id),
            NodeType::Transform {
                operation: name.clone(),
                description: logic.clone(),
            },
        );
        self.graph.add_node(node)?;

        // Add edges from inputs to transform
        for input_table in &input_tables {
            let edge = LineageEdge::new(
                format!("table_{}", input_table),
                format!("transform_{}", id),
                OperationType::ETL {
                    pipeline_name: name.clone(),
                },
            );
            self.graph.add_edge(edge)?;
        }

        // Add edges from transform to outputs
        for output_table in &output_tables {
            let edge = LineageEdge::new(
                format!("transform_{}", id),
                format!("table_{}", output_table),
                OperationType::ETL {
                    pipeline_name: name.clone(),
                },
            );
            self.graph.add_edge(edge)?;
        }

        {
            let mut transforms = self.transforms.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            transforms.insert(id, transform);
        }

        Ok(id)
    }

    /// Complete a transform operation
    pub fn complete_transform(&self, id: LineageId) -> Result<()> {
        let mut transforms = self.transforms.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let transform = transforms.get_mut(&id)
            .ok_or_else(|| DbError::NotFound(format!("Transform {} not found", id)))?;

        transform.status = TransformStatus::Completed;
        transform.completed_at = Some(SystemTime::now());

        Ok(())
    }

    /// Fail a transform operation
    pub fn fail_transform(&self, id: LineageId, error: String) -> Result<()> {
        let mut transforms = self.transforms.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let transform = transforms.get_mut(&id)
            .ok_or_else(|| DbError::NotFound(format!("Transform {} not found", id)))?;

        transform.status = TransformStatus::Failed { error };
        transform.completed_at = Some(SystemTime::now());

        Ok(())
    }

    /// Get transform operation by ID
    pub fn get_transform(&self, id: LineageId) -> Result<Option<TransformOperation>> {
        let transforms = self.transforms.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(transforms.get(&id).cloned())
    }

    /// Get all table lineage records for a table
    pub fn get_table_lineage(&self, table_id: TableId) -> Result<Vec<TableLineage>> {
        let lineage = self.table_lineage.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(lineage.iter()
            .filter(|l| l.source_table_id == table_id || l.target_table_id == table_id)
            .cloned()
            .collect())
    }

    /// Get all column lineage records for a column
    pub fn get_column_lineage(
        &self,
        table_id: TableId,
        column_id: ColumnId,
    ) -> Result<Vec<ColumnLineage>> {
        let lineage = self.column_lineage.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(lineage.iter()
            .filter(|l| {
                (l.source_table_id == table_id && l.source_column_id == column_id) ||
                (l.target_table_id == table_id && l.target_column_id == column_id)
            })
            .cloned()
            .collect())
    }

    /// Get the lineage graph
    pub fn get_graph(&self) -> &LineageGraph {
        &self.graph
    }

    /// Get table name from metadata
    pub fn get_table_name(&self, table_id: TableId) -> Result<Option<String>> {
        let metadata = self.table_metadata.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(metadata.get(&table_id).cloned())
    }

    /// Get column name from metadata
    pub fn get_column_name(&self, table_id: TableId, column_id: ColumnId) -> Result<Option<String>> {
        let metadata = self.column_metadata.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(metadata.get(&(table_id, column_id)).cloned())
    }

    /// Get all transforms
    pub fn get_all_transforms(&self) -> Result<Vec<TransformOperation>> {
        let transforms = self.transforms.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(transforms.values().cloned().collect())
    }

    /// Clear all lineage data
    pub fn clear(&self) -> Result<()> {
        self.graph.clear()?;

        {
            let mut column_lineage = self.column_lineage.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            column_lineage.clear();
        }

        {
            let mut table_lineage = self.table_lineage.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            table_lineage.clear();
        }

        {
            let mut transforms = self.transforms.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            transforms.clear();
        }

        {
            let mut table_metadata = self.table_metadata.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            table_metadata.clear();
        }

        {
            let mut column_metadata = self.column_metadata.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            column_metadata.clear();
        }

        Ok(())
    }
}

impl Default for LineageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_table() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();

        let name = tracker.get_table_name(1).unwrap();
        assert_eq!(name, Some("users".to_string()));
    }

    #[test]
    fn test_register_column() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_column(1, 1, "id".to_string()).unwrap();

        let name = tracker.get_column_name(1, 1).unwrap();
        assert_eq!(name, Some("id".to_string()));
    }

    #[test]
    fn test_track_table_lineage() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_table(2, "user_summary".to_string()).unwrap();

        tracker.track_table_lineage(
            1,
            2,
            OperationType::Insert,
            Some("INSERT INTO user_summary SELECT * FROM users".to_string()),
            None,
        ).unwrap();

        let lineage = tracker.get_table_lineage(1).unwrap();
        assert_eq!(lineage.len(), 1);
        assert_eq!(lineage[0].source_table_id, 1);
        assert_eq!(lineage[0].target_table_id, 2);
    }

    #[test]
    fn test_track_column_lineage() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "users".to_string()).unwrap();
        tracker.register_column(1, 1, "email".to_string()).unwrap();
        tracker.register_table(2, "contacts".to_string()).unwrap();
        tracker.register_column(2, 1, "contact_email".to_string()).unwrap();

        tracker.track_column_lineage(
            1, 1, 2, 1,
            Some("UPPER".to_string()),
        ).unwrap();

        let lineage = tracker.get_column_lineage(1, 1).unwrap();
        assert_eq!(lineage.len(), 1);
    }

    #[test]
    fn test_transform_tracking() {
        let tracker = LineageTracker::new();
        tracker.register_table(1, "raw_data".to_string()).unwrap();
        tracker.register_table(2, "processed_data".to_string()).unwrap();

        let id = tracker.start_transform(
            "ETL Pipeline".to_string(),
            vec![1],
            vec![2],
            "TRANSFORM raw_data TO processed_data".to_string(),
        ).unwrap();

        let transform = tracker.get_transform(id).unwrap().unwrap();
        assert_eq!(transform.status, TransformStatus::Running);

        tracker.complete_transform(id).unwrap();

        let transform = tracker.get_transform(id).unwrap().unwrap();
        assert_eq!(transform.status, TransformStatus::Completed);
    }
}
