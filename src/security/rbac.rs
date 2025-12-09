// # Role-Based Access Control (RBAC) Module
//
// Provides hierarchical role-based access control with role inheritance,
// composition, dynamic activation, and separation of duties constraints.
//
// ## Features
//
// - Hierarchical role definitions with parent-child relationships
// - Role composition and inheritance
// - Dynamic role activation and deactivation
// - Separation of Duties (SoD) constraints
// - Time-based role activation
// - Context-based role assignment
// - Role delegation and proxy capabilities
// - Fine-grained permission sets per role

use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime};
use crate::Result;
use crate::error::DbError;

/// Role identifier type
pub type RoleId = String;

/// User identifier type
pub type UserId = String;

/// Permission identifier type
pub type PermissionId = String;

/// Represents a role in the RBAC system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    /// Unique role identifier
    pub id: RoleId,
    /// Human-readable role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Parent roles for inheritance
    pub parent_roles: Vec<RoleId>,
    /// Direct permissions assigned to this role
    pub permissions: HashSet<PermissionId>,
    /// Custom attributes for the role
    pub attributes: HashMap<String, String>,
    /// Whether the role is active
    pub is_active: bool,
    /// Role creation timestamp
    pub created_at: i64,
    /// Role modification timestamp
    pub updated_at: i64,
    /// Role owner/creator
    pub owner: Option<UserId>,
    /// Role priority (higher values take precedence)
    pub priority: i32,
}

impl Role {
    /// Create a new role with the given ID and name
    pub fn new(id: RoleId, name: String) -> Self {
        let now = current_timestamp();
        Self {
            id,
            name,
            description: None,
            parent_roles: Vec::new(),
            permissions: HashSet::new(),
            attributes: HashMap::new(),
            is_active: true,
            created_at: now,
            updated_at: now,
            owner: None,
            priority: 0,
        }
    }

    /// Add a parent role for inheritance
    pub fn add_parent(&mut self, parent_id: RoleId) {
        if !self.parent_roles.contains(&parent_id) {
            self.parent_roles.push(parent_id);
            self.updated_at = current_timestamp();
        }
    }

    /// Remove a parent role
    pub fn remove_parent(&mut self, parentid: &RoleId) {
        self.parent_roles.retain(|id| id != parent_id);
        self.updated_at = current_timestamp();
    }

    /// Add a permission to the role
    pub fn add_permission(&mut self, permission: PermissionId) {
        self.permissions.insert(permission);
        self.updated_at = current_timestamp();
    }

    /// Remove a permission from the role
    pub fn remove_permission(&mut self, permission: &PermissionId) {
        self.permissions.remove(permission);
        self.updated_at = current_timestamp();
    }

    /// Check if role has a specific permission (direct only)
    pub fn has_permission(&self, permission: &PermissionId) -> bool {
        self.permissions.contains(permission)
    }
}

/// Represents a user's role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    /// Assignment ID
    pub id: String,
    /// User ID
    pub user_id: UserId,
    /// Role ID
    pub role_id: RoleId,
    /// Whether this role is currently activated
    pub is_activated: bool,
    /// When the assignment was created
    pub assigned_at: i64,
    /// When the assignment expires (None = never)
    pub expires_at: Option<i64>,
    /// Who assigned this role
    pub assigned_by: Option<UserId>,
    /// Conditions for role activation
    pub activation_conditions: Vec<ActivationCondition>,
}

/// Condition that must be met for role activation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivationCondition {
    /// Time-based activation (only active during certain hours)
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
    },
    /// Day of week restriction
    DayOfWeek {
        allowed_days: Vec<u8>, // 0 = Sunday, 6 = Saturday
    },
    /// IP address restriction
    IpAddress {
        allowed_ips: Vec<String>,
    },
    /// Location-based restriction
    Location {
        allowed_locations: Vec<String>,
    },
    /// Requires MFA to be completed
    RequiresMfa,
    /// Custom condition with a predicate expression
    Custom {
        expression: String,
    },
}

