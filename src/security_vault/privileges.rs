// # Privilege Analysis and Management
//
// Oracle-like privilege analysis system for detecting unused privileges,
// analyzing privilege paths, and providing least privilege recommendations.
//
// ## Features
//
// - **Least Privilege Analysis**: Identify minimum required privileges
// - **Privilege Path Analysis**: Trace how users get specific privileges
// - **Unused Privilege Detection**: Find granted but unused privileges
// - **Role Mining**: Discover optimal role definitions from usage patterns
// - **Privilege Escalation Detection**: Identify potential privilege escalation
//
// ## Privilege Hierarchy
//
// ```text
// ┌─────────────────────────────────────────┐
// │  System Privileges                      │
// │  (CREATE USER, DROP ANY TABLE, etc.)    │
// └──────────────┬──────────────────────────┘
//                │
//                ▼
// ┌─────────────────────────────────────────┐
// │  Object Privileges                      │
// │  (SELECT, INSERT, UPDATE on TABLE)      │
// └──────────────┬──────────────────────────┘
//                │
//                ▼
// ┌─────────────────────────────────────────┐
// │  Roles                                  │
// │  (Collection of privileges)             │
// └─────────────────────────────────────────┘
// ```

use std::collections::HashSet;
use crate::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use parking_lot::RwLock;

/// Privilege type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrivilegeType {
    /// System privilege (e.g., CREATE TABLE, CREATE USER)
    System(String),
    /// Object privilege (e.g., SELECT, INSERT on a table)
    Object {
        privilege: String,
        object_type: String,
        object_name: String,
    },
    /// Role privilege
    Role(String),
}

impl PrivilegeType {
    /// Get privilege name
    pub fn name(&self) -> String {
        match self {
            Self::System(name) => name.clone(),
            Self::Object { privilege, object_name, .. } => {
                format!("{} ON {}", privilege, object_name)
            }
            Self::Role(name) => format!("ROLE {}", name),
        }
    }

    /// Get severity level
    pub fn severity(&self) -> u8 {
        match self {
            Self::System(name) => {
                if name.contains("ANY") || name.contains("DROP") {
                    10 // Critical
                } else if name.starts_with("CREATE") {
                    7 // High
                } else {
                    5 // Medium
                }
            }
            Self::Object { privilege, .. } => {
                match privilege.to_uppercase().as_str() {
                    "DELETE" | "DROP" => 8,
                    "INSERT" | "UPDATE" => 6,
                    "SELECT" => 3,
                    _ => 5,
                }
            }
            Self::Role(_) => 5,
        }
    }
}

/// Privilege grant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeGrant {
    /// Grantee (user or role)
    pub grantee: String,
    /// Privilege being granted
    pub privilege: PrivilegeType,
    /// Grantor (who granted it)
    pub grantor: String,
    /// Grant timestamp
    pub granted_at: i64,
    /// With admin option (for system privileges)
    pub admin_option: bool,
    /// With grant option (for object privileges)
    pub grant_option: bool,
    /// Last used timestamp
    pub last_used: Option<i64>,
    /// Usage count
    pub usage_count: u64,
}

impl PrivilegeGrant {
    /// Create a new privilege grant
    pub fn new(grantee: String, privilege: PrivilegeType, grantor: String) -> Self {
        Self {
            grantee,
            privilege,
            grantor,
            granted_at: chrono::Utc::now().timestamp(),
            admin_option: false,
            grant_option: false,
            last_used: None,
            usage_count: 0,
        }
    }

    /// Check if privilege is unused
    pub fn is_unused(&self, threshold_days: u32) -> bool {
        if let Some(last_used) = self.last_used {
            let threshold = chrono::Utc::now().timestamp() - (threshold_days as i64 * 86400));
            last_used < threshold
        } else {
            // Never used
            true
        }
    }
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Privileges in this role
    pub privileges: Vec<PrivilegeType>,
    /// Parent roles (role hierarchy)
    pub parent_roles: Vec<String>,
    /// Created timestamp
    pub created_at: i64,
    /// Enabled flag
    pub enabled: bool,
}

impl Role {
    /// Create a new role
    pub fn new(name: String) -> Self {
        Self {
            name,
            privileges: Vec::new(),
            parent_roles: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
            enabled: true,
        }
    }

