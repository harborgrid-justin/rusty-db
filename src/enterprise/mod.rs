// # Enterprise Integration Layer
//
// This module provides enterprise-grade infrastructure for coordinating and integrating
// all RustyDB subsystems. It implements cross-cutting concerns, service orchestration,
// configuration management, and operational excellence patterns.
//
// ## Architecture Overview
//
// The enterprise layer acts as the nervous system of RustyDB, connecting all major
// subsystems through well-defined interfaces and providing essential enterprise features:
//
// ```text
// ┌─────────────────────────────────────────────────────────────────┐
// │                  Enterprise Integration Layer                   │
// ├─────────────────────────────────────────────────────────────────┤
// │                                                                 │
// │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
// │  │  Service Bus │  │   Config     │  │Feature Flags │         │
// │  │   (Routing)  │  │ (Management) │  │  (A/B Test)  │         │
// │  └──────────────┘  └──────────────┘  └──────────────┘         │
// │                                                                 │
// │  ┌──────────────┐  ┌──────────────┐                           │
// │  │  Lifecycle   │  │Cross-Cutting │                           │
// │  │  (Startup)   │  │  (Tracing)   │                           │
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
// ## Core Modules
//
// ### Service Bus (`service_bus`)
//
// Asynchronous message routing and event-driven architecture:
//
// - **Message Routing**: Pub/sub, request/reply, and fire-and-forget patterns
// - **Priority Queuing**: Critical operations bypass normal queue
// - **Dead Letter Queue**: Failed message capture and analysis
// - **Service Discovery**: Dynamic service registration and discovery
// - **Backpressure**: Automatic flow control to prevent overload
//
// ```rust,no_run
// use rusty_db::enterprise::service_bus::{ServiceBus, Message, MessagePriority};
//
// # async fn example() -> rusty_db::Result<()> {
// let bus = ServiceBus::new(1000);
//
// // Subscribe to transaction commit events
// let mut receiver = bus.subscribe("transaction.commit").await;
//
// // Publish high-priority message
// let msg = Message::new("transaction.commit", vec![1, 2, 3])
//     .with_priority(MessagePriority::High);
// bus.publish(msg).await?;
//
// // Receive message
// if let Some(msg) = receiver.recv().await {
//     println!("Transaction committed: {:?}", msg);
// }
// # Ok(())
// # }
// ```
//
// ### Configuration Management (`config`)
//
// Hierarchical, type-safe configuration with hot-reload:
//
// - **Hierarchical**: Configuration inheritance and overrides
// - **Dynamic Updates**: Hot-reload without restart
// - **Validation**: Schema-based validation with custom rules
// - **Encryption**: Secure storage of sensitive parameters
// - **Environments**: Dev, test, staging, and production profiles
//
// ```rust,no_run
// use rusty_db::enterprise::config::{ConfigManager, ConfigValue, Environment};
//
// # async fn example() -> rusty_db::Result<()> {
// let config = ConfigManager::new(Environment::Production);
//
// // Set and get configuration
// config.set("database.max_connections", ConfigValue::Integer(100)).await?;
// let value = config.get("database.max_connections").await?;
//
// // Watch for changes
// let mut watcher = config.watch("database.max_connections").await;
// tokio::spawn(async move {
//     while let Some(new_value) = watcher.recv().await {
//         println!("Config changed: {:?}", new_value);
//     }
// });
// # Ok(())
// # }
// ```
//
// ### Feature Flags (`feature_flags`)
//
// Runtime feature toggles and A/B testing:
//
// - **Runtime Toggles**: Enable/disable features without deployment
// - **A/B Testing**: Compare different implementations
// - **Gradual Rollout**: Percentage-based feature rollout
// - **Targeting**: User and group-based targeting
// - **Dependencies**: Define feature dependencies and conflicts
//
// ```rust,no_run
// use rusty_db::enterprise::feature_flags::{
//     FeatureFlagManager, Feature, EvaluationContext, RolloutStrategy
// };
//
// # async fn example() -> rusty_db::Result<()> {
// let manager = FeatureFlagManager::new();
//
// // Create feature with 10% rollout
// let feature = Feature::new("new_optimizer")
//     .with_description("New cost-based optimizer")
//     .with_rollout(RolloutStrategy::Percentage(10));
//
// manager.register(feature).await?;
//
// // Check if enabled for user
// let context = EvaluationContext::for_user("user123");
// if manager.is_enabled("new_optimizer", &context).await {
//     // Use new optimizer
// }
// # Ok(())
// # }
// ```
//
// ### Lifecycle Manager (`lifecycle`)
//
// Orchestrated component lifecycle management:
//
// - **Graceful Startup**: Dependency-ordered initialization
// - **Health Checks**: Aggregated health monitoring
// - **Graceful Shutdown**: Connection draining and cleanup
// - **Hot Reload**: Dynamic code and config reload
// - **State Snapshots**: System state persistence
//
// ```rust,no_run
// use rusty_db::enterprise::lifecycle::{LifecycleManager, Component};
// use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// let manager = LifecycleManager::new();
//
// // Register components
// // manager.register_component(component).await?;
//
// // Start all components in dependency order
// manager.startup().await?;
//
// // Check system health
// let health = manager.health_check_all().await;
// println!("Health: {:?}", health);
//
// // Graceful shutdown
// manager.shutdown().await?;
// # Ok(())
// # }
// ```
//
// ### Cross-Cutting Concerns (`cross_cutting`)
//
// Infrastructure patterns applied across all subsystems:
//
// - **Distributed Tracing**: Request correlation and span tracking
// - **Circuit Breaker**: Prevent cascading failures
// - **Rate Limiting**: Token bucket rate limiting
// - **Retry Logic**: Exponential backoff retries
// - **Bulkhead**: Resource isolation
//
// ```rust,no_run
// use rusty_db::enterprise::cross_cutting::{
//     CircuitBreaker, RateLimiter, RequestContext, TracingContext
// };
//
// # async fn example() -> rusty_db::Result<()> {
// // Circuit breaker
// let breaker = CircuitBreaker::new("external_api", 5, 60);
// let result = breaker.call(async {
//     // External call
//     Ok::<_, rusty_db::DbError>(42)
// }).await?;
//
// // Rate limiting
// let limiter = RateLimiter::new(100, 60); // 100 req/min
// if limiter.allow("user123").await {
//     // Process request
// }
//
// // Request context with tracing
// let ctx = RequestContext::new()
//     .with_user("user123")
//     .with_trace_id("trace-xyz");
// # Ok(())
// # }
// ```
//
// ## Integration Patterns
//
// ### Subsystem Coordination
//
// The enterprise layer coordinates major subsystems:
//
// ```rust,ignore
// // Example: Coordinated transaction commit
// async fn coordinated_commit(ctx: &RequestContext) -> Result<()> {
//     // Start distributed trace
//     ctx.tracing.start_span("transaction.commit").await;
//
//     // Check feature flag
//     if feature_flags.is_enabled("async_commit", &eval_ctx).await {
//         // Publish async commit event
//         bus.publish(Message::new("txn.commit.async", data)).await?;
//     } else {
//         // Synchronous commit
//         transaction_mgr.commit(txn_id).await?;
//     }
//
//     // Update metrics
//     ctx.tracing.event("commit.complete", metrics).await;
//     ctx.tracing.end_span().await;
//
//     Ok(())
// }
// ```
//
// ### Configuration-Driven Behavior
//
// ```rust,ignore
// // Example: Dynamic buffer pool sizing
// async fn adjust_buffer_pool(config: &ConfigManager) -> Result<()> {
//     let size = config.get("storage.buffer_pool_size").await?
//         .as_integer()
//         .unwrap_or(1000);
//
//     buffer_pool.resize(size as usize).await?;
//     Ok(())
// }
// ```
//
// ### Health Monitoring
//
// ```rust,ignore
// // Example: Aggregate health check
// async fn check_system_health(lifecycle: &LifecycleManager) -> HealthStatus {
//     let checks = lifecycle.health_check_all().await;
//
//     if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
//         HealthStatus::Healthy
//     } else if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
//         HealthStatus::Unhealthy
//     } else {
//         HealthStatus::Degraded
//     }
// }
// ```
//
// ## Best Practices
//
// ### 1. Use Service Bus for Loose Coupling
//
// Instead of direct subsystem calls, use the service bus for event-driven integration:
//
// ```rust,ignore
// // Good: Loose coupling via service bus
// bus.publish(Message::new("index.rebuild", index_data)).await?;
//
// // Avoid: Tight coupling
// index_manager.rebuild_index(index_id).await?;
// ```
//
// ### 2. Externalize Configuration
//
// Don't hardcode values - use configuration management:
//
// ```rust,ignore
// // Good: Configurable
// let timeout = config.get("query.timeout_secs").await?.as_integer()?;
//
// // Avoid: Hardcoded
// let timeout = 30;
// ```
//
// ### 3. Use Feature Flags for Gradual Rollout
//
// Roll out new features progressively:
//
// ```rust,ignore
// if feature_flags.is_enabled("new_feature", &context).await {
//     new_implementation().await?;
// } else {
//     old_implementation().await?;
// }
// ```
//
// ### 4. Implement Circuit Breakers for External Dependencies
//
// Protect against cascading failures:
//
// ```rust,ignore
// let breaker = CircuitBreaker::new("s3_storage", 5, 60);
// breaker.call(async {
//     s3_client.upload(data).await
// }).await?;
// ```
//
// ### 5. Always Use Request Context for Tracing
//
// Propagate context for end-to-end tracing:
//
// ```rust,ignore
// async fn process_query(ctx: &RequestContext, query: &str) -> Result<()> {
//     ctx.tracing.start_span("query.parse").await;
//     let ast = parse_query(query)?;
//     ctx.tracing.end_span().await;
//
//     ctx.tracing.start_span("query.execute").await;
//     let result = execute_query(&ast).await?;
//     ctx.tracing.end_span().await;
//
//     Ok(())
// }
// ```
//
// ## Performance Considerations
//
// - **Service Bus**: Uses bounded channels with backpressure to prevent memory exhaustion
// - **Config Manager**: Reads are lock-free via Arc<RwLock>, writes are serialized
// - **Feature Flags**: Evaluation is O(1) with in-memory hash maps
// - **Tracing**: Minimal overhead, async logging to avoid blocking
// - **Circuit Breaker**: Lock-free state checks with atomic operations where possible
//
// ## Thread Safety
//
// All components in this module are:
//
// - **Send + Sync**: Safe to share across threads
// - **Arc-wrapped**: Reference-counted for shared ownership
// - **RwLock/Mutex**: Interior mutability with appropriate locking
//
// ## Error Handling
//
// The enterprise layer uses RustyDB's unified error type (`DbError`) and provides:
//
// - **Automatic Retry**: Configurable retry with exponential backoff
// - **Circuit Breakers**: Fail fast when systems are down
// - **Graceful Degradation**: Fallback strategies for non-critical failures
//
// ## Testing Support
//
// All modules provide testing utilities:
//
// - **Feature Flag Overrides**: Force enable/disable for tests
// - **Mock Components**: Test lifecycle without real subsystems
// - **In-Memory Config**: No file I/O required for tests

