// # Data Lineage Tracking Module
//
// This module provides comprehensive data lineage tracking for RustyDB, enabling:
//
// - **Table-level lineage**: Track data flow between tables
// - **Column-level lineage**: Track transformations at the column level
// - **Transform tracking**: Monitor ETL operations and data pipelines
// - **Impact analysis**: Identify downstream dependencies (forward lineage)
// - **Root cause analysis**: Trace data origins (backward lineage)
// - **Visualization**: Export lineage graphs in DOT format for Graphviz
//
// ## Architecture
//
// The lineage module consists of three main components:
//
// 1. **LineageGraph** (`lineage_graph.rs`): Directed acyclic graph (DAG) representing
//    data flow with nodes (sources, transforms, targets) and edges (operations).
//
// 2. **LineageTracker** (`lineage_tracker.rs`): Core tracking engine that captures
//    lineage information from SQL operations, ETL pipelines, and data transformations.
//
// 3. **LineageQuery** (`lineage_query.rs`): Query interface for lineage analysis,
//    supporting forward/backward traversal and visualization export.
//
// ## Usage Examples
//
// ### Basic Table Lineage Tracking
//
// ```rust,no_run
// use rusty_db::lineage::{LineageTracker, OperationType};
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
//
// // Register tables
// tracker.register_table(1, "users".to_string())?;
// tracker.register_table(2, "user_summary".to_string())?;
//
// // Track lineage
// tracker.track_table_lineage(
//     1,  // source table
//     2,  // target table
//     OperationType::Insert,
//     Some("INSERT INTO user_summary SELECT * FROM users".to_string()),
//     None,
// )?;
// # Ok(())
// # }
// ```
//
// ### Column-Level Lineage
//
// ```rust,no_run
// use rusty_db::lineage::LineageTracker;
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
//
// // Register tables and columns
// tracker.register_table(1, "users".to_string())?;
// tracker.register_column(1, 1, "email".to_string())?;
// tracker.register_table(2, "contacts".to_string())?;
// tracker.register_column(2, 1, "contact_email".to_string())?;
//
// // Track column transformation
// tracker.track_column_lineage(
//     1, 1,  // source table, column
//     2, 1,  // target table, column
//     Some("UPPER".to_string()),  // transformation function
// )?;
// # Ok(())
// # }
// ```
//
// ### ETL Transform Tracking
//
// ```rust,no_run
// use rusty_db::lineage::LineageTracker;
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
//
// // Start ETL operation
// let transform_id = tracker.start_transform(
//     "Daily ETL".to_string(),
//     vec![1, 2],  // input tables
//     vec![3],     // output tables
//     "MERGE raw_data INTO processed_data".to_string(),
// )?;
//
// // ... perform ETL operations ...
//
// // Mark as completed
// tracker.complete_transform(transform_id)?;
// # Ok(())
// # }
// ```
//
// ### Impact Analysis (Forward Lineage)
//
// ```rust,no_run
// use rusty_db::lineage::{LineageTracker, LineageQuery, EntityRef};
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
// // ... set up lineage ...
//
// let query = LineageQuery::new(&tracker)
//     .with_max_depth(5);
//
// let result = query.forward(EntityRef::Table { table_id: 1 })?;
//
// println!("Tables affected by changes to table 1:");
// for entity in result.entities {
//     println!("  - {} (depth: {})", entity.name, entity.depth);
// }
// # Ok(())
// # }
// ```
//
// ### Root Cause Analysis (Backward Lineage)
//
// ```rust,no_run
// use rusty_db::lineage::{LineageTracker, LineageQuery, EntityRef};
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
// // ... set up lineage ...
//
// let query = LineageQuery::new(&tracker);
// let result = query.backward(EntityRef::Table { table_id: 5 })?;
//
// println!("Source tables for table 5:");
// for entity in result.entities {
//     println!("  - {} (depth: {})", entity.name, entity.depth);
// }
// # Ok(())
// # }
// ```
//
// ### Lineage Visualization
//
// ```rust,no_run
// use rusty_db::lineage::{LineageTracker, LineageVisualizer, EntityRef};
//
// # fn example() -> rusty_db::Result<()> {
// let tracker = LineageTracker::new();
// // ... set up lineage ...
//
// let visualizer = LineageVisualizer::new(&tracker);
//
// // Export entire graph
// let dot = visualizer.export_to_dot(None, None)?;
// std::fs::write("lineage.dot", dot)?;
//
// // Export lineage for specific table
// let dot = visualizer.export_to_dot(
//     Some(EntityRef::Table { table_id: 1 }),
//     Some(10),  // max depth
// )?;
// std::fs::write("table1_lineage.dot", dot)?;
//
// // Convert to image: dot -Tpng lineage.dot -o lineage.png
// # Ok(())
// # }
// ```
//
// ## Integration with RustyDB
//
// The lineage module integrates with:
//
// - **Parser**: Automatically extracts lineage from SQL queries
// - **Execution**: Tracks lineage during query execution
// - **Catalog**: Uses table and column metadata
// - **Transaction**: Associates lineage with transactions
// - **Monitoring**: Provides lineage metrics and statistics
//
// ## Performance Considerations
//
// - Thread-safe using Arc<RwLock<>> for concurrent access
// - Limits on graph size (100K nodes, 500K edges) prevent memory exhaustion
// - Efficient graph traversal using BFS with visited set
// - Lazy evaluation: lineage computed on-demand
//
// ## Future Enhancements
//
// - Persistent storage of lineage information
// - Time-travel lineage queries
// - Lineage-based access control
// - Automated data quality tracking via lineage
// - Machine learning-based lineage inference

// Core lineage graph structures
pub mod lineage_graph;

// Lineage tracking engine
pub mod lineage_tracker;

// Lineage query and analysis
pub mod lineage_query;

// Re-export main types for convenience
pub use lineage_graph::{
    LineageEdge, LineageGraph, LineageNode, NodeId, NodeType, OperationType,
};

pub use lineage_tracker::{
    ColumnLineage, LineageId, LineageTracker, TableLineage, TransformOperation, TransformStatus,
};

pub use lineage_query::{
    EntityRef, LineageDirection, LineageEntity, LineageQuery, LineageQueryResult,
    LineageVisualizer,
};
