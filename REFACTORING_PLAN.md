# Module Refactoring Plan

This document outlines the refactoring strategy for large files (>800 LOC) into smaller, cohesive modules.

## Overview

**Target**: Split 6 large files into ~30 smaller, focused modules
**Goal**: Each module ≤ 800 LOC with single responsibility

---

## 1. src/api/monitoring.rs (2428 LOC → 4 modules)

### 1.1 src/api/monitoring/metrics.rs (~600 LOC)
**Responsibility**: Core metric types and collection
- `MetricId`, `CounterMetric`, `GaugeMetric`
- `HistogramMetric`, `SummaryMetric`
- `MetricType`, `Timer`
- Basic metric operations

### 1.2 src/api/monitoring/aggregation.rs (~500 LOC)
**Responsibility**: Metric aggregation and windowing
- `MetricAggregator`, `AggregationWindow`
- `AggregatedMetricPoint`, `MetricNamespace`
- Time-series aggregation logic
- Window-based calculations

### 1.3 src/api/monitoring/prometheus.rs (~600 LOC)
**Responsibility**: Prometheus integration
- Prometheus exposition format
- Remote write protocol
- Metric export and formatting
- Cardinality management

### 1.4 src/api/monitoring/health.rs (~500 LOC)
**Responsibility**: Health checks and alerting
- Health check system (liveness, readiness, startup)
- Alert definitions and routing
- Notification handlers
- Health status aggregation

### 1.5 src/api/monitoring/mod.rs (~200 LOC)
**Responsibility**: Public API and orchestration
- `MonitoringApi` main struct
- Re-exports from submodules
- High-level coordination

---

## 2. src/api/gateway.rs (2424 LOC → 5 modules)

### 2.1 src/api/gateway/config.rs (~400 LOC)
**Responsibility**: Gateway configuration
- `GatewayConfig`, `ServiceDiscoveryConfig`
- `RetryPolicy`, `CircuitBreakerConfig`
- Configuration validation

### 2.2 src/api/gateway/routing.rs (~500 LOC)
**Responsibility**: Request routing
- `Route`, `ApiRequest`, `ApiResponse`
- Path matching and rewriting
- Route resolution logic

### 2.3 src/api/gateway/load_balancing.rs (~400 LOC)
**Responsibility**: Load balancing strategies
- `LoadBalancingStrategy` enum implementations
- Round-robin, least-connections, weighted
- Health-aware distribution

### 2.4 src/api/gateway/transformation.rs (~500 LOC)
**Responsibility**: Request/response transformation
- `RequestTransform`, `ResponseTransform`
- `PathRewrite`, `QueryTransform`, `BodyTransform`
- Header manipulation

### 2.5 src/api/gateway/service_registry.rs (~400 LOC)
**Responsibility**: Service discovery and registry
- `ServiceRegistry`, `BackendService`
- `ServiceEndpoint`, `ServiceMetadata`
- Service health tracking

### 2.6 src/api/gateway/mod.rs (~200 LOC)
**Responsibility**: Gateway orchestration
- `ApiGateway` main struct
- Request lifecycle management
- Re-exports

---

## 3. src/pool/connection_pool.rs (2338 LOC → 5 modules)

### 3.1 src/pool/config.rs (~300 LOC)
**Responsibility**: Pool configuration
- `PoolConfig`, `PoolConfigBuilder`
- Configuration validation and defaults
- Strong typing for pool parameters

### 3.2 src/pool/connection.rs (~500 LOC)
**Responsibility**: Connection management
- `PooledConnection`, `PooledConnectionGuard`
- Connection lifecycle
- Connection validation

### 3.3 src/pool/recycling.rs (~600 LOC)
**Responsibility**: Connection recycling
- `RecyclingManager`, `RecyclingStrategy`
- `AgingPolicy`, `StateResetManager`
- Connection refresh logic

### 3.4 src/pool/cache.rs (~400 LOC)
**Responsibility**: Statement and cursor caching
- `StatementCache`, `CursorCache`
- Cache eviction policies
- LRU implementation

### 3.5 src/pool/pool_core.rs (~700 LOC)
**Responsibility**: Pool core logic
- `ConnectionPool<C>` main implementation
- Acquire/release operations
- Pool sizing and scaling

### 3.6 src/pool/mod.rs (~200 LOC)
**Responsibility**: Public API
- Re-exports
- `PoolError` enum
- Helper functions

---

## 4. src/api/enterprise_integration.rs (2307 LOC → 5 modules)

