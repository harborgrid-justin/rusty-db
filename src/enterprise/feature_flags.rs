// # Feature Flag System
//
// Provides runtime feature toggles, A/B testing support for query optimization strategies,
// gradual rollout capabilities, and feature dependency/conflict resolution.
//
// ## Features
//
// - **Runtime Toggles**: Enable/disable features without redeployment
// - **A/B Testing**: Test different implementations with traffic splitting
// - **Gradual Rollout**: Progressive rollout with percentage-based targeting
// - **User Targeting**: Target features to specific users or groups
// - **Dependency Management**: Define feature dependencies and conflicts
// - **Metrics Collection**: Track feature usage and performance impact
//
// ## Example
//
// ```rust,no_run
// use rusty_db::enterprise::feature_flags::{FeatureFlagManager, Feature, RolloutStrategy};
//
// #[tokio::main]
// async fn main() {
//     let manager = FeatureFlagManager::new();
//
//     // Create a feature flag
//     let feature = Feature::new("new_query_optimizer")
//         .with_description("New cost-based optimizer")
//         .with_rollout(RolloutStrategy::Percentage(10)); // 10% rollout
//
//     manager.register(feature).await.unwrap();
//
//     // Check if feature is enabled
//     let context = Default::default();
//     if manager.is_enabled("new_query_optimizer", &context).await {
//         // Use new optimizer
//     }
// }
// ```

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{DbError, Result};

// Feature flag state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureState {
    // Feature is enabled for all users
    Enabled,
    // Feature is disabled for all users
    Disabled,
    // Feature is in conditional rollout
    Conditional,
}

// Rollout strategy for gradual feature deployment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RolloutStrategy {
    // Enable for all users
    All,
    // Disable for all users
    None,
    // Enable for specific percentage of users (0-100)
    Percentage(u8),
    // Enable for specific user IDs
    UserIds(HashSet<String>),
    // Enable for specific user groups
    Groups(HashSet<String>),
    // Enable based on custom attributes
    Attributes(HashMap<String, Vec<String>>),
    // Time-based rollout (start time, end time)
    TimeBased(SystemTime, SystemTime),
}

impl RolloutStrategy {
    // Check if rollout applies to the given context
    pub fn applies(&self, context: &EvaluationContext) -> bool {
        match self {
            RolloutStrategy::All => true,
            RolloutStrategy::None => false,
            RolloutStrategy::Percentage(pct) => Self::hash_percentage(&context.user_id) <= *pct,
            RolloutStrategy::UserIds(ids) => ids.contains(&context.user_id),
            RolloutStrategy::Groups(groups) => context.groups.iter().any(|g| groups.contains(g)),
            RolloutStrategy::Attributes(attrs) => attrs.iter().all(|(key, values)| {
                context
                    .attributes
                    .get(key)
                    .map(|v| values.contains(v))
                    .unwrap_or(false)
            }),
            RolloutStrategy::TimeBased(start, end) => {
                let now = SystemTime::now();
                now >= *start && now <= *end
            }
        }
    }

    // Calculate hash-based percentage for consistent bucketing
    fn hash_percentage(user_id: &str) -> u8 {
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        let result = hasher.finalize();
        // Use first byte for percentage (0-255 maps to 0-100)
        ((result[0] as u16 * 100) / 256) as u8
    }
}

// Evaluation context for feature flag decisions
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    // User identifier
    pub user_id: String,
    // User groups/roles
    pub groups: HashSet<String>,
    // Custom attributes
    pub attributes: HashMap<String, String>,
    // Session identifier
    pub session_id: Option<String>,
    // Request timestamp
    pub timestamp: Option<SystemTime>,
}

impl EvaluationContext {
    // Create a new context for a user
    pub fn for_user(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            groups: HashSet::new(),
            attributes: HashMap::new(),
            session_id: None,
            timestamp: Some(SystemTime::now()),
        }
    }

    // Add a group to the context
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.groups.insert(group.into());
        self
    }

    // Add an attribute to the context
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

// Feature flag definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    // Unique feature identifier
    pub id: String,
    // Feature name
    pub name: String,
    // Feature description
    pub description: String,
    // Current state
    pub state: FeatureState,
    // Rollout strategy
    pub rollout: RolloutStrategy,
    // Feature dependencies (must be enabled)
    pub dependencies: HashSet<String>,
    // Feature conflicts (cannot be enabled together)
    pub conflicts: HashSet<String>,
    // Feature tags for organization
    pub tags: HashSet<String>,
    // Feature owner
    pub owner: String,
    // Creation timestamp
    pub created_at: SystemTime,
    // Last updated timestamp
    pub updated_at: SystemTime,
    // Feature metadata
    pub metadata: HashMap<String, String>,
}