// Module declarations
pub mod config;
pub mod cross_cutting;
pub mod feature_flags;
pub mod lifecycle;
pub mod service_bus;

// Re-export commonly used types for convenience
pub use service_bus::{
    BusStatistics, DeliveryMode, Message, MessageMetadata, MessagePriority, ServiceBus, ServiceInfo,
};

pub use config::{
    ConfigChange, ConfigManager, ConfigSchema, ConfigSnapshot, ConfigValue, Environment,
    ValidationRule,
};

pub use feature_flags::{
    ABTest, EvaluationContext, EvaluationResult, Feature, FeatureFlagManager, FeatureState,
    RolloutStrategy, Variant,
};

pub use lifecycle::{
    Component, ComponentMetadata, ComponentState, HealthCheck, HealthStatus, LifecycleEvent,
    LifecycleManager, SystemSnapshot,
};

pub use cross_cutting::{
    retry_with_backoff, Bulkhead, CircuitBreaker, CircuitState, ErrorHandler, RateLimiter,
    RecoveryStrategy, RequestContext, RetryPolicy, Span, TracingContext,
};

// Enterprise layer version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Initialize the enterprise layer with default settings
pub async fn initialize() -> crate::Result<EnterpriseRuntime> {
    EnterpriseRuntime::new().await
}

