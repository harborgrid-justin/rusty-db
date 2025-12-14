// # RustyDB Orchestration Framework
//
// This module provides the core orchestration framework for RustyDB, coordinating all
// enterprise modules and providing critical infrastructure for building a robust,
// fault-tolerant, and scalable database system.
//
// ## Overview
//
// The orchestration framework is the nervous system of RustyDB, providing:
//
// - **Actor-Based Coordination**: Asynchronous message passing between components
// - **Service Registry**: Dependency injection and service discovery
// - **Dependency Management**: Automatic dependency resolution and initialization
// - **Circuit Breakers**: Fault tolerance and cascading failure prevention
// - **Health Monitoring**: Comprehensive health checks and aggregation
// - **Plugin System**: Extensible architecture for custom functionality
// - **Graceful Degradation**: Maintain availability under resource constraints
// - **Error Recovery**: Automatic retry, fallback, and compensation
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────────┐
// │                  Orchestration Framework                        │
// ├─────────────────────────────────────────────────────────────────┤
// │                                                                 │
// │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
// │  │Actor System  │  │   Service    │  │  Dependency  │         │
// │  │              │  │   Registry   │  │    Graph     │         │
// │  └──────────────┘  └──────────────┘  └──────────────┘         │
// │                                                                 │
// │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
// │  │Circuit       │  │   Health     │  │   Plugin     │         │
// │  │Breaker       │  │ Aggregator   │  │  Registry    │         │
// │  └──────────────┘  └──────────────┘  └──────────────┘         │
// │                                                                 │
// │  ┌──────────────┐  ┌──────────────┐                           │
// │  │ Degradation  │  │   Error      │                           │
// │  │  Strategy    │  │  Recovery    │                           │
// │  └──────────────┘  └──────────────┘                           │
// │                                                                 │
// └─────────────────────────────────────────────────────────────────┘
//           │           │           │           │           │
//           ▼           ▼           ▼           ▼           ▼
// ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
// │ Storage  │  │  Query   │  │Security  │  │Clustering│  │Analytics │
// │  Layer   │  │  Engine  │  │  Layer   │  │  Layer   │  │  Layer   │
// └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘
// ```
//
// ## Quick Start
//
// ```rust,no_run
// use rusty_db::orchestration::{Orchestrator, OrchestratorConfig};
//
// #[tokio::main]
// async fn main() -> rusty_db::Result<()> {
//     // Create orchestrator
//     let orchestrator = Orchestrator::new(OrchestratorConfig::default()).await?;
//
//     // Start all components
//     orchestrator.start().await?;
//
//     // Check system health
//     let health = orchestrator.health_check().await;
//     println!("System health: {:?}", health.status);
//
//     // Graceful shutdown
//     orchestrator.shutdown().await?;
//
//     Ok(())
// }
// ```
//
// ## Module Organization
//
// ### Core Modules
//
// - **`actor`**: Actor-based coordination with supervision trees
// - **`registry`**: Service registry and dependency injection
// - **`dependency_graph`**: Dependency resolution and topological sorting
//
// ### Resilience Modules
//
// - **`circuit_breaker`**: Circuit breaker pattern implementation
// - **`health`**: Health checking and aggregation
// - **`degradation`**: Graceful degradation strategies
// - **`error_recovery`**: Unified error handling and recovery
//
// ### Extensibility Modules
//
// - **`plugin`**: Plugin architecture for extensibility
//
// ## Usage Examples
//
// ### Actor-Based Communication
//
// ```rust,no_run
// use rusty_db::orchestration::actor::{ActorSystem, Actor, ActorContext};
//
// struct MyActor;
//
// #[async_trait::async_trait]
// impl Actor for MyActor {
//     async fn handle(&mut self, msg: Box<dyn std::any::Any + Send>, ctx: &ActorContext) -> rusty_db::Result<()> {
//         // Handle message
//         Ok(())
//     }
// }
//
// async fn example() -> rusty_db::Result<()> {
//     let system = ActorSystem::new();
//     let actor_ref = system.spawn(MyActor, Some("my-actor".into()), 100).await?;
//
//     actor_ref.send("Hello".to_string()).await?;
//
//     system.shutdown().await?;
//     Ok(())
// }
// ```
//
// ### Service Registry
//
// ```rust,no_run
// use rusty_db::orchestration::registry::{ServiceRegistry, ServiceMetadata, ServiceLifetime};
//
// async fn example() -> rusty_db::Result<()> {
//     let registry = ServiceRegistry::new();
//
//     // Register a service
//     let metadata = ServiceMetadata::new("my-service".into(), "MyService", ServiceLifetime::Singleton);
//     registry.register::<String, _>(
//         "my-service".into(),
//         |_| Ok("service instance".to_string()),
//         metadata
//     )?;
//
//     // Resolve the service
//     let instance = registry.resolve::<String>()?;
//
//     Ok(())
// }
// ```
//
// ### Circuit Breaker
//
// ```rust,no_run
// use rusty_db::orchestration::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//
// async fn example() -> rusty_db::Result<()> {
//     let breaker = CircuitBreaker::new("my-service".into(), CircuitBreakerConfig::default());
//
//     let result = breaker.call(async {
//         // Your operation here
//         Ok::<_, rusty_db::DbError>(42)
//     }).await?;
//
//     Ok(())
// }
// ```
//
// ## Best Practices
//
// ### 1. Use Dependency Injection
//
// Register all services in the service registry rather than creating tight coupling:
//
// ```rust,ignore
// // Good: Loose coupling via registry
// let storage = registry.resolve::<StorageEngine>()?;
//
// // Avoid: Tight coupling
// let storage = StorageEngine::new();
// ```
//
// ### 2. Implement Circuit Breakers for External Dependencies
//
// Wrap all external calls with circuit breakers:
//
// ```rust,ignore
// let breaker = circuit_breaker_registry.get_or_create("external_api");
// let result = breaker.call(|| external_api_call()).await?;
// ```
//
// ### 3. Define Health Checks for All Components
//
// Every major component should implement health checks:
//
// ```rust,ignore
// #[async_trait::async_trait]
// impl HealthCheck for MyComponent {
//     async fn check_health(&self) -> HealthCheckResult {
//         // Check component health
//         HealthCheckResult::healthy("my-component".into())
//     }
//
//     fn component_name(&self) -> &str {
//         "my-component"
//     }
// }
// ```
//
// ### 4. Use Actor Model for Concurrent Components
//
// For components that need concurrent access, use actors:
//
// ```rust,ignore
// let actor_system = ActorSystem::new();
// let worker = actor_system.spawn(WorkerActor::new(), Some("worker".into()), 100).await?;
// worker.send(WorkMessage::Process(data)).await?;
// ```
//
// ### 5. Enable Graceful Degradation
//
// Configure degradation triggers to maintain availability under load:
//
// ```rust,ignore
// let strategy = DegradationStrategy::new();
// let trigger = DegradationTrigger::new("high_load".into())
//     .with_cpu_threshold(0.8)
//     .with_memory_threshold(0.9);
// strategy.register_trigger(DegradationLevel::DegradedL1, trigger);
// ```
//
// ## Integration with Enterprise Layer
//
// The orchestration framework integrates seamlessly with RustyDB's enterprise layer:
//
// ```rust,ignore
// use rusty_db::enterprise::EnterpriseRuntime;
// use rusty_db::orchestration::Orchestrator;
//
// async fn integrated_startup() -> Result<()> {
//     // Create orchestrator
//     let orchestrator = Orchestrator::new(OrchestratorConfig::default()).await?;
//
//     // Create enterprise runtime
//     let enterprise = EnterpriseRuntime::new().await?;
//
//     // Register enterprise services with orchestrator
//     orchestrator.register_service_bus(enterprise.service_bus.clone());
//     orchestrator.register_config_manager(enterprise.config.clone());
//
//     // Start both
//     enterprise.start().await?;
//     orchestrator.start().await?;
//
//     Ok(())
// }
// ```
//
// ## Performance Considerations
//
// - **Actor System**: Lock-free message passing with bounded channels
// - **Service Registry**: Arc-based sharing with RwLock for concurrent access
// - **Circuit Breakers**: Atomic operations for state checks
// - **Health Checks**: Configurable intervals to balance accuracy and overhead
// - **Dependency Graph**: O(V + E) topological sort, cached results
//
// ## Thread Safety
//
// All components are designed to be thread-safe:
//
// - **Send + Sync**: All public types implement Send and Sync
// - **Arc Wrapping**: Shared ownership through Arc
// - **Interior Mutability**: RwLock and Mutex for safe mutation
// - **Atomic Operations**: Lock-free counters and flags
//
// ## Error Handling
//
// The framework uses RustyDB's unified error type and provides:
//
// - **Error Classification**: Automatic categorization of errors
// - **Automatic Retry**: Configurable retry with exponential backoff
// - **Fallback Strategies**: Multiple fallback options
// - **Compensation**: Undo operations on failure