    /// Add privilege to role
    pub fn add_privilege(&mut self, privilege: PrivilegeType) {
        if !self.privileges.contains(&privilege) {
            self.privileges.push(privilege);
        }
    }

    /// Add parent role
    pub fn add_parent(&mut self, parent: String) {
        if !self.parent_roles.contains(&parent) {
            self.parent_roles.push(parent);
        }
    }
}

/// Privilege path showing how a user got a privilege
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegePath {
    /// User ID
    pub user_id: String,
    /// Target privilege
    pub privilege: PrivilegeType,
    /// Path components (user -> role1 -> role2 -> privilege)
    pub path: Vec<String>,
    /// Direct grant or via role
    pub is_direct: bool,
}

/// Privilege recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivilegeRecommendation {
    /// Revoke unused privilege
    RevokeUnused {
        user_id: String,
        privilege: PrivilegeType,
        last_used: Option<i64>,
    },
    /// Grant missing privilege (based on usage pattern)
    GrantMissing {
        user_id: String,
        privilege: PrivilegeType,
        reason: String,
    },
    /// Replace direct grants with role
    ConsolidateToRole {
        user_id: String,
        privileges: Vec<PrivilegeType>,
        suggested_role: String,
    },
    /// Create new role
    CreateRole {
        role_name: String,
        privileges: Vec<PrivilegeType>,
        users: Vec<String>,
    },
    /// Potential privilege escalation
    PrivilegeEscalation {
        user_id: String,
        privilege: PrivilegeType,
        risk_level: u8,
        reason: String,
    },
}

/// Main Privilege Analyzer
pub struct PrivilegeAnalyzer {
    /// User privilege grants
    user_grants: RwLock<HashMap<String, Vec<PrivilegeGrant>>>,
    /// Role definitions
    roles: RwLock<HashMap<String, Role>>,
    /// User to role mappings
    user_roles: RwLock<HashMap<String, Vec<String>>>,
    /// Privilege usage tracking
    usage_log: RwLock<Vec<PrivilegeUsage>>,
    /// Analysis statistics
    stats: RwLock<AnalysisStats>,
}

/// Privilege usage record
#[derive(Debug, Clone)]
struct PrivilegeUsage {
    user_id: String,
    privilege: PrivilegeType,
    timestamp: i64,
}

/// Analysis statistics
#[derive(Debug, Default)]
struct AnalysisStats {
    total_grants: u64,
    unused_privileges: u64,
    excessive_privileges: u64,
    recommendations_generated: u64,
}

impl PrivilegeAnalyzer {
    /// Create a new privilege analyzer
    pub fn new() -> Result<Self> {
        Ok(Self {
            user_grants: RwLock::new(HashMap::new()),
            roles: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
            usage_log: RwLock::new(Vec::new()),
            stats: RwLock::new(AnalysisStats::default()),
        })
    }

    /// Grant a privilege to a user
    pub fn grant_privilege(
        &mut self,
        user_id: &str,
        privilege: PrivilegeType,
        grantor: &str,
    ) -> Result<()> {
        let grant = PrivilegeGrant::new(
            user_id.to_string(),
            privilege,
            grantor.to_string(),
        );

        let mut grants = self.user_grants.write();
        grants.entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(grant);

        self.stats.write().total_grants += 1;

        Ok(())
    }