/// Separation of Duties (SoD) constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeparationOfDutiesConstraint {
    /// Constraint ID
    pub id: String,
    /// Constraint name
    pub name: String,
    /// Description of the constraint
    pub description: Option<String>,
    /// Type of SoD constraint
    pub constraint_type: SoDType,
    /// Conflicting roles that cannot be assigned together
    pub conflicting_roles: Vec<RoleId>,
    /// Whether the constraint is active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: i64,
}

/// Type of Separation of Duties constraint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SoDType {
    /// Static SoD - roles cannot be assigned to the same user
    Static,
    /// Dynamic SoD - roles cannot be activated simultaneously
    Dynamic,
    /// Object-level SoD - roles cannot perform certain operations on the same object
    ObjectLevel,
}

/// Role delegation allowing a user to delegate their role to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDelegation {
    /// Delegation ID
    pub id: String,
    /// User delegating the role
    pub delegator: UserId,
    /// User receiving the delegated role
    pub delegate: UserId,
    /// Role being delegated
    pub role_id: RoleId,
    /// When delegation starts
    pub valid_from: i64,
    /// When delegation ends
    pub valid_until: i64,
    /// Can the delegate further delegate this role?
    pub can_redelegate: bool,
    /// Is this delegation currently active?
    pub is_active: bool,
}

/// Session context for role activation
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// User ID
    pub user_id: UserId,
    /// Currently activated roles
    pub activated_roles: HashSet<RoleId>,
    /// Session IP address
    pub ip_address: Option<String>,
    /// Session location
    pub location: Option<String>,
    /// Session start time
    pub session_start: i64,
    /// MFA completed in this session
    pub mfa_completed: bool,
    /// Custom context attributes
    pub attributes: HashMap<String, String>,
}

/// RBAC Manager - main interface for role-based access control
pub struct RbacManager {
    /// All defined roles
    roles: Arc<RwLock<HashMap<RoleId, Role>>>,
    /// User role assignments
    assignments: Arc<RwLock<HashMap<UserId, Vec<RoleAssignment>>>>,
    /// Separation of duties constraints
    sod_constraints: Arc<RwLock<Vec<SeparationOfDutiesConstraint>>>,
    /// Role delegations
    delegations: Arc<RwLock<Vec<RoleDelegation>>>,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionContext>>>,
    /// Role hierarchy cache (role -> all inherited permissions)
    permission_cache: Arc<RwLock<HashMap<RoleId<PermissionId>>>>,
}

impl RbacManager {
    /// Create a new RBAC manager
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
            sod_constraints: Arc::new(RwLock::new(Vec::new())),
            delegations: Arc::new(RwLock::new(Vec::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new role
    pub fn create_role(&self, role: Role) -> Result<()> {
        let mut roles = self.roles.write();

        if roles.contains_key(&role.id) {
            return Err(DbError::AlreadyExists(format!("Role {} already exists", role.id))));
        }

        // Validate parent roles exist
        for parent_id in &role.parent_roles {
            if !roles.contains_key(parent_id) {
                return Err(DbError::NotFound(format!("Parent role {} not found", parent_id))));
            }
        }

        // Check for circular dependencies
        if self.has_circular_dependency(&role, &roles) {
            return Err(DbError::InvalidInput("Circular role dependency detected".to_string()));
        }

        roles.insert(role.id.clone(), role);

        // Invalidate permission cache
        self.permission_cache.write().clear();

        Ok(())
    }

    /// Update an existing role
    pub fn update_role(&self, role: Role) -> Result<()> {
        let mut roles = self.roles.write();

        if !roles.contains_key(&role.id) {
            return Err(DbError::NotFound(format!("Role {} not found", role.id))));
        }

