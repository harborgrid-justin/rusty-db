//! # Session Management & Pool Lifecycle System
//!
//! Enterprise-grade session management system for RustyDB with Oracle-like capabilities.
//! This module provides comprehensive session lifecycle management, authentication,
//! resource control, connection pooling, and event handling.
//!
//! ## Key Features
//!
//! - **Session State Management**: Complete session context preservation including
//!   variables, settings, transaction state, cursors, and prepared statements
//! - **Multi-Method Authentication**: LDAP, Kerberos, SAML, token-based authentication
//!   with privilege caching and role activation
//! - **Resource Control**: Per-session memory quotas, CPU limits, I/O throttling,
//!   and parallel execution control
//! - **Connection Pooling**: DRCP-like connection pooling with session multiplexing,
//!   tag-based selection, and session affinity
//! - **Lifecycle Events**: Login/logoff triggers, state change callbacks, idle timeouts,
//!   and session migration
//!
//! ## Module Organization
//!
//! The session manager has been refactored into smaller, focused modules:
//!
//! - `sessions::state` - Session state and core types
//! - `sessions::auth` - Authentication providers
//! - `sessions::resources` - Resource management and limits
//! - `sessions::coordination` - Pool coordination
//! - `sessions::events` - Lifecycle event handling
//! - `sessions::manager` - Main session manager

// Re-export everything from the sessions module
pub use crate::pool::sessions::*;
