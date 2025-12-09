// Session lifecycle events module
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    Created,
    Terminated,
    Idle,
    Active,
}

pub struct EventHandler;
pub struct EventManager;
pub struct LoginTrigger;
pub struct LogoffTrigger;
pub struct IdleTimeout;

// Type alias for SessionEventManager
pub type SessionEventManager = EventManager;

// Session trigger type for lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionTrigger {
    BeforeLogin,
    AfterLogin,
    BeforeLogoff,
    AfterLogoff,
    OnIdle,
    OnActive,
    OnTimeout,
}

// Callback function for session events
pub type SessionCallback = Arc<dyn Fn(&SessionEvent) -> Result<()> + Send + Sync>;

// Purity level for session operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurityLevel {
    Pure,      // No side effects
    Impure,    // Has side effects
    Undefined, // Unknown
}