    /// Revoke a privilege from a user
    pub fn revoke_privilege(
        &mut self,
        user_id: &str,
        privilege: &PrivilegeType,
    ) -> Result<()> {
        let mut grants = self.user_grants.write();

        if let Some(user_grants) = grants.get_mut(user_id) {
            user_grants.retain(|g| &g.privilege != privilege);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("No grants found for user: {}", user_id)))
        }
    }

    /// Create a role
    pub fn create_role(&mut self, name: &str) -> Result<()> {
        let role = Role::new(name.to_string()));
        self.roles.write().insert(name.to_string(), role);
        Ok(())
    }

    /// Add privilege to role
    pub fn grant_to_role(&mut self, role_name: &str, privilege: PrivilegeType) -> Result<()> {
        let mut roles = self.roles.write();
        let role = roles.get_mut(role_name)
            .ok_or_else(|| DbError::NotFound(format!("Role not found: {}", role_name)))?;

        role.add_privilege(privilege);
        Ok(())
    }

    /// Grant role to user
    pub fn grant_role(&mut self, user_id: &str, role_name: &str) -> Result<()> {
        // Verify role exists
        if !self.roles.read().contains_key(role_name) {
            return Err(DbError::NotFound(format!("Role not found: {}", role_name))));
        }

        self.user_roles.write()
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(role_name.to_string());

        Ok(())
    }

    /// Revoke role from user
    pub fn revoke_role(&mut self, user_id: &str, role_name: &str) -> Result<()> {
        let mut user_roles = self.user_roles.write();

        if let Some(roles) = user_roles.get_mut(user_id) {
            roles.retain(|r| r != role_name);
            Ok(())
        } else {
            Err(DbError::NotFound(format!("User has no roles: {}", user_id)))
        }
    }

    /// Record privilege usage
    pub fn record_usage(&mut self, user_id: &str, privilege: PrivilegeType) {
        // Update usage in grants
        let mut grants = self.user_grants.write());
        if let Some(user_grants) = grants.get_mut(user_id) {
            for grant in user_grants.iter_mut() {
                if grant.privilege == privilege {
                    grant.last_used = Some(chrono::Utc::now().timestamp());
                    grant.usage_count += 1;
                }
            }
        }

        // Log usage
        self.usage_log.write().push(PrivilegeUsage {
            user_id: user_id.to_string(),
            privilege,
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    /// Analyze privileges for a user
    pub fn analyze_user(&self, user_id: &str) -> Result<Vec<PrivilegeRecommendation>> {
        let mut recommendations = Vec::new();

        // Get all user privileges (direct + via roles)
        let all_privileges = self.get_user_privileges(user_id);

        // Check for unused privileges
        let grants = self.user_grants.read();
        if let Some(user_grants) = grants.get(user_id) {
            for grant in user_grants {
                if grant.is_unused(90) {
                    recommendations.push(PrivilegeRecommendation::RevokeUnused {
                        user_id: user_id.to_string(),
                        privilege: grant.privilege.clone(),
                        last_used: grant.last_used,
                    });

                    self.stats.write().unused_privileges += 1;
                }
            }
        }

        // Check for excessive privileges
        for privilege in &all_privileges {
            if privilege.severity() >= 9 {
                recommendations.push(PrivilegeRecommendation::PrivilegeEscalation {
                    user_id: user_id.to_string(),
                    privilege: privilege.clone(),
                    risk_level: privilege.severity(),
                    reason: "High-risk privilege detected".to_string(),
                });

                self.stats.write().excessive_privileges += 1;
            }
        }

        // Check for consolidation opportunities
        if let Some(consolidation) = self.find_consolidation_opportunity(user_id) {
            recommendations.push(consolidation);
        }

        self.stats.write().recommendations_generated += recommendations.len() as u64;

        Ok(recommendations)
    }

    /// Get all privileges for a user (direct + via roles)
    pub fn get_user_privileges(&self, user_id: &str) -> Vec<PrivilegeType> {
        let mut privileges = HashSet::new();

        // Direct grants
        let grants = self.user_grants.read();
        if let Some(user_grants) = grants.get(user_id) {
            for grant in user_grants {
                privileges.insert(grant.privilege.clone());
            }
        }

        // Privileges via roles
        let user_roles = self.user_roles.read();
        if let Some(roles) = user_roles.get(user_id) {
            let role_defs = self.roles.read();
            for role_name in roles {
                if let Some(role) = role_defs.get(role_name) {
                    for privilege in &role.privileges {
                        privileges.insert(privilege.clone());
                    }

                    // Recursively get privileges from parent roles
                    for parent in &role.parent_roles {
                        if let Some(parent_role) = role_defs.get(parent) {
                            for privilege in &parent_role.privileges {
                                privileges.insert(privilege.clone());
                            }
                        }
                    }
                }
            }
        }

        privileges.into_iter().collect()
    }

    /// Trace privilege path (how user got a privilege)
    pub fn trace_privilege_path(
        &self,
        user_id: &str,
        privilege: &PrivilegeType,
    ) -> Vec<PrivilegePath> {
        let mut paths = Vec::new();

        // Check direct grant
        let grants = self.user_grants.read();
        if let Some(user_grants) = grants.get(user_id) {
            if user_grants.iter().any(|g| &g.privilege == privilege) {
                paths.push(PrivilegePath {
                    user_id: user_id.to_string(),
                    privilege: privilege.clone(),
                    path: vec![user_id.to_string(), privilege.name()],
                    is_direct: true,
                });
            }
        }

        // Check via roles
        let user_roles = self.user_roles.read();
        if let Some(roles) = user_roles.get(user_id) {
            let role_defs = self.roles.read();
            for role_name in roles {
                if let Some(role) = role_defs.get(role_name) {
                    if role.privileges.contains(privilege) {
                        paths.push(PrivilegePath {
                            user_id: user_id.to_string(),
                            privilege: privilege.clone(),
                            path: vec![
                                user_id.to_string(),
                                format!("ROLE {}", role_name),
                                privilege.name(),
                            ],
                            is_direct: false,
                        }));
                    }
                }
            }
        }

        paths
    }

    /// Find consolidation opportunities
    fn find_consolidation_opportunity(&self, user_id: &str) -> Option<PrivilegeRecommendation> {
        let grants = self.user_grants.read();
        let user_grants = grants.get(user_id)?;

        if user_grants.len() < 3 {
            return None; // Not worth consolidating
        }

        // Check if privileges match an existing role
        let roles = self.roles.read();
        for (role_name, role) in roles.iter() {
            let grant_privs: HashSet<_> = user_grants.iter()
                .map(|g| &g.privilege)
                .collect();

            let role_privs: HashSet<_> = role.privileges.iter().collect();

            // If 80% or more privileges match, suggest consolidation
            let intersection: Vec<_> = grant_privs.intersection(&role_privs).collect();
            if intersection.len() * 100 / user_grants.len() >= 80 {
                return Some(PrivilegeRecommendation::ConsolidateToRole {
                    user_id: user_id.to_string(),
                    privileges: user_grants.iter().map(|g| g.privilege.clone()).collect(),
                    suggested_role: role_name.clone(),
                });
            }
        }

        None
    }

    /// Mine roles from usage patterns
    pub fn mine_roles(&self, minusers: usize) -> Vec<PrivilegeRecommendation> {
        let mut recommendations = Vec::new();

        // Group users by their privilege sets
        let mut privilege_sets: HashMap<Vec<PrivilegeType>, Vec<String>> = HashMap::new();

        let grants = self.user_grants.read();
        for (user_id, user_grants) in grants.iter() {
            let mut privs: Vec<PrivilegeType> = user_grants.iter()
                .map(|g| g.privilege.clone())
                .collect();
            privs.sort_by_key(|p| p.name());

            privilege_sets.entry(privs)
                .or_insert_with(Vec::new)
                .push(user_id.clone());
        }

        // Find common patterns
        for (privileges, users) in privilege_sets {
            if users.len() >= min_users && privileges.len() >= 3 {
                let role_name = format!("DISCOVERED_ROLE_{}", users.len()));
                recommendations.push(PrivilegeRecommendation::CreateRole {
                    role_name,
                    privileges,
                    users,
                });
            }
        }

        recommendations
    }

    /// Detect potential privilege escalation
    pub fn detect_escalation(&self, user_id: &str) -> Vec<PrivilegeRecommendation> {
        let mut escalations = Vec::new();

        let privileges = self.get_user_privileges(user_id);

        for privilege in privileges {
            // Check for dangerous combinations
            if privilege.severity() >= 9 {
                escalations.push(PrivilegeRecommendation::PrivilegeEscalation {
                    user_id: user_id.to_string(),
                    privilege: privilege.clone(),
                    risk_level: privilege.severity(),
                    reason: "Critical privilege detected".to_string(),
                });
            }

            // Check for ANY privileges
            if let PrivilegeType::System(name) = &privilege {
                if name.contains("ANY") {
                    escalations.push(PrivilegeRecommendation::PrivilegeEscalation {
                        user_id: user_id.to_string(),
                        privilege: privilege.clone(),
                        risk_level: 10,
                        reason: "ANY privilege allows access to all objects".to_string(),
                    });
                }
            }
        }

        escalations
    }

    /// Get analysis statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        let stats = self.stats.read();
        (
            stats.total_grants,
            stats.unused_privileges,
            stats.excessive_privileges,
            stats.recommendations_generated,
        )
    }

    /// List all users
    pub fn list_users(&self) -> Vec<String> {
        self.user_grants.read().keys().cloned().collect()
    }

    /// List all roles
    pub fn list_roles(&self) -> Vec<String> {
        self.roles.read().keys().cloned().collect()
    }

    /// Get role details
    pub fn get_role(&self, name: &str) -> Option<Role> {
        self.roles.read().get(name).cloned()
    }

    /// Get user's direct grants
    pub fn get_user_grants(&self, user_id: &str) -> Vec<PrivilegeGrant> {
        self.user_grants.read()
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get user's roles
    pub fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        self.user_roles.read()
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_privilege() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let privilege = PrivilegeType::System("CREATE TABLE".to_string());
        analyzer.grant_privilege("user1", privilege.clone(), "admin").unwrap();

        let grants = analyzer.get_user_grants("user1");
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].privilege, privilege);
    }

    #[test]
    fn test_revoke_privilege() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let privilege = PrivilegeType::System("CREATE TABLE".to_string());
        analyzer.grant_privilege("user1", privilege.clone(), "admin").unwrap();
        analyzer.revoke_privilege("user1", &privilege).unwrap();

        let grants = analyzer.get_user_grants("user1");
        assert_eq!(grants.len(), 0);
    }

    #[test]
    fn test_create_role() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        analyzer.create_role("DBA").unwrap();
        let roles = analyzer.list_roles();
        assert!(roles.contains(&"DBA".to_string()));
    }

    #[test]
    fn test_grant_role() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        analyzer.create_role("DBA").unwrap();
        let privilege = PrivilegeType::System("CREATE TABLE".to_string());
        analyzer.grant_to_role("DBA", privilege.clone()).unwrap();
        analyzer.grant_role("user1", "DBA").unwrap();

        let privileges = analyzer.get_user_privileges("user1");
        assert_eq!(privileges.len(), 1);
        assert_eq!(privileges[0], privilege);
    }

    #[test]
    fn test_privilege_path() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let privilege = PrivilegeType::System("CREATE TABLE".to_string());
        analyzer.grant_privilege("user1", privilege.clone(), "admin").unwrap();

        let paths = analyzer.trace_privilege_path("user1", &privilege);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].is_direct);
    }

    #[test]
    fn test_unused_detection() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let privilege = PrivilegeType::System("CREATE TABLE".to_string());
        analyzer.grant_privilege("user1", privilege.clone(), "admin").unwrap();

        let recommendations = analyzer.analyze_user("user1").unwrap();

        // Should recommend revoking unused privilege
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_privilege_severity() {
        let high_risk = PrivilegeType::System("DROP ANY TABLE".to_string());
        assert_eq!(high_risk.severity(), 10);

        let medium_risk = PrivilegeType::System("CREATE TABLE".to_string());
        assert!(medium_risk.severity() < 10);

        let low_risk = PrivilegeType::Object {
            privilege: "SELECT".to_string(),
            object_type: "TABLE".to_string(),
            object_name: "employees".to_string(),
        };
        assert_eq!(low_risk.severity(), 3);
    }

    #[test]
    fn test_escalation_detection() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let dangerous = PrivilegeType::System("DROP ANY TABLE".to_string());
        analyzer.grant_privilege("user1", dangerous, "admin").unwrap();

        let escalations = analyzer.detect_escalation("user1");
        assert!(!escalations.is_empty());
    }

    #[test]
    fn test_role_mining() {
        let mut analyzer = PrivilegeAnalyzer::new().unwrap();

        let priv1 = PrivilegeType::System("CREATE TABLE".to_string());
        let priv2 = PrivilegeType::System("CREATE VIEW".to_string());
        let priv3 = PrivilegeType::System("CREATE INDEX".to_string());

        // Grant same privileges to multiple users
        for i in 1..=5 {
            let user = format!("user{}", i));
            analyzer.grant_privilege(&user, priv1.clone(), "admin").unwrap();
            analyzer.grant_privilege(&user, priv2.clone(), "admin").unwrap();
            analyzer.grant_privilege(&user, priv3.clone(), "admin").unwrap();
        }

        let recommendations = analyzer.mine_roles(3);

        // Should suggest creating a role
        assert!(!recommendations.is_empty());
    }
}
