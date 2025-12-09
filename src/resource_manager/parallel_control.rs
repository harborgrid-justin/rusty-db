// Parallel Execution Control for Resource Management
//
// This module implements parallel degree limits, parallel statement queuing,
// auto DOP calculation, parallel downgrade, and cross-instance coordination.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::collections::HashSet;
use std::time::SystemTime;
use std::time::Instant;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

use crate::error::{Result, DbError};
use super::consumer_groups::ConsumerGroupId;
use super::session_control::SessionId;

/// Parallel query identifier
pub type ParallelQueryId = u64;

/// Degree of Parallelism
pub type DegreeOfParallelism = u32;

/// Server pool identifier
pub type ServerPoolId = u64;

/// Parallel execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelMode {
    /// No parallelism
    Serial,
    /// Manual parallelism (user-specified DOP)
    Manual,
    /// Automatic parallelism (system-determined DOP)
    Automatic,
    /// Adaptive parallelism (runtime adjustment)
    Adaptive,
}

/// Parallel server type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerType {
    /// Query coordinator
    Coordinator,
    /// Producer server (reads data)
    Producer,
    /// Consumer server (processes data)
    Consumer,
}

/// Parallel server state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerState {
    /// Server is idle and available
    Idle,
    /// Server is allocated but not yet running
    Allocated,
    /// Server is running a query
    Running,
    /// Server is in cleanup phase
    Cleanup,
}

/// Parallel query request
#[derive(Debug, Clone)]
pub struct ParallelQueryRequest {
    /// Query identifier
    pub query_id: ParallelQueryId,
    /// Session identifier
    pub session_id: SessionId,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Requested degree of parallelism
    pub requested_dop: DegreeOfParallelism,
    /// Minimum acceptable DOP
    pub min_dop: DegreeOfParallelism,
    /// Parallel mode
    pub mode: ParallelMode,
    /// Estimated cost
    pub estimated_cost: Option<f64>,
    /// Estimated cardinality
    pub estimated_rows: Option<u64>,
    /// Whether query can be downgraded
    pub allow_downgrade: bool,
    /// Queue priority
    pub priority: u32,
    /// Request time
    pub requested_at: Instant,
    /// Deadline for starting
    pub deadline: Option<Instant>,
}

/// Parallel query execution
#[derive(Debug, Clone)]
pub struct ParallelExecution {
    /// Query identifier
    pub query_id: ParallelQueryId,
    /// Session identifier
    pub session_id: SessionId,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Granted degree of parallelism
    pub granted_dop: DegreeOfParallelism,
    /// Requested DOP (may be different)
    pub requested_dop: DegreeOfParallelism,
    /// Allocated servers
    pub allocated_servers: Vec<ServerId>,
    /// Start time
    pub started_at: Instant,
    /// Execution state
    pub state: ExecutionState,
    /// Number of rows processed
    pub rows_processed: u64,
    /// Bytes processed
    pub bytes_processed: u64,
}

/// Server identifier
pub type ServerId = u64;

/// Execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    /// Queued, waiting for servers
    Queued,
    /// Initializing parallel servers
    Initializing,
    /// Actively executing
    Executing,
    /// Completing and cleaning up
    Completing,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// Parallel server
#[derive(Debug, Clone)]
pub struct ParallelServer {
    /// Server identifier
    pub server_id: ServerId,
    /// Server type
    pub server_type: ServerType,
    /// Current state
    pub state: ServerState,
    /// Assigned query (if any)
    pub assigned_query: Option<ParallelQueryId>,
    /// Pool this server belongs to
    pub pool_id: ServerPoolId,
    /// CPU affinity
    pub cpu_affinity: Option<usize>,
    /// Creation time
    pub created_at: SystemTime,
    /// Last used time
    pub last_used: Option<Instant>,
}

/// Parallel server pool
#[derive(Debug, Clone)]
pub struct ServerPool {
    /// Pool identifier
    pub pool_id: ServerPoolId,
    /// Pool name
    pub name: String,
    /// Minimum servers to maintain
    pub min_servers: u32,
    /// Maximum servers allowed
    pub max_servers: u32,
    /// Current active servers
    pub current_servers: u32,
    /// Idle servers available
    pub idle_servers: u32,
    /// Consumer groups assigned to this pool
    pub assigned_groups: HashSet<ConsumerGroupId>,
}

/// Auto DOP calculator
pub struct AutoDopCalculator {
    /// Number of CPU cores
    cpu_cores: u32,
    /// I/O subsystem parallelism capability
    io_parallelism: u32,
    /// Current system load
    system_load: Arc<RwLock<f64>>,
}

