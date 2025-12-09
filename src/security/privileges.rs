// # Privilege Management Module
//
// Provides comprehensive privilege management including system privileges,
// object privileges, GRANT/REVOKE operations with admin option, and
// privilege inheritance through roles.
//
// ## Features
//
// - System-level privileges (CREATE, ALTER, DROP, etc.)
// - Object-level privileges (SELECT, INSERT, UPDATE, DELETE)
// - GRANT/REVOKE with WITH GRANT OPTION
// - Privilege inheritance through roles
// - Privilege dependency tracking
// - Cascading privilege revocation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::Result;
use crate::error::DbError;

/// Principal identifier (user or role)
pub type PrincipalId = String;

/// Object identifier
pub type ObjectId = String;

/// System privilege types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SystemPrivilege {
    // Database administration
    CreateDatabase,
    DropDatabase,
    AlterDatabase,
    BackupDatabase,
    RestoreDatabase,

    // Table privileges
    CreateTable,
    CreateAnyTable,
    AlterAnyTable,
    DropAnyTable,
    SelectAnyTable,
    InsertAnyTable,
    UpdateAnyTable,
    DeleteAnyTable,

    // Index privileges
    CreateIndex,
    CreateAnyIndex,
    AlterAnyIndex,
    DropAnyIndex,

    // View privileges
    CreateView,
    CreateAnyView,
    DropAnyView,

    // Sequence privileges
    CreateSequence,
    CreateAnySequence,
    AlterAnySequence,
    DropAnySequence,

    // Procedure/Function privileges
    CreateProcedure,
    CreateAnyProcedure,
    AlterAnyProcedure,
    DropAnyProcedure,
    ExecuteAnyProcedure,

    // Trigger privileges
    CreateTrigger,
    CreateAnyTrigger,
    AlterAnyTrigger,
    DropAnyTrigger,

    // User/Role management
    CreateUser,
    AlterUser,
    DropUser,
    CreateRole,
    AlterRole,
    DropRole,
    GrantAnyPrivilege,
    GrantAnyRole,

    // System operations
    AlterSystem,
    AuditSystem,
    ManageReplication,
    ManageEncryption,

    // Special privileges
    Sysdba,
    Sysoper,
    SysBackup,
}

/// Object privilege types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ObjectPrivilege {
    Select,
    Insert,
    Update,
    Delete,
    Execute,
    Alter,
    Index,
    References,
    Debug,
    All,
}

/// Object type for privilege management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrivilegeObjectType {
    Table,
    View,
    Sequence,
    Procedure,
    Function,
    Package,
    Type,
    Trigger,
}

/// Privilege grant record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeGrant {
    /// Grant ID
    pub grant_id: String,
    /// Grantee (user or role)
    pub grantee: PrincipalId,
    /// Grantor (who granted the privilege)
    pub grantor: PrincipalId,
    /// Type of privilege
    pub privilege_type: PrivilegeType,
    /// Can grantee grant this to others?
    pub with_grant_option: bool,
    /// Can grantee create hierarchy under this?
    pub with_hierarchy_option: bool,
    /// Grant timestamp
    pub granted_at: i64,
    /// Grant expiration (None = never expires)
    pub expires_at: Option<i64>,
    /// Is this grant currently active?
    pub is_active: bool,
}

/// Type of privilege being granted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrivilegeType {
    /// System-level privilege
    System(SystemPrivilege),
    /// Object-level privilege
    Object {
        privilege: ObjectPrivilege,
        object_type: PrivilegeObjectType,
        object_id: ObjectId,
        /// Specific columns (for SELECT, UPDATE, INSERT, REFERENCES)
        columns: Option<Vec<String>>,
    },
}

/// Privilege dependency (for cascade operations)
#[derive(Debug, Clone)]
pub struct PrivilegeDependency {
    /// Dependent grant
    pub dependent_grant_id: String,
    /// Grant it depends on
    pub depends_on_grant_id: String,
}

/// Privilege check result
#[derive(Debug, Clone)]
pub struct PrivilegeCheckResult {
    /// Does the principal have the privilege?
    pub has_privilege: bool,
    /// Source of the privilege (direct grant, role inheritance, etc.)
    pub source: Option<PrivilegeSource>,
    /// Can the principal grant this to others?
    pub can_grant: bool,
}

/// Source of a privilege
#[derive(Debug, Clone)]
pub enum PrivilegeSource {
    /// Direct grant to the principal
    DirectGrant { grant_id: String },
    /// Inherited through a role
    RoleInheritance { role_id: String, grant_id: String },
    /// Public grant
    PublicGrant { grant_id: String },
}

