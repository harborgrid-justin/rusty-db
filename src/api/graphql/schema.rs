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

// Build the complete GraphQL schema
pub fn build_schema() -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .extension(PerformanceExtension)
        .limit_depth(10)
        .limit_complexity(1000)
        .finish()
}

// Create schema with custom configuration
pub fn build_schema_with_config(config: SchemaConfig) -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    let mut builder = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot);

    if let Some(depth) = config.max_depth {
        builder = builder.limit_depth(depth);
    }

    if let Some(complexity) = config.max_complexity {
        builder = builder.limit_complexity(complexity);
    }

    if config.enable_performance_extension {
        builder = builder.extension(PerformanceExtension);
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
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(10),
            max_complexity: Some(1000),
            enable_performance_extension: true,
            enable_tracing: false,
        }
    }
}