impl AutoDopCalculator {
    /// Create a new auto DOP calculator
    pub fn new(cpu_cores: u32, io_parallelism: u32) -> Self {
        Self {
            cpu_cores,
            io_parallelism,
            system_load: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Calculate automatic DOP for a query
    pub fn calculate_dop(
        &self,
        estimated_cost: Option<f64>,
        estimated_rows: Option<u64>,
        available_servers: u32,
    ) -> DegreeOfParallelism {
        // Base DOP on number of CPUs
        let mut dop = self.cpu_cores;

        // Adjust based on cost
        if let Some(cost) = estimated_cost {
            // Higher cost = more parallelism (up to a point)
            if cost < 1000.0 {
                dop = 1; // Too cheap, serial execution
            } else if cost < 10000.0 {
                dop = (self.cpu_cores / 4).max(2);
            } else if cost < 100000.0 {
                dop = (self.cpu_cores / 2).max(4);
            }
            // Very high cost: use full DOP
        }

        // Adjust based on system load
        let load = *self.system_load.read().unwrap();
        if load > 0.8 {
            dop = (dop / 2).max(1);
        } else if load > 0.6 {
            dop = (dop * 3 / 4).max(1);
        }

        // Don't exceed available servers
        dop = dop.min(available_servers);

        // Don't exceed I/O parallelism for I/O-bound queries
        dop = dop.min(self.io_parallelism);

        dop.max(1)
    }

    /// Update system load
    pub fn update_system_load(&self, load: f64) {
        *self.system_load.write().unwrap() = load.min(1.0).max(0.0);
    }
}

/// Parallel execution controller
pub struct ParallelExecutionController {
    /// Server pools
    server_pools: Arc<RwLock<HashMap<ServerPoolId, ServerPool>>>,
    /// All servers
    servers: Arc<RwLock<HashMap<ServerId, ParallelServer>>>,
    /// Active executions
    active_executions: Arc<RwLock<HashMap<ParallelQueryId, ParallelExecution>>>,
    /// Query queue (waiting for servers)
    query_queue: Arc<Mutex<VecDeque<ParallelQueryRequest>>>,
    /// Group DOP limits
    group_dop_limits: Arc<RwLock<HashMap<ConsumerGroupId, DegreeOfParallelism>>>,
    /// Auto DOP calculator
    auto_dop_calculator: Arc<AutoDopCalculator>,
    /// Maximum total DOP across all queries
    max_total_dop: DegreeOfParallelism,
    /// Current total DOP in use
    current_total_dop: Arc<RwLock<DegreeOfParallelism>>,
    /// Parallel downgrade enabled
    downgrade_enabled: bool,
    /// Next query ID
    next_query_id: Arc<RwLock<ParallelQueryId>>,
    /// Next server ID
    next_server_id: Arc<RwLock<ServerId>>,
    /// Statistics
    stats: Arc<RwLock<ParallelStats>>,
}

/// Parallel execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParallelStats {
    /// Total parallel queries executed
    pub total_parallel_queries: u64,
    /// Total serial queries (DOP=1)
    pub total_serial_queries: u64,
    /// Queries downgraded from requested DOP
    pub downgraded_queries: u64,
    /// Queries queued for servers
    pub queued_queries: u64,
    /// Average DOP granted
    pub avg_dop_granted: f64,
    /// Peak total DOP
    pub peak_total_dop: DegreeOfParallelism,
    /// Server utilization percentage
    pub server_utilization_pct: f64,
}

impl ParallelExecutionController {
    /// Create a new parallel execution controller
    pub fn new(cpu_cores: u32, io_parallelism: u32, max_total_dop: DegreeOfParallelism) -> Self {
        Self {
            server_pools: Arc::new(RwLock::new(HashMap::new())),
            servers: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            query_queue: Arc::new(Mutex::new(VecDeque::new())),
            group_dop_limits: Arc::new(RwLock::new(HashMap::new())),
            auto_dop_calculator: Arc::new(AutoDopCalculator::new(cpu_cores, io_parallelism)),
            max_total_dop,
            current_total_dop: Arc::new(RwLock::new(0)),
            downgrade_enabled: true,
            next_query_id: Arc::new(RwLock::new(1)),
            next_server_id: Arc::new(RwLock::new(1)),
            stats: Arc::new(RwLock::new(ParallelStats::default())),
        }
    }

    /// Create a server pool
    pub fn create_server_pool(
        &self,
        name: String,
        min_servers: u32,
        max_servers: u32,
    ) -> Result<ServerPoolId> {
        let pool_id = {
            let pools = self.server_pools.read().unwrap();
            pools.len() as ServerPoolId + 1
        };

        let pool = ServerPool {
            pool_id,
            name,
            min_servers,
            max_servers,
            current_servers: 0,
            idle_servers: 0,
            assigned_groups: HashSet::new(),
        };

        let mut pools = self.server_pools.write().unwrap();
        pools.insert(pool_id, pool);

        // Create minimum servers
        for _ in 0..min_servers {
            self.create_server(pool_id, ServerType::Producer)?;
        }

        Ok(pool_id)
    }

