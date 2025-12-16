// Connection Management Module
//
// Connection state machine, metadata, and lifecycle management

use std::time::SystemTime;

// ============================================================================
// Connection State Machine
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Authenticated,
    Closing,
    Closed,
    Authenticating,
    Ready,
}

impl ConnectionState {
    /// Check if the connection can transition to a target state
    pub fn can_transition_to(&self, target: ConnectionState) -> bool {
        match (self, target) {
            // From Connecting
            (ConnectionState::Connecting, ConnectionState::Connected) => true,
            (ConnectionState::Connecting, ConnectionState::Authenticating) => true,
            (ConnectionState::Connecting, ConnectionState::Closed) => true,

            // From Connected
            (ConnectionState::Connected, ConnectionState::Authenticating) => true,
            (ConnectionState::Connected, ConnectionState::Authenticated) => true,
            (ConnectionState::Connected, ConnectionState::Closing) => true,
            (ConnectionState::Connected, ConnectionState::Closed) => true,

            // From Authenticating
            (ConnectionState::Authenticating, ConnectionState::Authenticated) => true,
            (ConnectionState::Authenticating, ConnectionState::Ready) => true,
            (ConnectionState::Authenticating, ConnectionState::Closing) => true,
            (ConnectionState::Authenticating, ConnectionState::Closed) => true,

            // From Authenticated
            (ConnectionState::Authenticated, ConnectionState::Ready) => true,
            (ConnectionState::Authenticated, ConnectionState::Closing) => true,
            (ConnectionState::Authenticated, ConnectionState::Closed) => true,

            // From Ready
            (ConnectionState::Ready, ConnectionState::Closing) => true,
            (ConnectionState::Ready, ConnectionState::Closed) => true,

            // From Closing
            (ConnectionState::Closing, ConnectionState::Closed) => true,

            // From Closed - no transitions allowed
            (ConnectionState::Closed, _) => false,

            // Same state transitions not allowed
            _ if self == &target => false,

            // All other transitions not allowed
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub client_version: String,
    pub connected_at: SystemTime,
}

impl Default for ConnectionMetadata {
    fn default() -> Self {
        Self {
            client_version: "unknown".to_string(),
            connected_at: SystemTime::now(),
        }
    }
}

pub struct ConnectionStateMachine {
    state: ConnectionState,
    metadata: ConnectionMetadata,
}

impl ConnectionStateMachine {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Connecting,
            metadata: ConnectionMetadata::default(),
        }
    }

    pub fn with_metadata(metadata: ConnectionMetadata) -> Self {
        Self {
            state: ConnectionState::Connecting,
            metadata,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn metadata(&self) -> &ConnectionMetadata {
        &self.metadata
    }

    /// Attempt to transition to a new state
    pub fn transition(&mut self, target: ConnectionState) -> Result<(), String> {
        if self.state.can_transition_to(target) {
            self.state = target;
            Ok(())
        } else {
            Err(format!(
                "Invalid transition from {:?} to {:?}",
                self.state, target
            ))
        }
    }
}

impl Default for ConnectionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: ConnectionState,
    pub to: ConnectionState,
}

#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub active_connections: usize,
    pub total_connections: u64,
}

pub struct ConnectionMigrator;

impl ConnectionMigrator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConnectionMigrator {
    fn default() -> Self {
        Self::new()
    }
}