// Enterprise runtime that coordinates all enterprise components
pub struct EnterpriseRuntime {
    // Service bus for message routing
    pub service_bus: std::sync::Arc<ServiceBus>,
    // Configuration manager
    pub config: ConfigManager,
    // Feature flag manager
    pub feature_flags: FeatureFlagManager,
    // Lifecycle manager
    pub lifecycle: LifecycleManager,
}

impl EnterpriseRuntime {
    // Create a new enterprise runtime with all components initialized
    pub async fn new() -> crate::Result<Self> {
        // Detect environment from environment variable or default to production
        let env = std::env::var("RUSTYDB_ENV")
            .ok()
            .and_then(|e| Environment::from_str(&e))
            .unwrap_or(Environment::Production);

        Ok(Self {
            service_bus: ServiceBus::new(10000),
            config: ConfigManager::new(env),
            feature_flags: FeatureFlagManager::new(),
            lifecycle: LifecycleManager::new(),
        })
    }

    // Start the enterprise runtime
    pub async fn start(&self) -> crate::Result<()> {
        // Load configuration if file exists
        if let Ok(config_path) = std::env::var("RUSTYDB_CONFIG") {
            if let Err(e) = self.config.load_from_file(&config_path).await {
                tracing::warn!("Failed to load config from {}: {}", config_path, e);
            }
        }

        // Start lifecycle manager
        self.lifecycle.startup().await?;

        tracing::info!("Enterprise runtime started successfully");
        Ok(())
    }