    /// Create a parallel server
    fn create_server(&self, pool_id: ServerPoolId, server_type: ServerType) -> Result<ServerId> {
        let server_id = {
            let mut next_id = self.next_server_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let server = ParallelServer {
            server_id,
            server_type,
            state: ServerState::Idle,
            assigned_query: None,
            pool_id,
            cpu_affinity: None,
            created_at: SystemTime::now(),
            last_used: None,
        };

        let mut servers = self.servers.write().unwrap();
        servers.insert(server_id, server);

        // Update pool
        let mut pools = self.server_pools.write().unwrap();
        if let Some(pool) = pools.get_mut(&pool_id) {
            pool.current_servers += 1;
            pool.idle_servers += 1;
        }

        Ok(server_id)
    }

    /// Request parallel execution
    pub fn request_parallel_execution(
        &self,
        session_id: SessionId,
        group_id: ConsumerGroupId,
        requested_dop: DegreeOfParallelism,
        mode: ParallelMode,
        estimated_cost: Option<f64>,
        estimated_rows: Option<u64>,
    ) -> Result<ParallelQueryId> {
        let query_id = {
            let mut next_id = self.next_query_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Calculate actual DOP
        let actual_dop = match mode {
            ParallelMode::Serial => 1,
            ParallelMode::Manual => requested_dop,
            ParallelMode::Automatic | ParallelMode::Adaptive => {
                let available = self.get_available_servers();
                self.auto_dop_calculator.calculate_dop(estimated_cost, estimated_rows, available)
            }
        };

        // Check group DOP limit
        let dop_limit = {
            let limits = self.group_dop_limits.read().unwrap();
            limits.get(&group_id).copied()
        };

        let final_dop = if let Some(limit) = dop_limit {
            actual_dop.min(limit)
        } else {
            actual_dop
        };

        let request = ParallelQueryRequest {
            query_id,
            session_id,
            group_id,
            requested_dop: final_dop,
            min_dop: if mode == ParallelMode::Manual { final_dop } else { 1 },
            mode,
            estimated_cost,
            estimated_rows,
            allow_downgrade: mode != ParallelMode::Manual,
            priority: 0,
            requested_at: Instant::now(),
            deadline: None,
        };

        // Try to allocate servers immediately
        if let Some(granted_dop) = self.try_allocate_servers(&request) {
            self.start_execution(query_id, session_id, group_id, granted_dop, final_dop)?;
            Ok(query_id)
        } else {
            // Queue for later
            let mut queue = self.query_queue.lock().unwrap();
            queue.push_back(request);

            let mut stats = self.stats.write().unwrap();
            stats.queued_queries += 1;

            Ok(query_id)
        }
    }

    /// Try to allocate servers for a request
    fn try_allocate_servers(&self, request: &ParallelQueryRequest) -> Option<DegreeOfParallelism> {
        let current_dop = *self.current_total_dop.read().unwrap();
        let available_dop = self.max_total_dop.saturating_sub(current_dop);

        if available_dop == 0 {
            return None;
        }

        // Try requested DOP first
        if request.requested_dop <= available_dop {
            return Some(request.requested_dop);
        }

        // If downgrade allowed, try lower DOPs
        if request.allow_downgrade && self.downgrade_enabled {
            let downgraded_dop = available_dop.min(request.requested_dop).max(request.min_dop);
            if downgraded_dop >= request.min_dop {
                return Some(downgraded_dop);
            }
        }

        None
    }

    /// Start parallel execution
    fn start_execution(
        &self,
        query_id: ParallelQueryId,
        session_id: SessionId,
        group_id: ConsumerGroupId,
        granted_dop: DegreeOfParallelism,
        requested_dop: DegreeOfParallelism,
    ) -> Result<()> {
        // Allocate servers
        let server_ids = self.allocate_servers(granted_dop)?;

        let execution = ParallelExecution {
            query_id,
            session_id,
            group_id,
            granted_dop,
            requested_dop,
            allocated_servers: server_ids,
            started_at: Instant::now(),
            state: ExecutionState::Executing,
            rows_processed: 0,
            bytes_processed: 0,
        };

        // Update state
        {
            let mut executions = self.active_executions.write().unwrap();
            executions.insert(query_id, execution);
        }

        {
            let mut current_dop = self.current_total_dop.write().unwrap();
            *current_dop += granted_dop;
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            if granted_dop > 1 {
                stats.total_parallel_queries += 1;
            } else {
                stats.total_serial_queries += 1;
            }

            if granted_dop < requested_dop {
                stats.downgraded_queries += 1;
            }

            let total_queries = stats.total_parallel_queries + stats.total_serial_queries;
            stats.avg_dop_granted = (stats.avg_dop_granted * (total_queries - 1) as f64
                + granted_dop as f64) / total_queries as f64;

            let current_dop = *self.current_total_dop.read().unwrap();
            stats.peak_total_dop = stats.peak_total_dop.max(current_dop);
        }

        Ok(())
    }

    /// Allocate servers from pool
    fn allocate_servers(&self, count: u32) -> Result<Vec<ServerId>> {
        let mut servers = self.servers.write().unwrap();
        let mut allocated = Vec::new();

        for server in servers.values_mut() {
            if server.state == ServerState::Idle && allocated.len() < count as usize {
                server.state = ServerState::Allocated;
                allocated.push(server.server_id);
            }
        }

        if allocated.len() < count as usize {
            // Rollback allocations
            for &server_id in &allocated {
                if let Some(server) = servers.get_mut(&server_id) {
                    server.state = ServerState::Idle;
                }
            }
            return Err(DbError::ResourceExhausted(
                "Not enough parallel servers available".to_string()
            ));
        }

        Ok(allocated)
    }

    /// Complete parallel execution
    pub fn complete_execution(&self, query_id: ParallelQueryId) -> Result<()> {
        let execution = {
            let mut executions = self.active_executions.write().unwrap();
            executions.remove(&query_id)
                .ok_or_else(|| DbError::NotFound(format!("Query {} not found", query_id)))?
        };

        // Free servers
        {
            let mut servers = self.servers.write().unwrap();
            for server_id in &execution.allocated_servers {
                if let Some(server) = servers.get_mut(server_id) {
                    server.state = ServerState::Idle;
                    server.assigned_query = None;
                    server.last_used = Some(Instant::now());
                }
            }
        }

        // Update total DOP
        {
            let mut current_dop = self.current_total_dop.write().unwrap();
            *current_dop = current_dop.saturating_sub(execution.granted_dop);
        }

        // Try to schedule queued queries
        self.process_queue()?;

        Ok(())
    }

    /// Process queued queries
    fn process_queue(&self) -> Result<()> {
        let mut queue = self.query_queue.lock().unwrap();

        while let Some(request) = queue.pop_front() {
            if let Some(granted_dop) = self.try_allocate_servers(&request) {
                drop(queue);
                self.start_execution(
                    request.query_id,
                    request.session_id,
                    request.group_id,
                    granted_dop,
                    request.requested_dop,
                )?;
                queue = self.query_queue.lock().unwrap();
            } else {
                // Put back and stop processing
                queue.push_front(request);
                break;
            }
        }

        Ok(())
    }

    /// Set group DOP limit
    pub fn set_group_dop_limit(
        &self,
        group_id: ConsumerGroupId,
        limit: DegreeOfParallelism,
    ) -> Result<()> {
        let mut limits = self.group_dop_limits.write().unwrap();
        limits.insert(group_id, limit);
        Ok(())
    }

    /// Get available servers
    fn get_available_servers(&self) -> u32 {
        let current_dop = *self.current_total_dop.read().unwrap();
        self.max_total_dop.saturating_sub(current_dop)
    }

    /// Get execution info
    pub fn get_execution(&self, query_id: ParallelQueryId) -> Option<ParallelExecution> {
        let executions = self.active_executions.read().unwrap();
        executions.get(&query_id).cloned()
    }

    /// Get statistics
    pub fn get_stats(&self) -> ParallelStats {
        self.stats.read().unwrap().clone()
    }

    /// Update system load (for auto DOP calculation)
    pub fn update_system_load(&self, load: f64) {
        self.auto_dop_calculator.update_system_load(load);
    }

    /// Enable/disable parallel downgrade
    pub fn set_downgrade_enabled(&mut self, enabled: bool) {
        self.downgrade_enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_controller_creation() {
        let controller = ParallelExecutionController::new(16, 32, 128);
        assert_eq!(controller.max_total_dop, 128);
    }

    #[test]
    fn test_auto_dop_calculation() {
        let calculator = AutoDopCalculator::new(16, 32);

        // Low cost query
        let dop = calculator.calculate_dop(Some(500.0), None, 32);
        assert_eq!(dop, 1);

        // High cost query
        let dop = calculator.calculate_dop(Some(500000.0), None, 32);
        assert!(dop > 1);
    }

    #[test]
    fn test_parallel_execution() {
        let controller = ParallelExecutionController::new(16, 32, 128);
        controller.create_server_pool("DEFAULT".to_string(), 16, 64).unwrap();

        let query_id = controller.request_parallel_execution(
            1,
            1,
            8,
            ParallelMode::Automatic,
            Some(100000.0),
            Some(1000000),
        ).unwrap();

        assert!(query_id > 0);
    }
}
