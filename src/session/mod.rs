// # Session Management Module
//
// Enterprise-grade session management system providing advanced session lifecycle management,
// security controls, and comprehensive analytics for RustyDB.
//
// ## Module Structure
//
// - `session_manager`: Core session lifecycle management
// - `session_security`: Security controls and protection
// - `session_analytics`: Session monitoring and analytics
//
// ## Features
//
// - Session lifecycle management (creation, migration, termination)
// - State persistence and recovery
// - Advanced security controls (token management, hijacking prevention)
// - Comprehensive session analytics and monitoring
// - Resource tracking and management
// - Multi-node session migration support

pub mod session_manager;
pub mod session_security;
pub mod session_analytics;

// Re-export main types
pub use session_manager::{
    SessionManager, SessionState, SessionContext, SessionConfig,
    SessionStatus, MigrationTarget,
};

pub use session_security::{
    SessionSecurity, SessionToken, SecurityPolicy, SecurityEvent,
    TokenType, IpBindingPolicy,
};

pub use session_analytics::{
    SessionAnalytics, SessionMetrics, ActivityLog, ResourceMetrics,
    BehaviorPattern, UserActivity,
};
