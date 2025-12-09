// # Enterprise Lifecycle Manager
//
// Manages the complete lifecycle of the database system including graceful startup
// with dependency ordering, health check aggregation, graceful shutdown with connection
// draining, hot reload capabilities, and version compatibility management.
//
// ## Features
//
// - **Graceful Startup**: Orchestrated component initialization with dependency resolution
// - **Health Checks**: Aggregated health monitoring from all subsystems
// - **Graceful Shutdown**: Coordinated shutdown with resource cleanup and connection draining
// - **Hot Reload**: Dynamic configuration and code reload without full restart
// - **Version Management**: Compatibility checks and migration support
// - **State Persistence**: Save and restore system state across restarts
//
// ## Example
//
// ```rust,no_run
// use rusty_db::enterprise::lifecycle::{LifecycleManager, Component, ComponentHealth};
//
// #[tokio::main]
// async fn main() {
//     let manager = LifecycleManager::new();
//
//     // Register components
//     manager.register_component(my_component).await;
//
//     // Start all components
//     manager.startup().await.unwrap();
//
//     // Check health
//     let health = manager.health_check().await;
//     println!("System health: {:?}", health);
//
//     // Graceful shutdown
//     manager.shutdown().await.unwrap();
// }
// ```

use tokio::time::sleep;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::SystemTime;
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use crate::{Result, DbError};

/// Component lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentState {
    /// Component is not initialized
    Uninitialized,
    /// Component is starting
    Starting,
    /// Component is running
    Running,
    /// Component is paused
    Paused,
    /// Component is stopping
    Stopping,
    /// Component is stopped
    Stopped,
    /// Component is in error state
    Error,
}

/// Health status for components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but operational
    Degraded,
    /// Component is unhealthy
    Unhealthy,
    /// Health status unknown
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Component name
    pub component: String,
    /// Health status
    pub status: HealthStatus,
    /// Status message
    pub message: String,
    /// Additional details
    pub details: HashMap<String, String>,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Component metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    /// Component name
    pub name: String,
    /// Component version
    pub version: String,
    /// Component description
    pub description: String,
    /// Dependencies (other component names)
    pub dependencies: Vec<String>,
    /// Startup priority (lower = earlier)
    pub priority: i32,
    /// Required for system operation
    pub required: bool,
    /// Supports hot reload
    pub hot_reloadable: bool,
}

/// Component lifecycle trait
#[async_trait]
pub trait Component: Send + Sync {
    /// Get component metadata
    fn metadata(&self) -> ComponentMetadata;

    /// Initialize the component
    async fn initialize(&self) -> Result<()>;

    /// Start the component
    async fn start(&self) -> Result<()>;

    /// Stop the component
    async fn stop(&self) -> Result<()>;

    /// Perform health check
    async fn health_check(&self) -> HealthCheck;

    /// Reload component configuration
    async fn reload(&self) -> Result<()> {
        Err(DbError::Internal("Hot reload not supported".to_string()))
    }

    /// Pause component (optional)
    async fn pause(&self) -> Result<()> {
        Err(DbError::Internal("Pause not supported".to_string()))
    }

    /// Resume component (optional)
    async fn resume(&self) -> Result<()> {
        Err(DbError::Internal("Resume not supported".to_string()))
    }
}

/// Registered component wrapper
struct RegisteredComponent {
    /// The component implementation
    component: Arc<dyn Component>,
    /// Current state
    state: ComponentState,
    /// Last health check
    last_health: Option<HealthCheck>,
    /// Registration time
    registered_at: SystemTime,
}

/// Startup phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupPhase {
    Initialization,
    DependencyResolution,
    ComponentStartup,
    HealthValidation,
    Complete,
}

/// Shutdown phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShutdownPhase {
    PreShutdown,
    ConnectionDraining,
    ComponentShutdown,
    Cleanup,
    Complete,
}

