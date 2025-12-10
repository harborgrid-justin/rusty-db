// Query Profiler
// Captures execution plans, row counts, time per operator, and wait events

use std::fmt;
use std::time::Instant;
use std::collections::VecDeque;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};


// Query execution operator types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatorType {
    TableScan,
    IndexScan,
    IndexSeek,
    NestedLoop,
    HashJoin,
    MergeJoin,
    Sort,
    HashAggregate,
    StreamAggregate,
    Filter,
    Project,
    Limit,
    Union,
    Intersect,
    Except,
    SubqueryScan,
    MaterializedView,
    WindowFunction,
}

impl fmt::Display for OperatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorType::TableScan => write!(f, "Table Scan"),
            OperatorType::IndexScan => write!(f, "Index Scan"),
            OperatorType::IndexSeek => write!(f, "Index Seek"),
            OperatorType::NestedLoop => write!(f, "Nested Loop"),
            OperatorType::HashJoin => write!(f, "Hash Join"),
            OperatorType::MergeJoin => write!(f, "Merge Join"),
            OperatorType::Sort => write!(f, "Sort"),
            OperatorType::HashAggregate => write!(f, "Hash Aggregate"),
            OperatorType::StreamAggregate => write!(f, "Stream Aggregate"),
            OperatorType::Filter => write!(f, "Filter"),
            OperatorType::Project => write!(f, "Projection"),
            OperatorType::Limit => write!(f, "Limit"),
            OperatorType::Union => write!(f, "Union"),
            OperatorType::Intersect => write!(f, "Intersect"),
            OperatorType::Except => write!(f, "Except"),
            OperatorType::SubqueryScan => write!(f, "Subquery Scan"),
            OperatorType::MaterializedView => write!(f, "Materialized View"),
            OperatorType::WindowFunction => write!(f, "Window Function"),
        }
    }
}

// Wait event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WaitEventType {
    CpuExecution,
    DiskRead,
    DiskWrite,
    NetworkIO,
    LockAcquisition,
    BufferPoolWait,
    LogWrite,
    Checkpoint,
    ReplicationSync,
    IndexMaintenance,
    Other,
}

impl fmt::Display for WaitEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaitEventType::CpuExecution => write!(f, "CPU Execution"),
            WaitEventType::DiskRead => write!(f, "Disk Read"),
            WaitEventType::DiskWrite => write!(f, "Disk Write"),
            WaitEventType::NetworkIO => write!(f, "Network I/O"),
            WaitEventType::LockAcquisition => write!(f, "Lock Acquisition"),
            WaitEventType::BufferPoolWait => write!(f, "Buffer Pool Wait"),
            WaitEventType::LogWrite => write!(f, "Log Write"),
            WaitEventType::Checkpoint => write!(f, "Checkpoint"),
            WaitEventType::ReplicationSync => write!(f, "Replication Sync"),
            WaitEventType::IndexMaintenance => write!(f, "Index Maintenance"),
            WaitEventType::Other => write!(f, "Other"),
        }
    }
}

// Wait event details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEvent {
    pub event_type: WaitEventType,
    pub duration: Duration,
    pub count: u64,
    pub details: String,
}

impl WaitEvent {
    pub fn new(event_type: WaitEventType, duration: Duration) -> Self {
        Self {
            event_type,
            duration,
            count: 1,
            details: String::new(),
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }
}

// Execution plan operator node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOperator {
    pub id: usize,
    pub operator_type: OperatorType,
    pub estimated_rows: u64,
    pub estimated_cost: f64,
    pub actual_rows: Option<u64>,
    pub actual_time: Option<Duration>,
    pub children: Vec<PlanOperator>,
    pub details: HashMap<String, String>,
    pub wait_events: Vec<WaitEvent>,
}

impl PlanOperator {
    pub fn new(id: usize, operator_type: OperatorType) -> Self {
        Self {
            id,
            operator_type,
            estimated_rows: 0,
            estimated_cost: 0.0,
            actual_rows: None,
            actual_time: None,
            children: Vec::new(),
            details: HashMap::new(),
            wait_events: Vec::new(),
        }
    }

    pub fn with_estimates(mut self, rows: u64, cost: f64) -> Self {
        self.estimated_rows = rows;
        self.estimated_cost = cost;
        self
    }

    pub fn with_actuals(mut self, rows: u64, time: Duration) -> Self {
        self.actual_rows = Some(rows);
        self.actual_time = Some(time);
        self
    }

    pub fn add_child(mut self, child: PlanOperator) -> Self {
        self.children.push(child);
        self
    }

    pub fn add_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn add_wait_event(mut self, event: WaitEvent) -> Self {
        self.wait_events.push(event);
        self
    }