impl Feature {
    // Create a new feature flag
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            description: String::new(),
            state: FeatureState::Disabled,
            rollout: RolloutStrategy::None,
            dependencies: HashSet::new(),
            conflicts: HashSet::new(),
            tags: HashSet::new(),
            owner: "system".to_string(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    // Set feature description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    // Set feature state
    pub fn with_state(mut self, state: FeatureState) -> Self {
        self.state = state;
        self
    }

    // Set rollout strategy
    pub fn with_rollout(mut self, rollout: RolloutStrategy) -> Self {
        self.rollout = rollout;
        self.state = FeatureState::Conditional;
        self
    }

    // Add a dependency
    pub fn with_dependency(mut self, feature: impl Into<String>) -> Self {
        self.dependencies.insert(feature.into());
        self
    }

    // Add a conflict
    pub fn with_conflict(mut self, feature: impl Into<String>) -> Self {
        self.conflicts.insert(feature.into());
        self
    }

    // Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    // Set owner
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = owner.into();
        self
    }
}

// Feature flag evaluation result
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    // Whether the feature is enabled
    pub enabled: bool,
    // Reason for the decision
    pub reason: String,
    // Variant for A/B testing (if applicable)
    pub variant: Option<String>,
}

// Feature usage statistics
#[derive(Debug, Clone, Default)]
pub struct FeatureStats {
    // Total evaluations
    pub total_evaluations: u64,
    // Enabled count
    pub enabled_count: u64,
    // Disabled count
    pub disabled_count: u64,
    // Unique users evaluated
    pub unique_users: HashSet<String>,
    // Last evaluation time
    pub last_evaluated: Option<SystemTime>,
}

// A/B test variant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    // Variant name
    pub name: String,
    // Variant weight (for distribution)
    pub weight: u8,
    // Variant configuration
    pub config: HashMap<String, String>,
}

// A/B test definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    // Test identifier
    pub id: String,
    // Feature being tested
    pub feature_id: String,
    // Test name
    pub name: String,
    // Test variants
    pub variants: Vec<Variant>,
    // Test duration
    pub duration: Duration,
    // Test start time
    pub started_at: SystemTime,
}

impl ABTest {
    // Select a variant for the given user
    pub fn select_variant(&self, user_id: &str) -> Option<&Variant> {
        if self.variants.is_empty() {
            return None;
        }

        let total_weight: u32 = self.variants.iter().map(|v| v.weight as u32).sum();
        if total_weight == 0 {
            return self.variants.first();
        }

        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(self.id.as_bytes());
        let result = hasher.finalize();
        let hash_value = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);

        let bucket = (hash_value % total_weight) as u8;
        let mut cumulative = 0;

        for variant in &self.variants {
            cumulative += variant.weight;
            if bucket < cumulative {
                return Some(variant);
            }
        }

        self.variants.last()
    }
}

// Feature flag manager
pub struct FeatureFlagManager {
    // All registered features
    features: Arc<RwLock<HashMap<String, Feature>>>,
    // Feature usage statistics
    stats: Arc<RwLock<HashMap<String, FeatureStats>>>,
    // Active A/B tests
    ab_tests: Arc<RwLock<HashMap<String, ABTest>>>,
    // Overrides for testing (feature_id -> enabled)
    overrides: Arc<RwLock<HashMap<String, bool>>>,
}