// Module declarations
pub mod actor;
pub mod circuit_breaker;
pub mod degradation;
pub mod dependency_graph;
pub mod error_recovery;
pub mod health;
pub mod plugin;
pub mod registry;

// Re-export commonly used types
pub use actor::{
    Actor, ActorContext, ActorId, ActorRef, ActorSystem, ActorSystemStats, Message,
    SupervisionStrategy, SupervisorConfig,
};

pub use registry::{
    RegistryStatistics, Service, ServiceContainer, ServiceFactory, ServiceLifetime,
    ServiceMetadata, ServiceRegistry, ServiceScope,
};

pub use dependency_graph::{
    DependencyEdge, DependencyGraph, DependencyNode, DependencyType, GraphStatistics,
};

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitBreakerStats, CircuitState,
};

pub use health::{
    AggregatedHealth, CascadingFailureDetector, HealthAggregator, HealthCheck, HealthCheckResult,
    HealthChecker, HealthStatus, SimpleHealthCheck,
};

pub use plugin::{
    Plugin, PluginConfig, PluginContext, PluginEvent, PluginEventBus, PluginInfo, PluginMetadata,
    PluginRegistry, PluginState,
};

pub use degradation::{
    DegradationLevel, DegradationStrategy, DegradationTrigger, Feature, LoadShedder, SystemMetrics,
};