    // Calculate total time including children
    pub fn total_time(&self) -> Duration {
        let mut total = self.actual_time.unwrap_or(Duration::ZERO);
        for child in &self.children {
            total += child.total_time();
        }
        total
    }

    // Calculate total rows including children
    pub fn total_rows(&self) -> u64 {
        let mut total = self.actual_rows.unwrap_or(0);
        for child in &self.children {
            total += child.total_rows();
        }
        total
    }

    // Check for estimation errors
    pub fn estimation_accuracy(&self) -> Option<f64> {
        self.actual_rows.map(|actual| {
            if self.estimated_rows == 0 {
                if actual == 0 {
                    1.0
                } else {
                    0.0
                }
            } else {
                (actual as f64 / self.estimated_rows as f64).min(
                    self.estimated_rows as f64 / actual as f64
                )
            }
        })
    }

    // Format as tree for display
    pub fn format_tree(&self, indent: usize) -> String {
        let mut output = String::new();
        let prefix = "  ".repeat(indent);

        output.push_str(&format!("{}[{}] {}\n", prefix, self.id, self.operator_type));
        output.push_str(&format!(
            "{}  Estimated: {} rows, cost {:.2}\n",
            prefix, self.estimated_rows, self.estimated_cost
        ));

        if let (Some(rows), Some(time)) = (self.actual_rows, self.actual_time) {
            output.push_str(&format!(
                "{}  Actual: {} rows, {:.2}ms\n",
                prefix,
                rows,
                time.as_secs_f64() * 1000.0
            ));

            if let Some(accuracy) = self.estimation_accuracy() {
                output.push_str(&format!("{}  Estimation accuracy: {:.2}%\n", prefix, accuracy * 100.0));
            }
        }

        if !self.details.is_empty() {
            output.push_str(&format!("{}  Details:\n", prefix));
            for (key, value) in &self.details {
                output.push_str(&format!("{}    {}: {}\n", prefix, key, value));
            }
        }

        if !self.wait_events.is_empty() {
            output.push_str(&format!("{}  Wait Events:\n", prefix));
            for event in &self.wait_events {
                output.push_str(&format!(
                    "{}    {}: {:.2}ms (count: {})\n",
                    prefix,
                    event.event_type,
                    event.duration.as_secs_f64() * 1000.0,
                    event.count
                ));
            }
        }

        for child in &self.children {
            output.push_str(&child.format_tree(indent + 1));
        }

        output
    }
}

// Query execution profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub query_id: u64,
    pub sql: String,
    pub plan: Option<PlanOperator>,
    pub total_execution_time: Duration,
    pub parse_time: Duration,
    pub optimize_time: Duration,
    pub execution_time: Duration,
    pub wait_events: HashMap<WaitEventType, Vec<WaitEvent>>,
    pub rows_returned: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub locks_acquired: u64,
    pub timestamp: SystemTime,
}

impl QueryProfile {
    /// Reserved for profiling stats
    #[allow(dead_code)]
    pub(crate) fn total_execution_time(&self) -> &Duration {
        &self.total_execution_time
    }
}

impl QueryProfile {
    pub fn new(query_id: u64, sql: impl Into<String>) -> Self {
        Self {
            query_id,
            sql: sql.into(),
            plan: None,
            total_execution_time: Duration::ZERO,
            parse_time: Duration::ZERO,
            optimize_time: Duration::ZERO,
            execution_time: Duration::ZERO,
            wait_events: HashMap::new(),
            rows_returned: 0,
            bytes_read: 0,
            bytes_written: 0,
            cache_hits: 0,
            cache_misses: 0,
            locks_acquired: 0,
            timestamp: SystemTime::now(),
        }
    }

    pub fn set_plan(&mut self, plan: PlanOperator) {
        self.plan = Some(plan);
    }

    pub fn set_timing(
        &mut self,
        parse: Duration,
        optimize: Duration,
        execution: Duration,
    ) {
        self.parse_time = parse;
        self.optimize_time = optimize;
        self.execution_time = execution;
        self.total_execution_time = parse + optimize + execution;
    }

    pub fn add_wait_event(&mut self, event: WaitEvent) {
        self.wait_events
            .entry(event.event_type)
            .or_insert_with(Vec::new)
            .push(event);
    }

    pub fn get_wait_time(&self, event_type: WaitEventType) -> Duration {
        self.wait_events
            .get(&event_type)
            .map(|events| events.iter().map(|e| e.duration).sum())
            .unwrap_or(Duration::ZERO)
    }