/// Privilege manager
pub struct PrivilegeManager {
    /// All privilege grants
    grants: Arc<RwLock<HashMap<String, PrivilegeGrant>>>,
    /// Grants by grantee (for quick lookup)
    grantee_index: Arc<RwLock<HashMap<PrincipalId<String>>>>,
    /// Grants by grantor (for dependency tracking)
    grantor_index: Arc<RwLock<HashMap<PrincipalId<String>>>>,
    /// Grants by object (for object-level privileges)
    object_index: Arc<RwLock<HashMap<ObjectId<String>>>>,
    /// Privilege dependencies
    dependencies: Arc<RwLock<Vec<PrivilegeDependency>>>,
    /// Grant ID counter
    grant_counter: Arc<RwLock<u64>>,
    /// Role hierarchy (role -> parent roles)
    role_hierarchy: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PrivilegeManager {
    /// Create a new privilege manager
    pub fn new() -> Self {
        Self {
            grants: Arc::new(RwLock::new(HashMap::new())),
            grantee_index: Arc::new(RwLock::new(HashMap::new())),
            grantor_index: Arc::new(RwLock::new(HashMap::new())),
            object_index: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(Vec::new())),
            grant_counter: Arc::new(RwLock::new(0)),
            role_hierarchy: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Grant a system privilege
    pub fn grant_system_privilege(
        &self,
        grantor: PrincipalId,
        grantee: PrincipalId,
        privilege: SystemPrivilege,
        with_grant_option: bool,
    ) -> Result<String> {
        // Verify grantor has the privilege with grant option
        if grantor != "SYSTEM" {
            let check = self.check_system_privilege(&grantor, &privilege);
            if !check.has_privilege {
                return Err(DbError::InvalidOperation(
                    "Grantor does not have the privilege".to_string()
                ));
            }
            if !check.can_grant {
                return Err(DbError::InvalidOperation(
                    "Grantor cannot grant this privilege".to_string()
                ));
            }
        }

        let grant_id = self.generate_grant_id();

        let grant = PrivilegeGrant {
            grant_id: grant_id.clone(),
            grantee: grantee.clone(),
            grantor: grantor.clone(),
            privilege_type: PrivilegeType::System(privilege),
            with_grant_option,
            with_hierarchy_option: false,
            granted_at: current_timestamp(),
            expires_at: None,
            is_active: true,
        };

        self.add_grant(grant)?;

        Ok(grant_id)
    }

    /// Grant an object privilege
    pub fn grant_object_privilege(
        &self,
        grantor: PrincipalId,
        grantee: PrincipalId,
        privilege: ObjectPrivilege,
        object_type: PrivilegeObjectType,
        object_id: ObjectId,
        columns: Option<Vec<String>>,
        with_grant_option: bool,
    ) -> Result<String> {
        // Verify grantor has the privilege with grant option
        if grantor != "SYSTEM" {
            let check = self.check_object_privilege(
                &grantor,
                &privilege,
                &object_type,
                &object_id,
                columns.as_ref(),
            );
            if !check.has_privilege {
                return Err(DbError::InvalidOperation(
                    "Grantor does not have the privilege".to_string()
                ));
            }
            if !check.can_grant {
                return Err(DbError::InvalidOperation(
                    "Grantor cannot grant this privilege".to_string()
                ));
            }
        }

        let grant_id = self.generate_grant_id();

        let grant = PrivilegeGrant {
            grant_id: grant_id.clone(),
            grantee: grantee.clone(),
            grantor: grantor.clone(),
            privilege_type: PrivilegeType::Object {
                privilege,
                object_type,
                object_id: object_id.clone(),
                columns,
            },
            with_grant_option,
            with_hierarchy_option: false,
            granted_at: current_timestamp(),
            expires_at: None,
            is_active: true,
        };

        // Add to object index
        self.object_index.write()
            .entry(object_id)
            .or_insert_with(HashSet::new)
            .insert(grant_id.clone());

        self.add_grant(grant)?;

        Ok(grant_id)
    }

    /// Revoke a privilege grant
    pub fn revoke_grant(
        &self,
        grant_id: &str,
        cascade: bool,
    ) -> Result<Vec<String>> {
        let grant = {
            let grants = self.grants.read();
            grants.get(grant_id)
                .ok_or_else(|| DbError::NotFound("Grant not found".to_string()))?
                .clone()
        };

        let mut revoked_grants = vec![grant_id.to_string()];

        // Find dependent grants
        let dependent_grants = self.find_dependent_grants(grant_id);

        if !dependent_grants.is_empty() && !cascade {
            return Err(DbError::InvalidOperation(
                format!("Cannot revoke: {} dependent grants exist. Use CASCADE.", dependent_grants.len())
            ))));
        }

        // Revoke dependent grants if cascade
        if cascade {
            for dep_grant_id in dependent_grants {
                let sub_revoked = self.revoke_grant(&dep_grant_id, true)?;
                revoked_grants.extend(sub_revoked);
            }
        }

        // Remove the grant
        self.remove_grant(grant_id)?;

        Ok(revoked_grants)
    }