    // Shutdown the enterprise runtime gracefully
    pub async fn shutdown(&self) -> crate::Result<()> {
        tracing::info!("Shutting down enterprise runtime...");

        // Shutdown lifecycle manager
        self.lifecycle.shutdown().await?;

        // Shutdown service bus
        self.service_bus.shutdown().await;

        tracing::info!("Enterprise runtime shutdown complete");
        Ok(())
    }

    // Get health status of all components
    pub async fn health_check(&self) -> Vec<HealthCheck> {
        self.lifecycle.health_check_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enterprise_runtime() {
        let runtime = EnterpriseRuntime::new().await.unwrap();

        // Test service bus
        let mut rx = runtime.service_bus.subscribe("test").await;
        runtime
            .service_bus
            .publish(Message::new("test", b"hello".to_vec()))
            .await
            .unwrap();

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.payload, b"hello");

        // Test config
        runtime
            .config
            .set("test.key", ConfigValue::String("value".into()))
            .await
            .unwrap();
        let val = runtime.config.get("test.key").await.unwrap();
        assert_eq!(val.as_string(), Some("value"));

        // Test feature flags
        let feature = Feature::new("test_feature").with_state(FeatureState::Enabled);
        runtime.feature_flags.register(feature).await.unwrap();

        let ctx = EvaluationContext::for_user("user1");
        assert!(runtime.feature_flags.is_enabled("test_feature", &ctx).await);
    }
}
