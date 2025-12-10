// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::{HashMap, HashSet};
use std::sync::{Arc};
use std::time::{SystemTime};
use parking_lot::RwLock;

use crate::api::Session;
use crate::error::DbError;
use super::types::*;

// ============================================================================
// Authorization Engine - RBAC, ABAC, Policy Engine
// ============================================================================

// Authorization engine
pub struct AuthorizationEngine {
    // RBAC manager
    rbac: Arc<RbacManager>,
    // ABAC evaluator
    abac: Arc<AbacEvaluator>,
    // Policy engine
    policy_engine: Arc<PolicyEngine>,
}

// RBAC manager
pub struct RbacManager {
    // Roles
    roles: Arc<RwLock<HashMap<String, Role>>>,
    // User role assignments
    user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

// Role definition
#[derive(Debug, Clone)]
pub struct Role {
    // Role ID
    pub id: String,
    // Role name
    pub name: String,
    // Description
    pub description: String,
    // Permissions
    pub permissions: Vec<String>,
    // Parent roles (inheritance)
    pub parent_roles: Vec<String>,
    // Created at
    pub created_at: SystemTime,
}

// ABAC evaluator
pub struct AbacEvaluator {
    // Attribute providers
    attribute_providers: Vec<Arc<dyn AttributeProvider>>,
}

// Attribute provider trait
pub trait AttributeProvider: Send + Sync {
    // Get attributes for subject
    fn get_subject_attributes(&self, subject: &str) -> HashMap<String, AttributeValue>;
    // Get attributes for resource
    fn get_resource_attributes(&self, resource: &str) -> HashMap<String, AttributeValue>;
    // Get attributes for environment
    fn get_environment_attributes(&self) -> HashMap<String, AttributeValue>;
}

// Attribute value
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    Number(i64),
    Boolean(bool),
    List(Vec<AttributeValue>),
}

// Policy engine
pub struct PolicyEngine {
    // Policies
    policies: Arc<RwLock<HashMap<String, Policy>>>,
}

// Policy definition
#[derive(Debug, Clone)]
pub struct Policy {
    // Policy ID
    pub id: String,
    // Policy name
    pub name: String,
    // Description
    pub description: String,
    // Effect
    pub effect: PolicyEffect,
    // Subjects (who)
    pub subjects: Vec<String>,
    // Resources (what)
    pub resources: Vec<String>,
    // Actions (how)
    pub actions: Vec<String>,
    // Conditions
    pub conditions: Vec<PolicyCondition>,
}

// Policy effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

// Policy condition
#[derive(Debug, Clone)]
pub struct PolicyCondition {
    // Attribute path
    pub attribute: String,
    // Operator
    pub operator: PolicyOperator,
    // Value
    pub value: AttributeValue,
}

// Policy operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEquals,
    LessThanOrEquals,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
}

impl AuthorizationEngine {
    // Create new authorization engine
    pub fn new() -> Self {
        Self {
            rbac: Arc::new(RbacManager::new()),
            abac: Arc::new(AbacEvaluator::new()),
            policy_engine: Arc::new(PolicyEngine::new()),
        }
    }