impl FeatureFlagManager {
    // Create a new feature flag manager
    pub fn new() -> Self {
        Self {
            features: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            ab_tests: Arc::new(RwLock::new(HashMap::new())),
            overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Register a new feature
    pub async fn register(&self, feature: Feature) -> Result<()> {
        // Check for conflicts with existing features
        {
            let features = self.features.read().await;
            for conflict in &feature.conflicts {
                if let Some(conflicting) = features.get(conflict) {
                    if conflicting.state == FeatureState::Enabled {
                        return Err(DbError::InvalidInput(format!(
                            "Cannot register feature '{}' - conflicts with enabled feature '{}'",
                            feature.name, conflict
                        )));
                    }
                }
            }
        }

        let mut features = self.features.write().await;
        features.insert(feature.name.clone(), feature);

        Ok(())
    }

    // Unregister a feature
    pub async fn unregister(&self, name: &str) -> Result<()> {
        let mut features = self.features.write().await;
        features
            .remove(name)
            .ok_or_else(|| DbError::NotFound(format!("Feature not found: {}", name)))?;

        Ok(())
    }

    // Check if a feature is enabled for the given context
    pub async fn is_enabled(&self, name: &str, context: &EvaluationContext) -> bool {
        self.evaluate(name, context)
            .await
            .map(|r| r.enabled)
            .unwrap_or(false)
    }

    // Evaluate a feature flag
    pub fn evaluate<'a>(
        &'a self,
        name: &'a str,
        context: &'a EvaluationContext,
    ) -> BoxFuture<'a, Result<EvaluationResult>> {
        Box::pin(async move {
            // Check for overrides first (for testing)
            {
                let overrides = self.overrides.read().await;
                if let Some(&enabled) = overrides.get(name) {
                    return Ok(EvaluationResult {
                        enabled,
                        reason: "Override".to_string(),
                        variant: None,
                    });
                }
            }

            let features = self.features.read().await;
            let feature = features
                .get(name)
                .ok_or_else(|| DbError::NotFound(format!("Feature not found: {}", name)))?;

            // Check dependencies
            for dep in &feature.dependencies {
                if !self.is_enabled(dep, context).await {
                    self.record_evaluation(name, context, false).await;
                    return Ok(EvaluationResult {
                        enabled: false,
                        reason: format!("Dependency '{}' not enabled", dep),
                        variant: None,
                    });
                }
            }

            // Check conflicts
            for conflict in &feature.conflicts {
                if self.is_enabled(conflict, context).await {
                    self.record_evaluation(name, context, false).await;
                    return Ok(EvaluationResult {
                        enabled: false,
                        reason: format!("Conflicts with enabled feature '{}'", conflict),
                        variant: None,
                    });
                }
            }

            // Evaluate based on state and rollout
            let enabled = match feature.state {
                FeatureState::Enabled => true,
                FeatureState::Disabled => false,
                FeatureState::Conditional => feature.rollout.applies(context),
            };

            // Check for A/B test
            let variant = {
                let tests = self.ab_tests.read().await;
                tests
                    .values()
                    .find(|t| t.feature_id == name)
                    .and_then(|test| test.select_variant(&context.user_id))
                    .map(|v| v.name.clone())
            };

            self.record_evaluation(name, context, enabled).await;

            Ok(EvaluationResult {
                enabled,
                reason: format!("State: {:?}, Rollout applies: {}", feature.state, enabled),
                variant,
            })
        })
    }

    // Record feature evaluation for statistics
    async fn record_evaluation(&self, name: &str, context: &EvaluationContext, enabled: bool) {
        let mut stats = self.stats.write().await;
        let stat = stats
            .entry(name.to_string())
            .or_insert_with(FeatureStats::default);

        stat.total_evaluations += 1;
        if enabled {
            stat.enabled_count += 1;
        } else {
            stat.disabled_count += 1;
        }
        stat.unique_users.insert(context.user_id.clone());
        stat.last_evaluated = Some(SystemTime::now());
    }

    // Update a feature
    pub async fn update(&self, name: &str, updater: impl FnOnce(&mut Feature)) -> Result<()> {
        let mut features = self.features.write().await;
        let feature = features
            .get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Feature not found: {}", name)))?;

        updater(feature);
        feature.updated_at = SystemTime::now();

        Ok(())
    }

    // Enable a feature
    pub async fn enable(&self, name: &str) -> Result<()> {
        self.update(name, |f| f.state = FeatureState::Enabled).await
    }

    // Disable a feature
    pub async fn disable(&self, name: &str) -> Result<()> {
        self.update(name, |f| f.state = FeatureState::Disabled)
            .await
    }

    // Set feature rollout percentage
    pub async fn set_rollout_percentage(&self, name: &str, percentage: u8) -> Result<()> {
        if percentage > 100 {
            return Err(DbError::InvalidInput(
                "Percentage must be 0-100".to_string(),
            ));
        }

        self.update(name, |f| {
            f.rollout = RolloutStrategy::Percentage(percentage);
            f.state = FeatureState::Conditional;
        })
        .await
    }