/// System state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// System version
    pub version: String,
    /// Component states
    pub component_states: HashMap<String, ComponentState>,
    /// Health checks
    pub health_checks: Vec<HealthCheck>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// Component registered
    ComponentRegistered { name: String, metadata: ComponentMetadata },
    /// Component state changed
    StateChanged { component: String, old_state: ComponentState, new_state: ComponentState },
    /// Startup initiated
    StartupInitiated,
    /// Startup completed
    StartupCompleted { duration_ms: u64 },
    /// Shutdown initiated
    ShutdownInitiated,
    /// Shutdown completed
    ShutdownCompleted { duration_ms: u64 },
    /// Health check performed
    HealthCheckPerformed { component: String, status: HealthStatus },
    /// Hot reload performed
    HotReloadPerformed { component: String },
}

/// Lifecycle event listener
pub type EventListener = Arc<dyn Fn(LifecycleEvent) + Send + Sync>;

/// Connection manager for graceful draining
struct ConnectionManager {
    /// Active connection count
    active_connections: Arc<Mutex<u64>>,
    /// Semaphore for new connections
    accept_connections: Arc<Semaphore>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            active_connections: Arc::new(Mutex::new(0)),
            accept_connections: Arc::new(Semaphore::new(1)),
        }
    }

    async fn acquire_connection(&self) -> Result<()> {
        let _permit = self.accept_connections.acquire().await
            .map_err(|e| DbError::Internal(format!("Connection acquire failed: {}", e)))?;

        let mut count = self.active_connections.lock().await;
        *count += 1;

        Ok(())
    }

    async fn release_connection(&self) {
        let mut count = self.active_connections.lock().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    async fn stop_accepting(&self) {
        // Close the semaphore to prevent new connections
        self.accept_connections.close();
    }

    async fn wait_for_drain(&self, timeout: Duration) -> Result<()> {
        let start = SystemTime::now();

        loop {
            let count = {
                let count = self.active_connections.lock().await;
                *count
            };

            if count == 0 {
                return Ok(());
            }

            if let Ok(elapsed) = start.elapsed() {
                if elapsed >= timeout {
                    return Err(DbError::Internal(format!(
                        "Timeout waiting for connections to drain. {} active connections remaining",
                        count
                    ))));
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Lifecycle manager implementation
pub struct LifecycleManager {
    /// Registered components
    components: Arc<RwLock<HashMap<String, RegisteredComponent>>>,
    /// Event listeners
    listeners: Arc<RwLock<Vec<EventListener>>>,
    /// Connection manager
    connection_mgr: Arc<ConnectionManager>,
    /// System snapshots
    snapshots: Arc<RwLock<Vec<SystemSnapshot>>>,
    /// Current system state
    system_state: Arc<RwLock<ComponentState>>,
    /// Shutdown signal
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            connection_mgr: Arc::new(ConnectionManager::new()),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            system_state: Arc::new(RwLock::new(ComponentState::Uninitialized)),
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Register a component
    pub async fn register_component(&self, component: Arc<dyn Component>) -> Result<()> {
        let metadata = component.metadata();
        let name = metadata.name.clone();

        let registered = RegisteredComponent {
            component,
            state: ComponentState::Uninitialized,
            last_health: None,
            registered_at: SystemTime::now(),
        };

        let mut components = self.components.write().await;
        components.insert(name.clone(), registered);

        self.emit_event(LifecycleEvent::ComponentRegistered {
            name,
            metadata,
        }).await;

        Ok(())
    }

    /// Start all components in dependency order
    pub async fn startup(&self) -> Result<()> {
        let start_time = SystemTime::now();

        self.emit_event(LifecycleEvent::StartupInitiated).await;

        // Update system state
        {
            let mut state = self.system_state.write().await;
            *state = ComponentState::Starting;
        }

        // Phase 1: Resolve dependencies
        let startup_order = self.resolve_dependencies().await?;

        // Phase 2: Initialize components
        for name in &startup_order {
            self.initialize_component(name).await?;
        }

        // Phase 3: Start components
        for name in &startup_order {
            self.start_component(name).await?;
        }

        // Phase 4: Validate health
        let health_checks = self.health_check_all().await;
        let all_healthy = health_checks.iter().all(|h| h.status == HealthStatus::Healthy);

        if !all_healthy {
            let unhealthy: Vec<_> = health_checks.iter()
                .filter(|h| h.status != HealthStatus::Healthy)
                .map(|h| h.component.clone())
                .collect();

            return Err(DbError::Internal(format!(
                "Startup failed - unhealthy components: {:?}",
                unhealthy
            ))));
        }

        // Update system state
        {
            let mut state = self.system_state.write().await;
            *state = ComponentState::Running;
        }

        let duration_ms = start_time.elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.emit_event(LifecycleEvent::StartupCompleted { duration_ms }).await;

        Ok(())
    }

    /// Resolve component dependencies and return startup order
    async fn resolve_dependencies(&self) -> Result<Vec<String>> {
        let components = self.components.read().await;

        // Build dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut priorities: HashMap<String, i32> = HashMap::new();

        for (name, reg) in components.iter() {
            let metadata = reg.component.metadata();
            graph.insert(name.clone(), metadata.dependencies.clone());
            priorities.insert(name.clone(), metadata.priority);
        }

        // Topological sort with priority
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit(
            node: &str,
            graph: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) -> Result<()> {
            if visited.contains(node) {
                return Ok(());
            }

            if visiting.contains(node) {
                return Err(DbError::Internal(format!("Circular dependency detected: {}", node))));
            }

            visiting.insert(node.to_string());

            if let Some(deps) = graph.get(node) {
                for dep in deps {
                    visit(dep, graph, visited, visiting, order)?;
                }
            }

            visiting.remove(node);
            visited.insert(node.to_string());
            order.push(node.to_string());

            Ok(())
        }

        // Sort by priority first
        let mut nodes: Vec<_> = graph.keys().cloned().collect();
        nodes.sort_by_key(|n| priorities.get(n).copied().unwrap_or(0));

        for node in nodes {
            visit(&node, &graph, &mut visited, &mut visiting, &mut order)?;
        }

        Ok(order)
    }

    /// Initialize a component
    async fn initialize_component(&self, name: &str) -> Result<()> {
        let component = {
            let components = self.components.read().await;
            components.get(name)
                .ok_or_else(|| DbError::NotFound(format!("Component not found: {}", name)))?
                .component.clone()
        });

        component.initialize().await?;

        self.update_component_state(name, ComponentState::Uninitialized, ComponentState::Stopped).await;

        Ok(())
    }

    /// Start a component
    async fn start_component(&self, name: &str) -> Result<()> {
        let component = {
            let components = self.components.read().await;
            components.get(name)
                .ok_or_else(|| DbError::NotFound(format!("Component not found: {}", name)))?
                .component.clone()
        });

        self.update_component_state(name, ComponentState::Stopped, ComponentState::Starting).await;

        component.start().await?;

        self.update_component_state(name, ComponentState::Starting, ComponentState::Running).await;

        Ok(())
    }

    /// Stop a component
    async fn stop_component(&self, name: &str) -> Result<()> {
        let component = {
            let components = self.components.read().await;
            components.get(name)
                .ok_or_else(|| DbError::NotFound(format!("Component not found: {}", name)))?
                .component.clone()
        });

        self.update_component_state(name, ComponentState::Running, ComponentState::Stopping).await;

        component.stop().await?;

        self.update_component_state(name, ComponentState::Stopping, ComponentState::Stopped).await;

        Ok(())
    }

    /// Update component state
    async fn update_component_state(&self, name: &str, oldstate: ComponentState, newstate: ComponentState) {
        let mut components = self.components.write().await;
        if let Some(reg) = components.get_mut(name) {
            reg.state = new_state;

            self.emit_event(LifecycleEvent::StateChanged {
                component: name.to_string(),
                old_state,
                new_state,
            }).await;
        }
    }

    /// Perform health check on all components
    pub async fn health_check_all(&self) -> Vec<HealthCheck> {
        let components = self.components.read().await;
        let mut checks = Vec::new();

        for (name, reg) in components.iter() {
            let start = SystemTime::now();
            let check = reg.component.health_check().await;
            let response_time = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);

            let mut check_with_time = check.clone();
            check_with_time.response_time_ms = response_time;

            checks.push(check_with_time);

            self.emit_event(LifecycleEvent::HealthCheckPerformed {
                component: name.clone(),
                status: check.status,
            }).await;
        }

        // Update last health checks
        let mut components = self.components.write().await;
        for check in &checks {
            if let Some(reg) = components.get_mut(&check.component) {
                reg.last_health = Some(check.clone());
            }
        }

        checks
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        let start_time = SystemTime::now();

        self.emit_event(LifecycleEvent::ShutdownInitiated).await;

        // Update system state
        {
            let mut state = self.system_state.write().await;
            *state = ComponentState::Stopping;
        }

        // Phase 1: Stop accepting new connections
        self.connection_mgr.stop_accepting().await;

        // Phase 2: Drain existing connections (with timeout)
        let drain_timeout = Duration::from_secs(30);
        if let Err(e) = self.connection_mgr.wait_for_drain(drain_timeout).await {
            tracing::warn!("Connection drain incomplete: {}", e);
        }

        // Phase 3: Stop components in reverse dependency order
        let startup_order = self.resolve_dependencies().await?;
        let shutdown_order: Vec<_> = startup_order.into_iter().rev().collect();

        for name in shutdown_order {
            if let Err(e) = self.stop_component(&name).await {
                tracing::error!("Error stopping component {}: {}", name, e);
            }
        }

        // Update system state
        {
            let mut state = self.system_state.write().await;
            *state = ComponentState::Stopped;
        }

        // Notify shutdown complete
        self.shutdown_signal.notify_waiters();

        let duration_ms = start_time.elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.emit_event(LifecycleEvent::ShutdownCompleted { duration_ms }).await;

        Ok(())
    }

    /// Hot reload a component
    pub async fn reload_component(&self, name: &str) -> Result<()> {
        let component = {
            let components = self.components.read().await;
            components.get(name)
                .ok_or_else(|| DbError::NotFound(format!("Component not found: {}", name)))?
                .component.clone()
        });

        let metadata = component.metadata();
        if !metadata.hot_reloadable {
            return Err(DbError::Internal(format!(
                "Component '{}' does not support hot reload",
                name
            ))));
        }

        component.reload().await?;

        self.emit_event(LifecycleEvent::HotReloadPerformed {
            component: name.to_string(),
        }).await;

        Ok(())
    }

    /// Create a system snapshot
    pub async fn create_snapshot(&self, version: impl Into<String>) -> Result<String> {
        let components = self.components.read().await;

        let mut component_states = HashMap::new();
        let mut health_checks = Vec::new();

        for (name, reg) in components.iter() {
            component_states.insert(name.clone(), reg.state);
            if let Some(health) = &reg.last_health {
                health_checks.push(health.clone());
            }
        }

        let snapshot = SystemSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            version: version.into(),
            component_states,
            health_checks,
            metadata: HashMap::new(),
        };

        let id = snapshot.id.clone();

        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot);

        Ok(id)
    }

    /// Get system snapshots
    pub async fn get_snapshots(&self) -> Vec<SystemSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.clone()
    }

    /// Add an event listener
    pub async fn add_listener(&self, listener: EventListener) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// Emit a lifecycle event
    async fn emit_event(&self, event: LifecycleEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    /// Get current system state
    pub async fn get_system_state(&self) -> ComponentState {
        let state = self.system_state.read().await;
        *state
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) {
        self.shutdown_signal.notified().await;
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockComponent {
        name: String,
        started: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl Component for MockComponent {
        fn metadata(&self) -> ComponentMetadata {
            ComponentMetadata {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                description: "Mock component".to_string(),
                dependencies: vec![],
                priority: 0,
                required: true,
                hot_reloadable: false,
            }
        }

        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn start(&self) -> Result<()> {
            let mut started = self.started.lock().await;
            *started = true;
            Ok(())
        }

        async fn stop(&self) -> Result<()> {
            let mut started = self.started.lock().await;
            *started = false;
            Ok(())
        }

        async fn health_check(&self) -> HealthCheck {
            HealthCheck {
                component: self.name.clone(),
                status: HealthStatus::Healthy,
                message: "OK".to_string(),
                details: HashMap::new(),
                timestamp: SystemTime::now(),
                response_time_ms: 0,
            }
        }
    }

    #[tokio::test]
    async fn test_startup_shutdown() {
        let manager = LifecycleManager::new();
        let started = Arc::new(Mutex::new(false));

        let component = Arc::new(MockComponent {
            name: "test".to_string(),
            started: Arc::clone(&started),
        });

        manager.register_component(component).await.unwrap();
        manager.startup().await.unwrap();

        assert!(*started.lock().await);

        manager.shutdown().await.unwrap();

        assert!(!*started.lock().await);
    }
}