    // Authorize session for permissions
    pub fn authorize(&self, session: &Session, required_permissions: &[String]) -> Result<bool, DbError> {
        // Check RBAC first
        if self.rbac.has_permissions(&session.user_id, required_permissions) {
            return Ok(true);
        }

        // Check session permissions
        for perm in required_permissions {
            if !session.permissions.contains(perm) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    // Check if user has permission
    pub fn check_permission(&self, user_id: &str, permission: &str) -> bool {
        self.rbac.has_permission(user_id, permission)
    }

    // Evaluate policy
    pub fn evaluate_policy(&self, subject: &str, resource: &str, action: &str) -> Result<bool, DbError> {
        self.policy_engine.evaluate(subject, resource, action, &self.abac)
    }
}

impl RbacManager {
    fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create role
    pub fn create_role(&self, role: Role) {
        let mut roles = self.roles.write();
        roles.insert(role.id.clone(), role);
    }

    // Assign role to user
    pub fn assign_role(&self, user_id: String, role_id: String) {
        let mut user_roles = self.user_roles.write();
        user_roles.entry(user_id).or_insert_with(Vec::new).push(role_id);
    }

    // Remove role from user
    pub fn remove_role(&self, user_id: &str, role_id: &str) -> bool {
        let mut user_roles = self.user_roles.write();
        if let Some(roles) = user_roles.get_mut(user_id) {
            if let Some(pos) = roles.iter().position(|r| r == role_id) {
                roles.remove(pos);
                return true;
            }
        }
        false
    }

    // Get user roles
    pub fn get_user_roles(&self, user_id: &str) -> Vec<Role> {
        let user_roles = self.user_roles.read();
        let roles = self.roles.read();

        user_roles.get(user_id)
            .map(|role_ids| {
                role_ids.iter()
                    .filter_map(|id| roles.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    // Get all permissions for user (including inherited)
    pub fn get_user_permissions(&self, user_id: &str) -> HashSet<String> {
        let mut permissions = HashSet::new();
        let user_roles = self.get_user_roles(user_id);

        for role in user_roles {
            for perm in &role.permissions {
                permissions.insert(perm.clone());
            }
        }

        permissions
    }

    // Check if user has permission
    pub fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        let permissions = self.get_user_permissions(user_id);
        permissions.contains(permission)
    }

    // Check if user has all permissions
    pub fn has_permissions(&self, user_id: &str, required: &[String]) -> bool {
        let permissions = self.get_user_permissions(user_id);
        required.iter().all(|p| permissions.contains(p))
    }
}

impl AbacEvaluator {
    fn new() -> Self {
        Self {
            attribute_providers: Vec::new(),
        }
    }

    // Add attribute provider
    pub fn add_provider(&mut self, provider: Arc<dyn AttributeProvider>) {
        self.attribute_providers.push(provider);
    }

    // Evaluate ABAC policy
    pub fn evaluate(&self, subject: &str, resource: &str, action: &str, conditions: &[PolicyCondition]) -> bool {
        // Collect attributes
        let mut subject_attrs = HashMap::new();
        let mut resource_attrs = HashMap::new();
        let mut env_attrs = HashMap::new();

        for provider in &self.attribute_providers {
            subject_attrs.extend(provider.get_subject_attributes(subject));
            resource_attrs.extend(provider.get_resource_attributes(resource));
            env_attrs.extend(provider.get_environment_attributes());
        }

        // Evaluate conditions
        for condition in conditions {
            let attr_value = if condition.attribute.starts_with("subject.") {
                subject_attrs.get(&condition.attribute[8..])
            } else if condition.attribute.starts_with("resource.") {
                resource_attrs.get(&condition.attribute[9..])
            } else if condition.attribute.starts_with("environment.") {
                env_attrs.get(&condition.attribute[12..])
            } else {
                None
            };

            if let Some(value) = attr_value {
                if !self.evaluate_condition(value, &condition.operator, &condition.value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    // Evaluate single condition
    fn evaluate_condition(&self, actual: &AttributeValue, operator: &PolicyOperator, expected: &AttributeValue) -> bool {
        match operator {
            PolicyOperator::Equals => actual == expected,
            PolicyOperator::NotEquals => actual != expected,
            PolicyOperator::GreaterThan => match (actual, expected) {
                (AttributeValue::Number(a), AttributeValue::Number(b)) => a > b,
                _ => false,
            },
            PolicyOperator::LessThan => match (actual, expected) {
                (AttributeValue::Number(a), AttributeValue::Number(b)) => a < b,
                _ => false,
            },
            PolicyOperator::GreaterThanOrEquals => match (actual, expected) {
                (AttributeValue::Number(a), AttributeValue::Number(b)) => a >= b,
                _ => false,
            },
            PolicyOperator::LessThanOrEquals => match (actual, expected) {
                (AttributeValue::Number(a), AttributeValue::Number(b)) => a <= b,
                _ => false,
            },
            PolicyOperator::In => match (actual, expected) {
                (val, AttributeValue::List(list)) => list.contains(val),
                _ => false,
            },
            PolicyOperator::NotIn => match (actual, expected) {
                (val, AttributeValue::List(list)) => !list.contains(val),
                _ => false,
            },
            PolicyOperator::Contains => match (actual, expected) {
                (AttributeValue::List(list), val) => list.contains(val),
                (AttributeValue::String(s), AttributeValue::String(sub)) => s.contains(sub),
                _ => false,
            },
            PolicyOperator::StartsWith => match (actual, expected) {
                (AttributeValue::String(s), AttributeValue::String(prefix)) => s.starts_with(prefix),
                _ => false,
            },
            PolicyOperator::EndsWith => match (actual, expected) {
                (AttributeValue::String(s), AttributeValue::String(suffix)) => s.ends_with(suffix),
                _ => false,
            },
        }
    }
}

impl PolicyEngine {
    fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Add policy
    pub fn add_policy(&self, policy: Policy) {
        let mut policies = self.policies.write();
        policies.insert(policy.id.clone(), policy);
    }

    // Remove policy
    pub fn remove_policy(&self, policy_id: &str) -> bool {
        let mut policies = self.policies.write();
        policies.remove(policy_id).is_some()
    }

    // Evaluate policies
    pub fn evaluate(&self, subject: &str, resource: &str, action: &str, abac: &AbacEvaluator) -> Result<bool, DbError> {
        let policies = self.policies.read();

        let mut allow = false;
        let mut deny = false;

        for policy in policies.values() {
            // Check if policy applies
            if !self.matches_pattern(&policy.subjects, subject) {
                continue;
            }
            if !self.matches_pattern(&policy.resources, resource) {
                continue;
            }
            if !self.matches_pattern(&policy.actions, action) {
                continue;
            }

            // Evaluate conditions
            if !policy.conditions.is_empty() {
                if !abac.evaluate(subject, resource, action, &policy.conditions) {
                    continue;
                }
            }

            match policy.effect {
                PolicyEffect::Allow => allow = true,
                PolicyEffect::Deny => deny = true,
            }
        }

        // Deny takes precedence
        if deny {
            Ok(false)
        } else {
            Ok(allow)
        }
    }

    // Match pattern (supports wildcards)
    fn matches_pattern(&self, patterns: &[String], value: &str) -> bool {
        if patterns.is_empty() {
            return true;
        }

        for pattern in patterns {
            if pattern == "*" || pattern == value {
                return true;
            }
            if pattern.contains('*') {
                // Simple wildcard matching
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    if value.starts_with(parts[0]) && value.ends_with(parts[1]) {
                        return true;
                    }
                }
            }
        }

        false
    }

    // Get all policies
    pub fn get_policies(&self) -> Vec<Policy> {
        let policies = self.policies.read();
        policies.values().cloned().collect()
    }
}