pub use error_recovery::{
    ClassifiedError, ErrorCategory, ErrorClassifier, ErrorSeverity, RecoveryAction,
    RecoveryManager, RetryConfig, RetryExecutor,
};

use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

use crate::error::Result;

// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    // Actor system mailbox size
    pub actor_mailbox_size: usize,
    // Maximum health history
    pub max_health_history: usize,
    // Retry configuration
    pub retry_config: RetryConfig,
    // Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,
    // Enable auto-recovery
    pub auto_recovery: bool,
    // Enable graceful degradation
    pub graceful_degradation: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            actor_mailbox_size: 1000,
            max_health_history: 1000,
            retry_config: RetryConfig::default(),
            circuit_breaker_config: CircuitBreakerConfig::default(),
            auto_recovery: true,
            graceful_degradation: true,
        }
    }
}

// Main orchestrator coordinating all subsystems
pub struct Orchestrator {
    // Configuration
    #[allow(dead_code)]
    config: OrchestratorConfig,
    // Actor system
    actor_system: Arc<ActorSystem>,
    // Service registry
    service_registry: Arc<ServiceRegistry>,
    // Dependency graph
    dependency_graph: Arc<RwLock<DependencyGraph>>,
    // Circuit breaker registry
    circuit_breakers: Arc<CircuitBreakerRegistry>,
    // Health aggregator
    health_aggregator: Arc<HealthAggregator>,
    // Plugin registry
    plugin_registry: Arc<PluginRegistry>,
    // Degradation strategy
    degradation: Arc<DegradationStrategy>,
    // Recovery manager
    recovery: Arc<RecoveryManager>,
    // Orchestrator state
    state: RwLock<OrchestratorState>,
}

// Orchestrator state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum OrchestratorState {
    // Not yet initialized
    Uninitialized,
    // Initialized but not started
    Initialized,
    // Running
    Running,
    // Stopping
    Stopping,
    // Stopped
    Stopped,
}

impl Orchestrator {
    // Create a new orchestrator
    pub async fn new(config: OrchestratorConfig) -> Result<Arc<Self>> {
        info!("Creating orchestrator with configuration: {:?}", config);

        let actor_system = ActorSystem::new();
        let service_registry = ServiceRegistry::new();
        let dependency_graph = Arc::new(RwLock::new(DependencyGraph::new()));
        let circuit_breakers = Arc::new(CircuitBreakerRegistry::new(
            config.circuit_breaker_config.clone(),
        ));
        let health_aggregator = Arc::new(HealthAggregator::new(config.max_health_history));
        let plugin_registry = Arc::new(PluginRegistry::new());
        let degradation = Arc::new(DegradationStrategy::new());
        let recovery = Arc::new(RecoveryManager::new(config.retry_config.clone()));

        let orchestrator = Arc::new(Self {
            config,
            actor_system,
            service_registry,
            dependency_graph,
            circuit_breakers,
            health_aggregator,
            plugin_registry,
            degradation,
            recovery,
            state: RwLock::new(OrchestratorState::Initialized),
        });

        info!("Orchestrator created successfully");
        Ok(orchestrator)
    }

    // Start the orchestrator
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        let mut state = self.state.write();
        if *state == OrchestratorState::Running {
            warn!("Orchestrator already running");
            return Ok(());
        }

        info!("Starting orchestrator...");

        // Update state
        *state = OrchestratorState::Running;
        drop(state);

        // Initialize services in dependency order
        self.service_registry.initialize_all()?;

        // Start plugins
        self.plugin_registry.initialize_all().await?;
        self.plugin_registry.start_all().await?;