### 4.1 src/api/enterprise/service_registry.rs (~500 LOC)
**Responsibility**: Service registration and discovery
- `ServiceRegistry`, `ServiceRegistration`
- `ServiceMetadata`, `ServiceState`
- Service lifecycle management

### 4.2 src/api/enterprise/dependency_injection.rs (~400 LOC)
**Responsibility**: DI container
- `DependencyContainer`
- Lifetime management
- Interface registration

### 4.3 src/api/enterprise/feature_flags.rs (~400 LOC)
**Responsibility**: Feature flag management
- `FeatureFlagManager`, `FeatureFlag`
- `FlagCondition`, conditional evaluation
- A/B testing support

### 4.4 src/api/enterprise/versioning.rs (~300 LOC)
**Responsibility**: Version compatibility
- `VersionCompatibilityChecker`
- `VersionConstraint`
- Semantic version parsing

### 4.5 src/api/enterprise/config_aggregation.rs (~400 LOC)
**Responsibility**: Configuration management
- `ConfigurationAggregator`
- Multi-source config merging
- Environment overrides

### 4.6 src/api/enterprise/event_bus.rs (~500 LOC)
**Responsibility**: Service event bus
- `ServiceEventBus`, `ServiceEvent`
- Event subscription and publishing
- Async event handling

### 4.7 src/api/enterprise/mod.rs (~200 LOC)
**Responsibility**: Module coordination
- Re-exports
- Enterprise integration facade

---

## 5. src/security/auto_recovery.rs (1670 LOC → 3 modules)

### 5.1 src/security/recovery/detection.rs (~600 LOC)
**Responsibility**: Failure detection
- `CrashDetector`, `CorruptionDetector`
- `DetectedFailure`, failure classification
- Health monitoring

### 5.2 src/security/recovery/strategies.rs (~600 LOC)
**Responsibility**: Recovery strategies
- `RecoveryStrategy` enum
- `RecoveryPlan`, `RecoveryResult`
- Strategy selection logic

### 5.3 src/security/recovery/transaction_rollback.rs (~500 LOC)
**Responsibility**: Transaction rollback
- `TransactionRollbackManager`
- `TransactionState`, `TransactionOperation`
- Rollback execution

### 5.4 src/security/recovery/mod.rs (~200 LOC)
**Responsibility**: Recovery orchestration
- `AutoRecoveryManager`
- RTO/RPO tracking
- Re-exports

---

## 6. src/security/security_core.rs (1631 LOC → 3 modules)

### 6.1 src/security/core/policy_engine.rs (~700 LOC)
**Responsibility**: Policy evaluation
- `SecurityPolicyEngine`
- `SecurityPolicy`, `PolicyRule`
- `PolicyDecision`, evaluation logic

### 6.2 src/security/core/defense_orchestrator.rs (~500 LOC)
**Responsibility**: Defense-in-depth
- `DefenseOrchestrator`
- `DefenseLayer`, layer coordination
- Threat level management

### 6.3 src/security/core/event_correlation.rs (~500 LOC)
**Responsibility**: Security event correlation
- `SecurityEventCorrelator`
- `CorrelatedEvent`, `AttackPattern`
- Pattern matching and detection

### 6.4 src/security/core/mod.rs (~200 LOC)
**Responsibility**: Security core coordination
- Re-exports
- Unified security interface

---

## Implementation Strategy

### Phase 1: Create Module Structure
1. Create subdirectories and mod.rs files
2. Define public interfaces in mod.rs

### Phase 2: Extract Code
1. Move structs, enums, and impls to appropriate modules
2. Update imports in moved code
3. Add proper documentation

### Phase 3: Update Parent Modules
1. Update mod.rs with re-exports
2. Ensure backward compatibility with `pub use`

### Phase 4: Testing
1. Run `cargo check` after each module
2. Fix import errors
3. Verify no functionality changes

### Phase 5: Documentation
1. Add module-level rustdoc
2. Document examples for each module
3. Update architecture docs

---

## Benefits

1. **Maintainability**: Easier to understand and modify individual modules
2. **Testability**: Smaller units are easier to test in isolation
3. **Collaboration**: Reduced merge conflicts with focused modules
4. **Performance**: Faster compilation with smaller compilation units
5. **Clarity**: Clear separation of concerns and responsibilities

---

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Files > 800 LOC | 6 | 0 |
| Average file size | 2133 LOC | ~450 LOC |
| Total modules | 6 | ~30 |
| Max file size | 2428 LOC | ~700 LOC |