    // Get feature by name
    pub async fn get(&self, name: &str) -> Option<Feature> {
        let features = self.features.read().await;
        features.get(name).cloned()
    }

    // Get all features
    pub async fn list(&self) -> Vec<Feature> {
        let features = self.features.read().await;
        features.values().cloned().collect()
    }

    // Get features by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<Feature> {
        let features = self.features.read().await;
        features
            .values()
            .filter(|f| f.tags.contains(tag))
            .cloned()
            .collect()
    }

    // Get feature statistics
    pub async fn get_stats(&self, name: &str) -> Option<FeatureStats> {
        let stats = self.stats.read().await;
        stats.get(name).cloned()
    }

    // Create an A/B test
    pub async fn create_ab_test(&self, test: ABTest) -> Result<()> {
        // Verify feature exists
        {
            let features = self.features.read().await;
            if !features.contains_key(&test.feature_id) {
                return Err(DbError::NotFound(format!(
                    "Feature not found: {}",
                    test.feature_id
                )));
            }
        }

        let mut tests = self.ab_tests.write().await;
        tests.insert(test.id.clone(), test);

        Ok(())
    }

    // Get A/B test
    pub async fn get_ab_test(&self, id: &str) -> Option<ABTest> {
        let tests = self.ab_tests.read().await;
        tests.get(id).cloned()
    }

    // Set override for testing
    pub async fn set_override(&self, name: impl Into<String>, enabled: bool) {
        let mut overrides = self.overrides.write().await;
        overrides.insert(name.into(), enabled);
    }

    // Clear override
    pub async fn clear_override(&self, name: &str) {
        let mut overrides = self.overrides.write().await;
        overrides.remove(name);
    }

    // Clear all overrides
    pub async fn clear_all_overrides(&self) {
        let mut overrides = self.overrides.write().await;
        overrides.clear();
    }
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_feature_toggle() {
        let manager = FeatureFlagManager::new();
        let feature = Feature::new("test_feature").with_state(FeatureState::Enabled);

        manager.register(feature).await.unwrap();

        let context = EvaluationContext::for_user("user1");
        assert!(manager.is_enabled("test_feature", &context).await);
    }

    #[tokio::test]
    async fn test_percentage_rollout() {
        let manager = FeatureFlagManager::new();
        let feature = Feature::new("test_rollout").with_rollout(RolloutStrategy::Percentage(50));

        manager.register(feature).await.unwrap();

        // Test multiple users - should see roughly 50% enabled
        let mut enabled_count = 0;
        for i in 0..100 {
            let context = EvaluationContext::for_user(format!("user{}", i));
            if manager.is_enabled("test_rollout", &context).await {
                enabled_count += 1;
            }
        }

        // Allow some variance
        assert!(enabled_count >= 30 && enabled_count <= 70);
    }

    #[tokio::test]
    async fn test_dependencies() {
        let manager = FeatureFlagManager::new();

        let base_feature = Feature::new("base").with_state(FeatureState::Disabled);
        let dependent_feature = Feature::new("dependent")
            .with_dependency("base")
            .with_state(FeatureState::Enabled);

        manager.register(base_feature).await.unwrap();
        manager.register(dependent_feature).await.unwrap();

        let context = EvaluationContext::for_user("user1");

        // Dependent should be disabled because base is disabled
        assert!(!manager.is_enabled("dependent", &context).await);

        // Enable base
        manager.enable("base").await.unwrap();

        // Now dependent should be enabled
        assert!(manager.is_enabled("dependent", &context).await);
    }

    #[tokio::test]
    async fn test_ab_testing() {
        let manager = FeatureFlagManager::new();

        let feature = Feature::new("test_ab").with_state(FeatureState::Enabled);
        manager.register(feature).await.unwrap();

        let test = ABTest {
            id: "test1".to_string(),
            feature_id: "test_ab".to_string(),
            name: "Test A/B".to_string(),
            variants: vec![
                Variant {
                    name: "control".to_string(),
                    weight: 50,
                    config: HashMap::new(),
                },
                Variant {
                    name: "treatment".to_string(),
                    weight: 50,
                    config: HashMap::new(),
                },
            ],
            duration: Duration::from_secs(86400),
            started_at: SystemTime::now(),
        };

        manager.create_ab_test(test).await.unwrap();

        let context = EvaluationContext::for_user("user1");
        let result = manager.evaluate("test_ab", &context).await.unwrap();

        assert!(result.variant.is_some());
    }
}