    /// Check if a principal has a system privilege
    pub fn check_system_privilege(
        &self,
        principal: &PrincipalId,
        privilege: &SystemPrivilege,
    ) -> PrivilegeCheckResult {
        // Check direct grants
        let grantee_index = self.grantee_index.read();
        if let Some(grant_ids) = grantee_index.get(principal) {
            let grants = self.grants.read();

            for grant_id in grant_ids {
                if let Some(grant) = grants.get(grant_id) {
                    if !grant.is_active {
                        continue;
                    }

                    // Check expiration
                    if let Some(expires_at) = grant.expires_at {
                        if current_timestamp() >= expires_at {
                            continue;
                        }
                    }

                    if let PrivilegeType::System(ref p) = grant.privilege_type {
                        if p == privilege {
                            return PrivilegeCheckResult {
                                has_privilege: true,
                                source: Some(PrivilegeSource::DirectGrant {
                                    grant_id: grant_id.clone()
                                }),
                                can_grant: grant.with_grant_option,
                            };
                        }
                    }
                }
            }
        }

        // Check through role hierarchy
        let result = self.check_privilege_through_roles(principal, privilege);
        if result.has_privilege {
            return result;
        }

        PrivilegeCheckResult {
            has_privilege: false,
            source: None,
            can_grant: false,
        }
    }

    /// Check if a principal has an object privilege
    pub fn check_object_privilege(
        &self,
        principal: &PrincipalId,
        privilege: &ObjectPrivilege,
        object_type: &PrivilegeObjectType,
        object_id: &ObjectId,
        columns: Option<&Vec<String>>,
    ) -> PrivilegeCheckResult {
        // Get grants for this object
        let object_index = self.object_index.read();
        let grant_ids = match object_index.get(object_id) {
            Some(ids) => ids,
            None => return PrivilegeCheckResult {
                has_privilege: false,
                source: None,
                can_grant: false,
            },
        };

        let grants = self.grants.read();
        let grantee_index = self.grantee_index.read();

        // Get principal's grants
        let principal_grants = grantee_index.get(principal);

        if let Some(principal_grant_ids) = principal_grants {
            for grant_id in grant_ids {
                if !principal_grant_ids.contains(grant_id) {
                    continue;
                }

                if let Some(grant) = grants.get(grant_id) {
                    if !grant.is_active {
                        continue;
                    }

                    // Check expiration
                    if let Some(expires_at) = grant.expires_at {
                        if current_timestamp() >= expires_at {
                            continue;
                        }
                    }

                    if let PrivilegeType::Object {
                        privilege: ref p,
                        object_type: ref ot,
                        ref object_id,
                        columns: ref grant_columns,
                    } = grant.privilege_type {
                        if ot != object_type {
                            continue;
                        }

                        // Check privilege match (All includes everything)
                        let privilege_matches = p == privilege || p == &ObjectPrivilege::All;

                        if !privilege_matches {
                            continue;
                        }

                        // Check column-level privileges if specified
                        if let Some(required_cols) = columns {
                            if let Some(grant_cols) = grant_columns {
                                // Must have all required columns
                                if !required_cols.iter().all(|c| grant_cols.contains(c)) {
                                    continue;
                                }
                            }
                            // If grant has no column restriction, it covers all columns
                        }

                        return PrivilegeCheckResult {
                            has_privilege: true,
                            source: Some(PrivilegeSource::DirectGrant {
                                grant_id: grant_id.clone()
                            }),
                            can_grant: grant.with_grant_option,
                        };
                    }
                }
            }
        }

        PrivilegeCheckResult {
            has_privilege: false,
            source: None,
            can_grant: false,
        }
    }

