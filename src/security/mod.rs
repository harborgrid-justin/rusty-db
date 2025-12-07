use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

/// User authentication and authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    password_hash: String,
    pub roles: HashSet<String>,
    pub permissions: HashSet<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    CreateTable,
    DropTable,
    Select,
    Insert,
    Update,
    Delete,
    CreateUser,
    GrantPermission,
    CreateIndex,
    CreateView,
    Backup,
    Restore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

/// Security manager for authentication and authorization
pub struct SecurityManager {
    users: Arc<RwLock<HashMap<String, User>>>,
    roles: Arc<RwLock<HashMap<String, Role>>>,
    sessions: Arc<RwLock<HashMap<String, String>>>, // session_id -> username
}

impl SecurityManager {
    pub fn new() -> Self {
        let mut manager = Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Create default admin user
        manager.create_default_admin();
        manager.create_default_roles();
        
        manager
    }
    
    fn create_default_admin(&self) {
        let mut users = self.users.write();
        let admin = User {
            username: "admin".to_string(),
            password_hash: hash_password("admin"),
            roles: vec!["admin".to_string()].into_iter().collect(),
            permissions: HashSet::from([
                Permission::CreateTable, Permission::DropTable,
                Permission::Select, Permission::Insert, Permission::Update, Permission::Delete,
                Permission::CreateUser, Permission::GrantPermission,
                Permission::CreateIndex, Permission::CreateView,
                Permission::Backup, Permission::Restore,
            ]),
        };
        users.insert("admin".to_string(), admin);
    }
    
    fn create_default_roles(&self) {
        let mut roles = self.roles.write();
        
        // Admin role
        roles.insert("admin".to_string(), Role {
            name: "admin".to_string(),
            permissions: HashSet::from([
                Permission::CreateTable, Permission::DropTable,
                Permission::Select, Permission::Insert, Permission::Update, Permission::Delete,
                Permission::CreateUser, Permission::GrantPermission,
                Permission::CreateIndex, Permission::CreateView,
                Permission::Backup, Permission::Restore,
            ]),
        });
        
        // Read-only role
        roles.insert("reader".to_string(), Role {
            name: "reader".to_string(),
            permissions: HashSet::from([Permission::Select]),
        });
        
        // Writer role
        roles.insert("writer".to_string(), Role {
            name: "writer".to_string(),
            permissions: HashSet::from([
                Permission::Select, Permission::Insert,
                Permission::Update, Permission::Delete,
            ]),
        });
    }
    
    pub fn authenticate(&self, username: &str, password: &str) -> Result<String> {
        let users = self.users.read();
        
        if let Some(user) = users.get(username) {
            if verify_password(password, &user.password_hash) {
                let session_id = generate_session_id();
                self.sessions.write().insert(session_id.clone(), username.to_string());
                return Ok(session_id);
            }
        }
        
        Err(DbError::Network("Authentication failed".to_string()))
    }
    
    pub fn authorize(&self, session_id: &str, permission: Permission) -> Result<()> {
        let sessions = self.sessions.read();
        let username = sessions.get(session_id)
            .ok_or_else(|| DbError::Network("Invalid session".to_string()))?;
        
        let users = self.users.read();
        let user = users.get(username)
            .ok_or_else(|| DbError::Network("User not found".to_string()))?;
        
        if user.permissions.contains(&permission) {
            Ok(())
        } else {
            Err(DbError::Network("Permission denied".to_string()))
        }
    }
    
    pub fn create_user(&self, username: String, password: String, roles: HashSet<String>) -> Result<()> {
        let mut users = self.users.write();
        
        if users.contains_key(&username) {
            return Err(DbError::Network("User already exists".to_string()));
        }
        
        let mut permissions = HashSet::new();
        let roles_map = self.roles.read();
        for role_name in &roles {
            if let Some(role) = roles_map.get(role_name) {
                permissions.extend(role.permissions.iter().cloned());
            }
        }
        
        let user = User {
            username: username.clone(),
            password_hash: hash_password(&password),
            roles,
            permissions,
        };
        
        users.insert(username, user);
        Ok(())
    }
}

fn hash_password(password: &str) -> String {
    // Simple hash for demo - in production use bcrypt/argon2
    format!("hashed_{}", password)
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash == format!("hashed_{}", password)
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("session_{}", timestamp)
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_authentication() -> Result<()> {
        let sm = SecurityManager::new();
        let session = sm.authenticate("admin", "admin")?;
        assert!(!session.is_empty());
        Ok(())
    }
    
    #[test]
    fn test_authorization() -> Result<()> {
        let sm = SecurityManager::new();
        let session = sm.authenticate("admin", "admin")?;
        sm.authorize(&session, Permission::CreateTable)?;
        Ok(())
    }
}
