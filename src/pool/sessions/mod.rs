//! Session management module
//!
//! This module provides comprehensive session management including:
//! - Session state management
//! - Authentication
//! - Resource control
//! - Pool coordination
//! - Lifecycle events
//! - Main session manager

pub mod state;
pub mod auth;
pub mod resources;
pub mod coordination;
pub mod events;
pub mod manager;

// Re-export core types
pub use state::{
    SID, CursorId, StatementId,
    SessionState, SessionSettings, SessionStatus,
    TransactionState, IsolationLevel, OptimizerMode, CursorSharingMode,
    CursorState, CursorStatus, PreparedStatement, ClientInfo, ResourceUsage,
};

// Re-export auth types
pub use auth::{
    AuthenticationProvider, AuthMethod, AuthConfig,
    PrivilegeCache, RoleManager, SessionCredentials,
    Credentials, AuthenticationResult,
};

// Re-export resource types
pub use resources::{
    ResourceLimits, ResourceController, ResourceGovernor,
    ConsumerGroup, ResourcePlan,
};

// Re-export coordination types
pub use coordination::{
    SessionPool, SessionTag, SessionAffinity,
    PooledSession, SessionSelector,
    PoolConfig, PoolStatistics,
};

// Re-export event types
pub use events::{
    SessionEvent, EventHandler, EventManager,
    LoginTrigger, LogoffTrigger, IdleTimeout,
    SessionEventManager, SessionTrigger, SessionCallback, PurityLevel,
};

// Re-export manager
pub use manager::{
    SessionManager, SessionConfig,
};