    /// Get all grants for a principal
    pub fn get_principal_grants(&self, principal: &PrincipalId) -> Vec<PrivilegeGrant> {
        let grantee_index = self.grantee_index.read();
        let grants = self.grants.read();

        if let Some(grant_ids) = grantee_index.get(principal) {
            grant_ids.iter()
                .filter_map(|id| grants.get(id))
                .filter(|g| g.is_active)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all grants for an object
    pub fn get_object_grants(&self, objectid: &ObjectId) -> Vec<PrivilegeGrant> {
        let object_index = self.object_index.read();
        let grants = self.grants.read();

        if let Some(grant_ids) = object_index.get(object_id) {
            grant_ids.iter()
                .filter_map(|id| grants.get(id))
                .filter(|g| g.is_active)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Set role hierarchy for privilege inheritance
    pub fn set_role_hierarchy(&self, role: String, parent_roles: Vec<String>) {
        self.role_hierarchy.write().insert(role, parent_roles);
    }

    /// Get effective privileges for a principal (including role inheritance)
    pub fn get_effective_privileges(&self, principal: &PrincipalId) -> EffectivePrivileges {
        let mut system_privileges = HashSet::new();
        let mut object_privileges: HashMap<ObjectId, Vec<(ObjectPrivilege, PrivilegeObjectType)>> = HashMap::new();

        // Get direct grants
        let grants = self.get_principal_grants(principal);

        for grant in grants {
            match grant.privilege_type {
                PrivilegeType::System(priv_type) => {
                    system_privileges.insert(priv_type);
                }
                PrivilegeType::Object { privilege, object_type, ref object_id, .. } => {
                    object_privileges.entry(object_id.clone())
                        .or_insert_with(Vec::new)
                        .push((privilege, object_type));
                }
            }
        }

        // Add privileges from roles
        let role_privs = self.get_privileges_through_roles(principal);
        system_privileges.extend(role_privs.system_privileges);
        for (obj_id, privs) in role_privs.object_privileges {
            object_privileges.entry(obj_id)
                .or_insert_with(Vec::new)
                .extend(privs);
        }

        EffectivePrivileges {
            system_privileges,
            object_privileges,
        }
    }

    /// Get privilege statistics
    pub fn get_statistics(&self) -> PrivilegeStatistics {
        let grants = self.grants.read();

        let total_grants = grants.len();
        let active_grants = grants.values().filter(|g| g.is_active).count();
        let grants_with_grant_option = grants.values()
            .filter(|g| g.is_active && g.with_grant_option)
            .count();

        let system_privilege_grants = grants.values()
            .filter(|g| matches!(g.privilege_type, PrivilegeType::System(_)))
            .count();

        let object_privilege_grants = grants.values()
            .filter(|g| matches!(g.privilege_type, PrivilegeType::Object { .. }))
            .count();

        PrivilegeStatistics {
            total_grants,
            active_grants,
            grants_with_grant_option,
            system_privilege_grants,
            object_privilege_grants,
        }
    }

    // Private helper methods

    fn add_grant(&self, grant: PrivilegeGrant) -> Result<()> {
        let grant_id = grant.grant_id.clone();
        let grantee = grant.grantee.clone();
        let grantor = grant.grantor.clone();

        self.grants.write().insert(grant_id.clone(), grant);

        self.grantee_index.write()
            .entry(grantee)
            .or_insert_with(HashSet::new)
            .insert(grant_id.clone());

        self.grantor_index.write()
            .entry(grantor)
            .or_insert_with(HashSet::new)
            .insert(grant_id);

        Ok(())
    }

    fn remove_grant(&self, grant_id: &str) -> Result<()> {
        let grant = {
            let mut grants = self.grants.write();
            grants.remove(grant_id)
                .ok_or_else(|| DbError::NotFound("Grant not found".to_string()))?
        };

        // Remove from indexes
        if let Some(grantee_grants) = self.grantee_index.write().get_mut(&grant.grantee) {
            grantee_grants.remove(grant_id);
        }

        if let Some(grantor_grants) = self.grantor_index.write().get_mut(&grant.grantor) {
            grantor_grants.remove(grant_id);
        }

        if let PrivilegeType::Object { ref object_id, .. } = grant.privilege_type {
            if let Some(object_grants) = self.object_index.write().get_mut(object_id) {
                object_grants.remove(grant_id);
            }
        }

        Ok(())
    }

    fn find_dependent_grants(&self, grant_id: &str) -> Vec<String> {
        let dependencies = self.dependencies.read();
        dependencies.iter()
            .filter(|d| d.depends_on_grant_id == grant_id)
            .map(|d| d.dependent_grant_id.clone())
            .collect()
    }

    fn check_privilege_through_roles(
        &self,
        principal: &PrincipalId,
        privilege: &SystemPrivilege,
    ) -> PrivilegeCheckResult {
        let role_hierarchy = self.role_hierarchy.read();

        // BFS through role hierarchy
        let mut to_check = vec![principal.clone()];
        let mut visited = HashSet::new();

        while let Some(role) = to_check.pop() {
            if visited.contains(&role) {
                continue;
            }
            visited.insert(role.clone());

            // Check if this role has the privilege
            let grantee_index = self.grantee_index.read();
            if let Some(grant_ids) = grantee_index.get(&role) {
                let grants = self.grants.read();

                for grant_id in grant_ids {
                    if let Some(grant) = grants.get(grant_id) {
                        if !grant.is_active {
                            continue;
                        }

                        if let PrivilegeType::System(ref p) = grant.privilege_type {
                            if p == privilege {
                                return PrivilegeCheckResult {
                                    has_privilege: true,
                                    source: Some(PrivilegeSource::RoleInheritance {
                                        role_id: role.clone(),
                                        grant_id: grant_id.clone(),
                                    }),
                                    can_grant: grant.with_grant_option,
                                };
                            }
                        }
                    }
                }
            }

            // Add parent roles to check
            if let Some(parents) = role_hierarchy.get(&role) {
                to_check.extend(parents.clone());
            }
        }

        PrivilegeCheckResult {
            has_privilege: false,
            source: None,
            can_grant: false,
        }
    }

    fn get_privileges_through_roles(&self, principal: &PrincipalId) -> EffectivePrivileges {
        let mut system_privileges = HashSet::new();
        let mut object_privileges: HashMap<ObjectId, Vec<(ObjectPrivilege, PrivilegeObjectType)>> = HashMap::new();

        let role_hierarchy = self.role_hierarchy.read();
        let mut to_check = vec![principal.clone()];
        let mut visited = HashSet::new();

        while let Some(role) = to_check.pop() {
            if visited.contains(&role) {
                continue;
            }
            visited.insert(role.clone());

            let grants = self.get_principal_grants(&role);
            for grant in grants {
                match grant.privilege_type {
                    PrivilegeType::System(priv_type) => {
                        system_privileges.insert(priv_type);
                    }
                    PrivilegeType::Object { privilege, object_type, ref object_id, .. } => {
                        object_privileges.entry(object_id.clone())
                            .or_insert_with(Vec::new)
                            .push((privilege, object_type));
                    }
                }
            }

            if let Some(parents) = role_hierarchy.get(&role) {
                to_check.extend(parents.clone());
            }
        }

        EffectivePrivileges {
            system_privileges,
            object_privileges,
        }
    }

    fn generate_grant_id(&self) -> String {
        let mut counter = self.grant_counter.write();
        *counter += 1;
        format!("GRANT_{:08}", *counter)
    }
}

impl Default for PrivilegeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Effective privileges for a principal
#[derive(Debug, Clone)]
pub struct EffectivePrivileges {
    pub system_privileges: HashSet<SystemPrivilege>,
    pub object_privileges: HashMap<ObjectId, Vec<(ObjectPrivilege, PrivilegeObjectType)>>,
}

/// Privilege statistics
#[derive(Debug, Clone)]
pub struct PrivilegeStatistics {
    pub total_grants: usize,
    pub active_grants: usize,
    pub grants_with_grant_option: usize,
    pub system_privilege_grants: usize,
    pub object_privilege_grants: usize,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*));

    #[test]
    fn test_grant_system_privilege() {
        let manager = PrivilegeManager::new();

        let grant_id = manager.grant_system_privilege(
            "SYSTEM".to_string(),
            "user1".to_string(),
            SystemPrivilege::CreateTable,
            false,
        ).unwrap();

        assert!(grant_id.starts_with("GRANT_"));
    }

    #[test]
    fn test_check_privilege() {
        let manager = PrivilegeManager::new();

        manager.grant_system_privilege(
            "SYSTEM".to_string(),
            "user1".to_string(),
            SystemPrivilege::CreateTable,
            false,
        ).unwrap();

        let result = manager.check_system_privilege(
            &"user1".to_string(),
            &SystemPrivilege::CreateTable,
        );

        assert!(result.has_privilege);
        assert!(!result.can_grant);
    }

    #[test]
    fn test_revoke_privilege() {
        let manager = PrivilegeManager::new();

        let grant_id = manager.grant_system_privilege(
            "SYSTEM".to_string(),
            "user1".to_string(),
            SystemPrivilege::CreateTable,
            false,
        ).unwrap();

        assert!(manager.revoke_grant(&grant_id, false).is_ok());

        let result = manager.check_system_privilege(
            &"user1".to_string(),
            &SystemPrivilege::CreateTable,
        );

        assert!(!result.has_privilege);
    }
}