        // Validate parent roles exist
        for parent_id in &role.parent_roles {
            if !roles.contains_key(parent_id) {
                return Err(DbError::NotFound(format!("Parent role {} not found", parent_id))));
            }
        }

        // Check for circular dependencies
        if self.has_circular_dependency(&role, &roles) {
            return Err(DbError::InvalidInput("Circular role dependency detected".to_string()));
        }

        roles.insert(role.id.clone(), role);

        // Invalidate permission cache
        self.permission_cache.write().clear();

        Ok(())
    }

    /// Delete a role
    pub fn delete_role(&self, roleid: &RoleId) -> Result<()> {
        let mut roles = self.roles.write();

        if !roles.contains_key(role_id) {
            return Err(DbError::NotFound(format!("Role {} not found", role_id))));
        }

        // Check if any role has this as a parent
        for role in roles.values() {
            if role.parent_roles.contains(role_id) {
                return Err(DbError::InvalidOperation(
                    format!("Cannot delete role {}; it is a parent of role {}", role_id, role.id)
                ));
            }
        }

        // Remove role assignments
        let mut assignments = self.assignments.write();
        for user_assignments in assignments.values_mut() {
            user_assignments.retain(|a| &a.role_id != role_id);
        }

        roles.remove(role_id);

        // Invalidate permission cache
        self.permission_cache.write().clear();

        Ok(())
    }

    /// Get a role by ID
    pub fn get_role(&self, role_id: &RoleId) -> Result<Role> {
        self.roles.read()
            .get(role_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Role {} not found", role_id)))
    }

    /// Get all roles
    pub fn get_all_roles(&self) -> Vec<Role> {
        self.roles.read().values().cloned().collect()
    }

    /// Assign a role to a user
    pub fn assign_role(&self, assignment: RoleAssignment) -> Result<()> {
        // Verify role exists
        if !self.roles.read().contains_key(&assignment.role_id) {
            return Err(DbError::NotFound(format!("Role {} not found", assignment.role_id))));
        }

        // Check SoD constraints
        self.check_sod_constraints(&assignment.user_id, &assignment.role_id)?;

        let mut assignments = self.assignments.write();
        let user_assignments = assignments.entry(assignment.user_id.clone()).or_insert_with(Vec::new);

        // Check if already assigned
        if user_assignments.iter().any(|a| a.role_id == assignment.role_id) {
            return Err(DbError::AlreadyExists(
                format!("Role {} already assigned to user {}", assignment.role_id, assignment.user_id)
            )));
        }

        user_assignments.push(assignment);
        Ok(())
    }

    /// Revoke a role from a user
    pub fn revoke_role(&self, user_id: &UserId, role_id: &RoleId) -> Result<()> {
        let mut assignments = self.assignments.write();

        if let Some(user_assignments) = assignments.get_mut(user_id) {
            let original_len = user_assignments.len();
            user_assignments.retain(|a| &a.role_id != role_id);

            if user_assignments.len() == original_len {
                return Err(DbError::NotFound(
                    format!("Role {} not assigned to user {}", role_id, user_id)
                )));
            }

            Ok(())
        } else {
            Err(DbError::NotFound(format!("No role assignments for user {}", user_id)))
        }
    }

    /// Get all role assignments for a user
    pub fn get_user_roles(&self, user_id: &UserId) -> Vec<RoleAssignment> {
        self.assignments.read()
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Activate a role in a session
    pub fn activate_role(&self, session_id: &str, role_id: &RoleId) -> Result<()> {
        let mut sessions = self.sessions.write());
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| DbError::NotFound("Session not found".to_string()))?;

        // Check if user has this role assigned
        let assignments = self.assignments.read();
        let user_assignments = assignments.get(&session.user_id)
            .ok_or_else(|| DbError::NotFound("No role assignments found".to_string()))?;

        let assignment = user_assignments.iter()
            .find(|a| &a.role_id == role_id)
            .ok_or_else(|| DbError::NotFound(format!("Role {} not assigned to user", role_id)))?);

        // Check activation conditions
        self.check_activation_conditions(assignment, session)?;

        // Check dynamic SoD constraints
        self.check_dynamic_sod(&session.user_id, role_id, &session.activated_roles)?;

        session.activated_roles.insert(role_id.clone());
        Ok(())
    }

    /// Deactivate a role in a session
    pub fn deactivate_role(&self, session_id: &str, role_id: &RoleId) -> Result<()> {
        let mut sessions = self.sessions.write();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| DbError::NotFound("Session not found".to_string()))?;

        session.activated_roles.remove(role_id);
        Ok(())
    }

    /// Get all effective permissions for a user (including inherited)
    pub fn get_effective_permissions(&self, user_id: &UserId) -> HashSet<PermissionId> {
        let assignments = self.assignments.read();
        let user_assignments = assignments.get(user_id);

        if user_assignments.is_none() {
            return HashSet::new();
        }

        let mut permissions = HashSet::new();
        for assignment in user_assignments.unwrap() {
            if !assignment.is_activated {
                continue;
            }

            // Check if assignment is expired
            if let Some(expires_at) = assignment.expires_at {
                if expires_at < current_timestamp() {
                    continue;
                }
            }

            // Get all permissions from role hierarchy
            let role_permissions = self.get_role_permissions(&assignment.role_id);
            permissions.extend(role_permissions);
        }

        permissions
    }

    /// Get all permissions for a role including inherited permissions
    pub fn get_role_permissions(&self, role_id: &RoleId) -> HashSet<PermissionId> {
        // Check cache first
        {
            let cache = self.permission_cache.read();
            if let Some(cached) = cache.get(role_id) {
                return cached.clone();
            }
        }

        // Compute permissions with inheritance
        let mut permissions = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_permissions_recursive(role_id, &mut permissions, &mut visited);

        // Update cache
        self.permission_cache.write().insert(role_id.clone(), permissions.clone());

        permissions
    }

    /// Check if a user has a specific permission (through any role)
    pub fn has_permission(&self, user_id: &UserId, permission: &PermissionId) -> bool {
        self.get_effective_permissions(user_id).contains(permission)
    }

    /// Add a Separation of Duties constraint
    pub fn add_sod_constraint(&self, constraint: SeparationOfDutiesConstraint) -> Result<()> {
        // Validate that all roles in the constraint exist
        let roles = self.roles.read();
        for role_id in &constraint.conflicting_roles {
            if !roles.contains_key(role_id) {
                return Err(DbError::NotFound(format!("Role {} not found", role_id))));
            }
        }

        self.sod_constraints.write().push(constraint);
        Ok(())
    }

    /// Remove a Separation of Duties constraint
    pub ffn remove_sod_constraint(&self, constraintid: &str)-> Result<()> {
        let mut constraints = self.sod_constraints.write();
        let original_len = constraints.len();
        constraints.retain(|c| c.id != constraint_id);

        if constraints.len() == original_len {
            return Err(DbError::NotFound(format!("SoD constraint {} not found", constraint_id))));
        }

        Ok(())
    }

    /// Get all SoD constraints
    pub fn get_sod_constraints(&self) -> Vec<SeparationOfDutiesConstraint> {
        self.sod_constraints.read().clone()
    }

    /// Create a role delegation
    pub fn create_delegation(&self, delegation: RoleDelegation) -> Result<()> {
        // Verify the delegator has the role
        let assignments = self.assignments.read();
        let delegator_assignments = assignments.get(&delegation.delegator)
            .ok_or_else(|| DbError::NotFound("Delegator has no role assignments".to_string()))?;

        if !delegator_assignments.iter().any(|a| a.role_id == delegation.role_id) {
            return Err(DbError::InvalidOperation(
                "Delegator does not have the role being delegated".to_string()
            ));
        }

        // Check if delegation would violate SoD constraints for the delegate
        self.check_sod_constraints(&delegation.delegate, &delegation.role_id)?;

        self.delegations.write().push(delegation);
        Ok(())
    }

    /// Revoke a role delegation
    pub fnfn revoke_delegation(&self, delegationid: &str)> Result<()> {
        let mut delegations = self.delegations.write();
        let original_len = delegations.len();
        delegations.retain(|d| d.id != delegation_id);

        if delegations.len() == original_len {
            return Err(DbError::NotFound(format!("Delegation {} not found", delegation_id))));
        }

        Ok(())
    }

    /// Get active delegations for a user
    pub fn get_user_delegations(&self, user_id: &UserId) -> Vec<RoleDelegation> {
        let now = current_timestamp();
        self.delegations.read()
            .iter()
            .filter(|d| {
                (&d.delegate == user_id || &d.delegator == user_id) &&
                d.is_active &&
                d.valid_from <= now &&
                d.valid_until > now
            })
            .cloned()
            .collect()
    }

    // Private helper methods

    fn has_circular_dependency(&self, role: &Role, roles: &HashMap<RoleId, Role>) -> bool {
        let mut visited = HashSet::new();
        self.detect_cycle(&role.id, roles, &mut visited, &HashSet::new())
    }

    fn detect_cycle(
        &self,
        role_id: &RoleId,
        roles: &HashMap<RoleId, Role>,
        visited: &mut HashSet<RoleId>,
        rec_stack: &HashSet<RoleId>,
    ) -> bool {
        if rec_stack.contains(role_id) {
            return true;
        }

        if visited.contains(role_id) {
            return false;
        }

        visited.insert(role_id.clone());
        let mut new_rec_stack = rec_stack.clone();
        new_rec_stack.insert(role_id.clone());

        if let Some(role) = roles.get(role_id) {
            for parent_id in &role.parent_roles {
                if self.detect_cycle(parent_id, roles, visited, &new_rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn collect_permissions_recursive(
        &self,
        role_id: &RoleId,
        permissions: &mut HashSet<PermissionId>,
        visited: &mut HashSet<RoleId>,
    ) {
        if visited.contains(role_id) {
            return;
        }

        visited.insert(role_id.clone());

        let roles = self.roles.read();
        if let Some(role) = roles.get(role_id) {
            // Add direct permissions
            permissions.extend(role.permissions.iter().cloned());

            // Recursively add parent permissions
            for parent_id in &role.parent_roles {
                self.collect_permissions_recursive(parent_id, permissions, visited);
            }
        }
    }

    fn fn check_sod_constraints(&self, user_id: &UserId, newrole_id: &RoleId) Result<()> {
        let constraints = self.sod_constraints.read();
        let assignments = self.assignments.read();

        // Get user's current roles
        let current_roles: HashSet<RoleId> = assignments.get(user_id)
            .map(|assignments| assignments.iter().map(|a| a.role_id.clone()).collect())
            .unwrap_or_default();

        for constraint in constraints.iter() {
            if !constraint.is_active || constraint.constraint_type != SoDType::Static {
                continue;
            }

            if !constraint.conflicting_roles.contains(new_role_id) {
                continue;
            }

            // Check if user already has a conflicting role
            for role_id in &constraint.conflicting_roles {
                if role_id != new_role_id && current_roles.contains(role_id) {
                    return Err(DbError::InvalidOperation(
                        format!("SoD constraint violated: {} conflicts with {}", new_role_id, role_id)
                    )));
                }
            }
        }

        Ok(())
    }

    fn cfn check_dynamic_sod(
        &self,
        user_id: &UserId,
        newrole_id: &RoleId,
        activated_roles: &HashSet<RoleId>,
    )Result<()> {
        let constraints = self.sod_constraints.read();

        for constraint in constraints.iter() {
            if !constraint.is_active || constraint.constraint_type != SoDType::Dynamic {
                continue;
            }

            if !constraint.conflicting_roles.contains(new_role_id) {
                continue;
            }

            // Check if any conflicting role is already activated
            for role_id in &constraint.conflicting_roles {
                if role_id != new_role_id && activated_roles.contains(role_id) {
                    return Err(DbError::InvalidOperation(
                        format!("Dynamic SoD constraint violated: cannot activate {} while {} is active",
                                new_role_id, role_id)
                    )));
                }
            }
        }

        Ok(())
    }

    fn check_activation_conditions(
        &self,
        assignment: &RoleAssignment,
        session: &SessionContext,
    ) -> Result<()> {
        for condition in &assignment.activation_conditions {
            match condition {
                ActivationCondition::RequiresMfa => {
                    if !session.mfa_completed {
                        return Err(DbError::InvalidOperation("MFA required for role activation".to_string()));
                    }
                }
                ActivationCondition::IpAddress { allowed_ips } => {
                    if let Some(ip) = &session.ip_address {
                        if !allowed_ips.contains(ip) {
                            return Err(DbError::InvalidOperation("IP address not allowed".to_string()));
                        }
                    } else {
                        return Err(DbError::InvalidOperation("IP address required".to_string()));
                    }
                }
                ActivationCondition::Location { allowed_locations } => {
                    if let Some(location) = &session.location {
                        if !allowed_locations.contains(location) {
                            return Err(DbError::InvalidOperation("Location not allowed".to_string()));
                        }
                    } else {
                        return Err(DbError::InvalidOperation("Location required".to_string()));
                    }
                }
                ActivationCondition::TimeWindow { start_hour, end_hour } => {
                    // This is simplified - would need actual time checking
                    let _current_hour = 12; // Placeholder
                    // Check if current hour is within window
                }
                ActivationCondition::DayOfWeek { allowed_days } => {
                    // This is simplified - would need actual day checking
                    let _current_day = 3; // Placeholder
                    // Check if current day is allowed
                }
                ActivationCondition::Custom { expression: _ } => {
                    // Would need expression evaluator
                }
            }
        }

        Ok(())
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current timestamp in seconds since Unix epoch
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_create_role() {
        let manager = RbacManager::new();
        let role = Role::new("admin".to_string(), "Administrator".to_string());
        assert!(manager.create_role(role).is_ok());
    }

    #[test]
    fn test_role_inheritance() {
        let manager = RbacManager::new();

        let mut parent_role = Role::new("parent".to_string(), "Parent Role".to_string());
        parent_role.add_permission("read".to_string());
        manager.create_role(parent_role).unwrap();

        let mut child_role = Role::new("child".to_string(), "Child Role".to_string());
        child_role.add_parent("parent".to_string());
        child_role.add_permission("write".to_string());
        manager.create_role(child_role).unwrap();

        let permissions = manager.get_role_permissions(&"child".to_string());
        assert!(permissions.contains("read"));
        assert!(permissions.contains("write"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let manager = RbacManager::new();

        let role1 = Role::new("role1".to_string(), "Role 1".to_string());
        manager.create_role(role1).unwrap();

        let mut role2 = Role::new("role2".to_string(), "Role 2".to_string());
        role2.add_parent("role1".to_string());
        manager.create_role(role2).unwrap();

        // Try to make role1 inherit from role2 (circular)
        let mut role1_updated = manager.get_role(&"role1".to_string()).unwrap();
        role1_updated.add_parent("role2".to_string());
        assert!(manager.update_role(role1_updated).is_err());
    }

    #[test]
    fn test_sod_constraint() {
        let manager = RbacManager::new();

        let role1 = Role::new("role1".to_string(), "Role 1".to_string());
        let role2 = Role::new("role2".to_string(), "Role 2".to_string());
        manager.create_role(role1).unwrap();
        manager.create_role(role2).unwrap();

        // Add SoD constraint
        let constraint = SeparationOfDutiesConstraint {
            id: "sod1".to_string(),
            name: "Test SoD".to_string(),
            description: None,
            constraint_type: SoDType::Static,
            conflicting_roles: vec!["role1".to_string(), "role2".to_string()],
            is_active: true,
            created_at: current_timestamp(),
        };
        manager.add_sod_constraint(constraint).unwrap();

        // Assign first role
        let assignment1 = RoleAssignment {
            id: "a1".to_string(),
            user_id: "user1".to_string(),
            role_id: "role1".to_string(),
            is_activated: true,
            assigned_at: current_timestamp(),
            expires_at: None,
            assigned_by: None,
            activation_conditions: vec![],
        };
        manager.assign_role(assignment1).unwrap();

        // Try to assign conflicting role - should fail
        let assignment2 = RoleAssignment {
            id: "a2".to_string(),
            user_id: "user1".to_string(),
            role_id: "role2".to_string(),
            is_activated: true,
            assigned_at: current_timestamp(),
            expires_at: None,
            assigned_by: None,
            activation_conditions: vec![],
        };
        assert!(manager.assign_role(assignment2).is_err());
    }
}
