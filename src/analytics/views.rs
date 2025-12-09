// View and Materialized View Management
//
// This module provides types and operations for managing database views
// and materialized views, including:
//
// - **Views**: Virtual tables based on SQL queries
// - **Materialized Views**: Pre-computed query results stored on disk
// - **Refresh Scheduling**: Automatic refresh of materialized views
// - **View Statistics**: Access and performance tracking

use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::time::{Duration};
use crate::catalog::Schema;

// =============================================================================
// View Types
// =============================================================================

// View definition - a virtual table based on a SQL query.
//
// Views provide a logical abstraction over base tables without storing
// data physically. They are evaluated at query time.
//
// # Example
//
// ```rust,ignore
// let view = View {
//     name: "active_users".to_string(),
//     query: "SELECT * FROM users WHERE active = true".to_string(),
//     schema: user_schema,
//     updatable: true,
//     check_option: Some(CheckOption::Cascaded),
// };
// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    // Name of the view
    pub name: String,

    // SQL query defining the view
    pub query: String,

    // Schema of the view's result set
    pub schema: Schema,

    // Whether the view supports INSERT/UPDATE/DELETE
    pub updatable: bool,

    // Check option for updatable views
    pub check_option: Option<CheckOption>,
}

// Check option for updatable views.
//
// Controls validation of data modifications through views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckOption {
    // Only check the view's own WHERE clause
    Local,
    // Check all underlying view WHERE clauses
    Cascaded,
}

// =============================================================================
// Materialized View Types
// =============================================================================

// Materialized view definition with pre-computed data.
//
// Unlike regular views, materialized views store their query results
// physically, providing faster read performance at the cost of storage
// and refresh overhead.
//
// # Example
//
// ```rust,ignore
// let mv = MaterializedView {
//     name: "daily_sales_summary".to_string(),
//     query: "SELECT date, SUM(amount) FROM sales GROUP BY date".to_string(),
//     schema: sales_schema,
//     last_refreshed: SystemTime::now(),
//     refresh_schedule: Some(RefreshSchedule::daily()),
//     data: vec![],
//     indexes: vec![],
//     statistics: ViewStatistics::default(),
// };
// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    // Name of the materialized view
    pub name: String,

    // SQL query defining the view
    pub query: String,

    // Schema of the result set
    pub schema: Schema,

    // Last refresh timestamp
    pub last_refreshed: SystemTime,

    // Optional automatic refresh schedule
    pub refresh_schedule: Option<RefreshSchedule>,

    // Cached query result data
    pub data: Vec<Vec<String>>,

    // Indexes on the materialized view
    pub indexes: Vec<MaterializedViewIndex>,

    // Access and performance statistics
    pub statistics: ViewStatistics,
}

impl MaterializedView {
    // Create a new materialized view.
    pub fn new(name: String, query: String, schema: Schema) -> Self {
        Self {
            name,
            query,
            schema,
            last_refreshed: SystemTime::now(),
            refresh_schedule: None,
            data: Vec::new(),
            indexes: Vec::new(),
            statistics: ViewStatistics::default(),
        }
    }

    // Check if the materialized view needs refresh.
    pub fn needs_refresh(&self) -> bool {
        if let Some(schedule) = &self.refresh_schedule {
            SystemTime::now() >= schedule.next_refresh
        } else {
            false
        }
    }

    // Get the age of the data in seconds.
    pub fn data_age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.last_refreshed)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    // Add an index to the materialized view.
    pub fn add_index(&mut self, index: MaterializedViewIndex) {
        self.indexes.push(index);
    }

    // Update the data and refresh timestamp.
    pub fn update_data(&mut self, data: Vec<Vec<String>>) {
        self.data = data;
        self.last_refreshed = SystemTime::now();
        self.statistics.row_count = self.data.len() as u64;

        // Update next refresh time if scheduled
        if let Some(schedule) = &mut self.refresh_schedule {
            schedule.next_refresh = SystemTime::now() + schedule.interval;
        }
    }
}