    pub fn total_wait_time(&self) -> Duration {
        self.wait_events
            .values()
            .flatten()
            .map(|e| e.duration)
            .sum()
    }

    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    pub fn format_summary(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Query ID: {}\n", self.query_id));
        output.push_str(&format!("SQL: {}\n", self.sql));
        output.push_str(&format!(
            "Total Execution Time: {:.2}ms\n",
            self.total_execution_time.as_secs_f64() * 1000.0
        ));
        output.push_str(&format!(
            "  Parse: {:.2}ms\n",
            self.parse_time.as_secs_f64() * 1000.0
        ));
        output.push_str(&format!(
            "  Optimize: {:.2}ms\n",
            self.optimize_time.as_secs_f64() * 1000.0
        ));
        output.push_str(&format!(
            "  Execution: {:.2}ms\n",
            self.execution_time.as_secs_f64() * 1000.0
        ));
        output.push_str(&format!("Rows Returned: {}\n", self.rows_returned));
        output.push_str(&format!("Bytes Read: {}\n", self.bytes_read));
        output.push_str(&format!("Bytes Written: {}\n", self.bytes_written));
        output.push_str(&format!("Cache Hit Ratio: {:.2}%\n", self.cache_hit_ratio() * 100.0));
        output.push_str(&format!("Locks Acquired: {}\n", self.locks_acquired));

        if !self.wait_events.is_empty() {
            output.push_str("\nWait Events:\n");
            for (event_type, events) in &self.wait_events {
                let total_time: Duration = events.iter().map(|e| e.duration).sum();
                let total_count: u64 = events.iter().map(|e| e.count).sum();
                output.push_str(&format!(
                    "  {}: {:.2}ms (count: {})\n",
                    event_type,
                    total_time.as_secs_f64() * 1000.0,
                    total_count
                ));
            }
        }

        if let Some(plan) = &self.plan {
            output.push_str("\nExecution Plan:\n");
            output.push_str(&plan.format_tree(0));
        }

        output
    }
}

// Query profiler that tracks and stores query execution profiles
pub struct QueryProfiler {
    profiles: Arc<RwLock<VecDeque<QueryProfile>>>,
    max_profiles: usize,
    enabled: Arc<RwLock<bool>>,
    slow_query_threshold: Duration,
}

impl QueryProfiler {
    pub fn new(max_profiles: usize) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(VecDeque::with_capacity(max_profiles))),
            max_profiles,
            enabled: Arc::new(RwLock::new(true)),
            slow_query_threshold: Duration::from_secs(1),
        }
    }

    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn set_slow_query_threshold(&mut self, threshold: Duration) {
        self.slow_query_threshold = threshold;
    }

    pub fn record_profile(&self, profile: QueryProfile) {
        if !self.is_enabled() {
            return;
        }

        let mut profiles = self.profiles.write();
        if profiles.len() >= self.max_profiles {
            profiles.pop_front();
        }
        profiles.push_back(profile);
    }

    pub fn get_profile(&self, query_id: u64) -> Option<QueryProfile> {
        let profiles = self.profiles.read();
        profiles.iter().find(|p| p.query_id == query_id).cloned()
    }

    pub fn get_recent_profiles(&self, limit: usize) -> Vec<QueryProfile> {
        let profiles = self.profiles.read();
        profiles
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_slow_queries(&self) -> Vec<QueryProfile> {
        let profiles = self.profiles.read();
        profiles
            .iter()
            .filter(|p| p.total_execution_time >= self.slow_query_threshold)
            .cloned()
            .collect()
    }

    pub fn get_top_queries_by_time(&self, limit: usize) -> Vec<QueryProfile> {
        let profiles = self.profiles.read();
        let mut sorted: Vec<_> = profiles.iter().cloned().collect();
        sorted.sort_by(|a, b| b.total_execution_time.cmp(&a.total_execution_time));
        sorted.into_iter().take(limit).collect()
    }

    pub fn get_queries_with_estimation_errors(&self, threshold: f64) -> Vec<QueryProfile> {
        let profiles = self.profiles.read();
        profiles
            .iter()
            .filter(|p| {
                if let Some(plan) = &p.plan {
                    if let Some(accuracy) = plan.estimation_accuracy() {
                        return accuracy < threshold;
                    }
                }
                false
            })
            .cloned()
            .collect()
    }

    pub fn get_average_execution_time(&self) -> Duration {
        let profiles = self.profiles.read();
        if profiles.is_empty() {
            return Duration::ZERO;
        }

        let total: Duration = profiles.iter().map(|p| p.total_execution_time).sum();
        total / profiles.len() as u32
    }

    pub fn get_wait_event_summary(&self) -> HashMap<WaitEventType, (Duration, u64)> {
        let profiles = self.profiles.read();
        let mut summary: HashMap<WaitEventType, (Duration, u64)> = HashMap::new();

        for profile in profiles.iter() {
            for (event_type, events) in &profile.wait_events {
                let (total_time, total_count) = summary.entry(*event_type).or_insert((Duration::ZERO, 0));
                for event in events {
                    *total_time += event.duration;
                    *total_count += event.count;
                }
            }
        }

        summary
    }

    pub fn clear(&self) {
        self.profiles.write().clear();
    }
}