        info!("Orchestrator started successfully");
        Ok(())
    }

    // Shutdown the orchestrator
    pub async fn shutdown(self: &Arc<Self>) -> Result<()> {
        let mut state = self.state.write();
        if *state == OrchestratorState::Stopped {
            warn!("Orchestrator already stopped");
            return Ok(());
        }

        info!("Shutting down orchestrator...");
        *state = OrchestratorState::Stopping;
        drop(state);

        // Stop plugins
        self.plugin_registry.stop_all().await?;

        // Shutdown services
        self.service_registry.shutdown_all()?;

        // Shutdown actor system
        self.actor_system.shutdown().await?;

        // Update state
        *self.state.write() = OrchestratorState::Stopped;

        info!("Orchestrator shutdown complete");
        Ok(())
    }

    // Get actor system
    pub fn actor_system(&self) -> &Arc<ActorSystem> {
        &self.actor_system
    }

    // Get service registry
    pub fn service_registry(&self) -> &Arc<ServiceRegistry> {
        &self.service_registry
    }

    // Get dependency graph
    pub fn dependency_graph(&self) -> &Arc<RwLock<DependencyGraph>> {
        &self.dependency_graph
    }

    // Get circuit breaker registry
    pub fn circuit_breakers(&self) -> &Arc<CircuitBreakerRegistry> {
        &self.circuit_breakers
    }

    // Get health aggregator
    pub fn health_aggregator(&self) -> &Arc<HealthAggregator> {
        &self.health_aggregator
    }

    // Get plugin registry
    pub fn plugin_registry(&self) -> &Arc<PluginRegistry> {
        &self.plugin_registry
    }

    // Get degradation strategy
    pub fn degradation(&self) -> &Arc<DegradationStrategy> {
        &self.degradation
    }

    // Get recovery manager
    pub fn recovery(&self) -> &Arc<RecoveryManager> {
        &self.recovery
    }

    // Check overall system health
    pub async fn health_check(&self) -> AggregatedHealth {
        self.health_aggregator.check_all().await
    }

    // Get orchestrator statistics
    pub async fn statistics(&self) -> OrchestratorStatistics {
        OrchestratorStatistics {
            state: *self.state.read(),
            actor_stats: self.actor_system.statistics().await,
            registry_stats: self.service_registry.statistics(),
            health_stats: self.health_aggregator.statistics(),
            degradation_stats: self.degradation.statistics(),
        }
    }

    // Get current state
    #[allow(private_interfaces)]
    pub fn state(&self) -> OrchestratorState {
        *self.state.read()
    }

    // Check if orchestrator is running
    pub fn is_running(&self) -> bool {
        *self.state.read() == OrchestratorState::Running
    }
}

// Orchestrator statistics
#[derive(Debug, Clone)]
pub struct OrchestratorStatistics {
    // Current state
    #[allow(private_interfaces)]
    pub state: OrchestratorState,
    // Actor system statistics
    pub actor_stats: ActorSystemStats,
    // Service registry statistics
    pub registry_stats: RegistryStatistics,
    // Health aggregator statistics
    pub health_stats: health::HealthAggregatorStats,
    // Degradation statistics
    pub degradation_stats: degradation::DegradationStats,
}

// Orchestrator version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_lifecycle() {
        let orchestrator = Orchestrator::new(OrchestratorConfig::default())
            .await
            .unwrap();

        assert_eq!(orchestrator.state(), OrchestratorState::Initialized);

        orchestrator.start().await.unwrap();
        assert_eq!(orchestrator.state(), OrchestratorState::Running);
        assert!(orchestrator.is_running());

        orchestrator.shutdown().await.unwrap();
        assert_eq!(orchestrator.state(), OrchestratorState::Stopped);
        assert!(!orchestrator.is_running());
    }

    #[tokio::test]
    async fn test_orchestrator_components() {
        let orchestrator = Orchestrator::new(OrchestratorConfig::default())
            .await
            .unwrap();

        // All components should be accessible
        let _ = orchestrator.actor_system();
        let _ = orchestrator.service_registry();
        let _ = orchestrator.circuit_breakers();
        let _ = orchestrator.health_aggregator();
        let _ = orchestrator.plugin_registry();
        let _ = orchestrator.degradation();
        let _ = orchestrator.recovery();
    }

    #[tokio::test]
    async fn test_orchestrator_statistics() {
        let orchestrator = Orchestrator::new(OrchestratorConfig::default())
            .await
            .unwrap();

        let stats = orchestrator.statistics().await;
        assert_eq!(stats.state, OrchestratorState::Initialized);
    }
}