// Refresh schedule for materialized views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshSchedule {
    // Interval between refreshes
    pub interval: Duration,

    // Next scheduled refresh time
    pub next_refresh: SystemTime,

    // Whether auto-refresh is enabled
    pub auto_refresh: bool,
}

impl RefreshSchedule {
    // Create a schedule that refreshes every N seconds.
    pub fn every_seconds(seconds: u64) -> Self {
        Self {
            interval: Duration::from_secs(seconds),
            next_refresh: SystemTime::now() + Duration::from_secs(seconds),
            auto_refresh: true,
        }
    }

    // Create a schedule that refreshes every N minutes.
    pub fn every_minutes(minutes: u64) -> Self {
        Self::every_seconds(minutes * 60)
    }

    // Create a schedule that refreshes every N hours.
    pub fn every_hours(hours: u64) -> Self {
        Self::every_seconds(hours * 3600)
    }

    // Create a daily refresh schedule.
    pub fn daily() -> Self {
        Self::every_hours(24)
    }
}

// Index on a materialized view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedViewIndex {
    // Index name
    pub name: String,

    // Indexed columns
    pub columns: Vec<String>,

    // Whether the index enforces uniqueness
    pub unique: bool,
}

impl MaterializedViewIndex {
    // Create a new non-unique index.
    pub fn new(name: String, columns: Vec<String>) -> Self {
        Self {
            name,
            columns,
            unique: false,
        }
    }

    // Create a new unique index.
    pub fn unique(name: String, columns: Vec<String>) -> Self {
        Self {
            name,
            columns,
            unique: true,
        }
    }
}

// =============================================================================
// View Statistics
// =============================================================================

// Statistics for a view or materialized view.
//
// Tracks access patterns and performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewStatistics {
    // Number of rows in the view
    pub row_count: u64,

    // Size of data in bytes
    pub data_size_bytes: u64,

    // Last access timestamp
    pub last_accessed: SystemTime,

    // Total number of accesses
    pub access_count: u64,

    // Average query time in milliseconds
    pub avg_query_time_ms: f64,
}

impl Default for ViewStatistics {
    fn default() -> Self {
        Self {
            row_count: 0,
            data_size_bytes: 0,
            last_accessed: SystemTime::now(),
            access_count: 0,
            avg_query_time_ms: 0.0,
        }
    }
}

impl ViewStatistics {
    // Record an access to the view.
    pub fn record_access(&mut self, query_time_ms: f64) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now();

        // Update running average
        let total_time = self.avg_query_time_ms * (self.access_count - 1) as f64;
        self.avg_query_time_ms = (total_time + query_time_ms) / self.access_count as f64;
    }

    // Get the access frequency (accesses per hour).
    pub fn access_frequency_per_hour(&self) -> f64 {
        // This would need first_accessed to calculate properly
        // Simplified: assume 1 hour of operation
        self.access_count as f64
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_creation() {
        let view = View {
            name: "test_view".to_string(),
            query: "SELECT * FROM users".to_string(),
            schema: Schema::new("test_view".to_string(), vec![]),
            updatable: false,
            check_option: None,
        };

        assert_eq!(view.name, "test_view");
        assert!(!view.updatable);
    }

    #[test]
    fn test_materialized_view_refresh() {
        let mut mv = MaterializedView::new(
            "test_mv".to_string(),
            "SELECT * FROM users".to_string(),
            Schema::new("test_mv".to_string(), vec![]),
        );

        mv.refresh_schedule = Some(RefreshSchedule::every_seconds(0));
        assert!(mv.needs_refresh());
    }

    #[test]
    fn test_view_statistics() {
        let mut stats = ViewStatistics::default();
        stats.record_access(10.0);
        stats.record_access(20.0);

        assert_eq!(stats.access_count, 2);
        assert!((stats.avg_query_time_ms - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_refresh_schedule() {
        let schedule = RefreshSchedule::daily();
        assert_eq!(schedule.interval, Duration::from_secs(86400));
        assert!(schedule.auto_refresh);
    }
}