impl Default for QueryProfiler {
    fn default() -> Self {
        Self::new(1000)
    }
}

// Profile builder for constructing query profiles incrementally
pub struct ProfileBuilder {
    profile: QueryProfile,
    parse_start: Option<Instant>,
    optimize_start: Option<Instant>,
    execution_start: Option<Instant>,
}

impl ProfileBuilder {
    pub fn new(query_id: u64, sql: impl Into<String>) -> Self {
        Self {
            profile: QueryProfile::new(query_id, sql),
            parse_start: None,
            optimize_start: None,
            execution_start: None,
        }
    }

    pub fn start_parse(&mut self) {
        self.parse_start = Some(Instant::now());
    }

    pub fn end_parse(&mut self) {
        if let Some(start) = self.parse_start.take() {
            self.profile.parse_time = start.elapsed();
        }
    }

    pub fn start_optimize(&mut self) {
        self.optimize_start = Some(Instant::now());
    }

    pub fn end_optimize(&mut self) {
        if let Some(start) = self.optimize_start.take() {
            self.profile.optimize_time = start.elapsed();
        }
    }

    pub fn start_execution(&mut self) {
        self.execution_start = Some(Instant::now());
    }

    pub fn end_execution(&mut self) {
        if let Some(start) = self.execution_start.take() {
            self.profile.execution_time = start.elapsed();
        }
    }

    pub fn set_plan(&mut self, plan: PlanOperator) {
        self.profile.plan = Some(plan);
    }

    pub fn add_wait_event(&mut self, event: WaitEvent) {
        self.profile.add_wait_event(event);
    }

    pub fn set_rows_returned(&mut self, rows: u64) {
        self.profile.rows_returned = rows;
    }

    pub fn set_bytes_read(&mut self, bytes: u64) {
        self.profile.bytes_read = bytes;
    }

    pub fn set_bytes_written(&mut self, bytes: u64) {
        self.profile.bytes_written = bytes;
    }

    pub fn set_cache_stats(&mut self, hits: u64, misses: u64) {
        self.profile.cache_hits = hits;
        self.profile.cache_misses = misses;
    }

    pub fn set_locks_acquired(&mut self, locks: u64) {
        self.profile.locks_acquired = locks;
    }

    pub fn build(mut self) -> QueryProfile {
        self.profile.total_execution_time =
            self.profile.parse_time + self.profile.optimize_time + self.profile.execution_time;
        self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_operator() {
        let child = PlanOperator::new(2, OperatorType::IndexScan)
            .with_estimates(100, 10.5)
            .with_actuals(95, Duration::from_millis(50));

        let parent = PlanOperator::new(1, OperatorType::Filter)
            .with_estimates(50, 15.0)
            .with_actuals(48, Duration::from_millis(75))
            .add_child(child);

        assert_eq!(parent.total_time(), Duration::from_millis(125));
        assert_eq!(parent.total_rows(), 143);
    }

    #[test]
    fn test_estimation_accuracy() {
        let op = PlanOperator::new(1, OperatorType::TableScan)
            .with_estimates(100, 10.0)
            .with_actuals(100, Duration::from_millis(50));

        assert_eq!(op.estimation_accuracy(), Some(1.0));

        let op2 = PlanOperator::new(1, OperatorType::TableScan)
            .with_estimates(100, 10.0)
            .with_actuals(200, Duration::from_millis(50));

        assert_eq!(op2.estimation_accuracy(), Some(0.5));
    }

    #[test]
    fn test_query_profile() {
        let mut profile = QueryProfile::new(1, "SELECT * FROM users");
        profile.set_timing(
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(100),
        );

assert_eq!(*profile.total_execution_time(), Duration::from_millis(130));

        profile.cache_hits = 80;
        profile.cache_misses = 20;
        assert_eq!(profile.cache_hit_ratio(), 0.8);
    }

    #[test]
    fn test_profiler() {
        let profiler = QueryProfiler::new(100);

        let profile = QueryProfile::new(1, "SELECT * FROM users");
        profiler.record_profile(profile);

        assert_eq!(profiler.get_recent_profiles(10).len(), 1);
        assert!(profiler.get_profile(1).is_some());
    }

    #[test]
    fn test_profile_builder() {
        let mut builder = ProfileBuilder::new(1, "SELECT * FROM users");

        builder.start_parse();
        std::thread::sleep(Duration::from_millis(10));
        builder.end_parse();

        builder.start_execution();
        std::thread::sleep(Duration::from_millis(20));
        builder.end_execution();

        builder.set_rows_returned(100);
        builder.set_cache_stats(80, 20);

        let profile = builder.build();
        assert_eq!(profile.rows_returned, 100);
        assert!(profile.parse_time >= Duration::from_millis(10));
    }
}
