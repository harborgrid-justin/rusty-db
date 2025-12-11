// GraphQL Schema Builder
//
// Constructs the complete GraphQL schema

use async_graphql::Schema;

use super::queries::QueryRoot;
use super::mutations::MutationRoot;
use super::subscriptions::SubscriptionRoot;
use super::complexity::PerformanceExtension;

// ============================================================================
// SCHEMA BUILDER
// ============================================================================

// Build the complete GraphQL schema with secure defaults
pub fn build_schema() -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .extension(PerformanceExtension)
        .limit_depth(10)
        .limit_complexity(1000)
        .disable_introspection() // SECURITY: Disable introspection in production
        .finish()
}

// Create schema with custom configuration
pub fn build_schema_with_config(config: SchemaConfig) -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    let mut builder = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot);

    // Apply depth limit for DoS prevention
    if let Some(depth) = config.max_depth {
        builder = builder.limit_depth(depth);
    }

    // Apply complexity limit for DoS prevention
    if let Some(complexity) = config.max_complexity {
        builder = builder.limit_complexity(complexity);
    }

    // Add performance monitoring extension if enabled
    if config.enable_performance_extension {
        builder = builder.extension(PerformanceExtension);
    }

    // SECURITY: Disable introspection unless explicitly enabled
    // Introspection reveals the entire schema structure and is an information disclosure risk
    if !config.enable_introspection {
        builder = builder.disable_introspection();
    }

    builder.finish()
}

// Schema configuration
#[derive(Clone, Debug)]
pub struct SchemaConfig {
    pub max_depth: Option<usize>,
    pub max_complexity: Option<usize>,
    pub enable_performance_extension: bool,
    pub enable_tracing: bool,
    pub enable_introspection: bool,
    pub enable_playground: bool,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(10),
            max_complexity: Some(1000),
            enable_performance_extension: true,
            enable_tracing: false,
            // Secure defaults: introspection and playground disabled in production
            enable_introspection: false,
            enable_playground: false,
        }
    }
}
