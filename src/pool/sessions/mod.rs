// Session management module
//
// This module provides comprehensive session management including:
// - Session state management
// - Authentication
// - Resource control
// - Pool coordination
// - Lifecycle events
// - Main session manager

pub mod auth;
pub mod coordination;
pub mod events;
pub mod manager;
pub mod resources;
pub mod state;

// Re-export core types
pub use state::{
    ClientInfo, CursorId, CursorSharingMode, CursorState, CursorStatus, IsolationLevel,
    OptimizerMode, PreparedStatement, ResourceUsage, SessionSettings, SessionState, SessionStatus,
    StatementId, TransactionState, SID,
};

// Re-export auth types
pub use auth::{
    AuthConfig, AuthMethod, AuthenticationProvider, AuthenticationResult, Credentials,
    PrivilegeCache, RoleManager, SessionCredentials,
};

// Re-export resource types
pub use resources::{
    ConsumerGroup, ResourceController, ResourceGovernor, ResourceLimits, ResourcePlan,
};

// Re-export coordination types
pub use coordination::{
    PoolConfig, PoolStatistics, PooledSession, SessionAffinity, SessionPool, SessionSelector,
    SessionTag,
};

// Re-export event types
pub use events::{
    EventHandler, EventManager, IdleTimeout, LoginTrigger, LogoffTrigger, PurityLevel,
    SessionCallback, SessionEvent, SessionEventManager, SessionTrigger,
};

// Re-export manager
pub use manager::{SessionConfig, SessionManager};
