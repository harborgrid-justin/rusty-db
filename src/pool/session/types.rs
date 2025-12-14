// Session domain types with strong typing
//
// This module provides newtype wrappers for session-related identifiers
// to prevent type confusion and enable compile-time safety.

use serde::{Deserialize, Serialize};
use std::fmt;

// Session identifier (SID) - unique per session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

impl SessionId {
    // Create a new session ID
    pub fn new(id: u64) -> Self {
        SessionId(id)
    }

    // Get the inner value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SID:{}", self.0)
    }
}

impl From<u64> for SessionId {
    fn from(id: u64) -> Self {
        SessionId(id)
    }
}

// Cursor identifier - unique per cursor within a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CursorId(pub u64);

impl CursorId {
    pub fn new(id: u64) -> Self {
        CursorId(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for CursorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CURSOR:{}", self.0)
    }
}

// Prepared statement identifier - unique per statement cache
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatementId(pub u64);

impl StatementId {
    pub fn new(id: u64) -> Self {
        StatementId(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for StatementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "STMT:{}", self.0)
    }
}

// Username - domain-specific string wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    // Create a new username with validation
    //
    // # Errors
    //
    // Returns error if username is empty or invalid
    pub fn new(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        if name.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if name.len() > 128 {
            return Err("Username too long (max 128 characters)".to_string());
        }
        Ok(Username(name))
    }

    // Get the username as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Schema name - domain-specific string wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaName(String);

impl SchemaName {
    pub fn new(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        if name.is_empty() {
            return Err("Schema name cannot be empty".to_string());
        }
        Ok(SchemaName(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchemaName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SchemaName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_creation() {
        let sid = SessionId::new(42);
        assert_eq!(sid.as_u64(), 42);
        assert_eq!(format!("{}", sid), "SID:42");
    }

    #[test]
    fn test_session_id_from_u64() {
        let sid: SessionId = 100.into();
        assert_eq!(sid.as_u64(), 100);
    }

    #[test]
    fn test_username_validation() {
        assert!(Username::new("alice").is_ok());
        assert!(Username::new("").is_err());
        assert!(Username::new("a".repeat(129)).is_err());
    }

    #[test]
    fn test_username_display() {
        let user = Username::new("bob").unwrap();
        assert_eq!(format!("{}", user), "bob");
        assert_eq!(user.as_str(), "bob");
    }

    #[test]
    fn test_schema_name() {
        let schema = SchemaName::new("public").unwrap();
        assert_eq!(schema.as_str(), "public");
        assert!(SchemaName::new("").is_err());
    }

    #[test]
    fn test_cursor_id() {
        let cid = CursorId::new(7);
        assert_eq!(cid.as_u64(), 7);
        assert_eq!(format!("{}", cid), "CURSOR:7");
    }

    #[test]
    fn test_statement_id() {
        let stmt = StatementId::new(99);
        assert_eq!(stmt.as_u64(), 99);
        assert_eq!(format!("{}", stmt), "STMT:99");
    }
}
