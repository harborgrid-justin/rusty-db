// Protocol Error Types

use std::io;
use thiserror::Error;
use super::ConnectionState;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Insufficient data in buffer")]
    InsufficientData,

    #[error("Invalid magic number")]
    InvalidMagic,

    #[error("Invalid protocol version")]
    InvalidVersion,

    #[error("Invalid message type")]
    InvalidMessageType,

    #[error("Invalid compression type")]
    InvalidCompression,

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Incompatible protocol version")]
    IncompatibleVersion,

    #[error("Unexpected message")]
    UnexpectedMessage,

    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ConnectionState,
        to: ConnectionState,
    },

    #[error("Connection not found")]
    ConnectionNotFound,

    #[error("Request timeout")]
    RequestTimeout,

    #[error("Request cancelled")]
    RequestCancelled,

    #[error("Unknown request")]
    UnknownRequest,

    #[error("Pipeline shutdown")]
    PipelineShutdown,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Stream closed")]
    StreamClosed,

    #[error("Extension already registered")]
    ExtensionAlreadyRegistered,

    #[error("Extension not found")]
    ExtensionNotFound,

    #[error("No custom message slots available")]
    NoCustomMessageSlotsAvailable,

    #[error("Flow control timeout")]
    FlowControlTimeout,

    #[error("Rate limit timeout")]
    RateLimitTimeout,
}

#[cfg(test)]
mod tests {
    use super::super::message_types::*;

    #[test]
    fn test_protocol_version_compatibility() {
        let v1_0 = ProtocolVersion::V1_0_0;
        let v1_1 = ProtocolVersion::V1_1_0;
        let v2_0 = ProtocolVersion::V2_0_0;

        assert!(v1_1.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v1_1));
        assert!(!v2_0.is_compatible_with(&v1_0));
    }

    #[test]
    fn test_packet_checksum() {
        use bytes::Bytes;
        let payload = Bytes::from("test data");
        let packet = Packet::new(MessageType::Query, payload);
        assert!(packet.verify_checksum());
    }

    #[test]
    fn test_connection_state_transitions() {
        use super::super::connection_management::ConnectionState;
        assert!(ConnectionState::Connecting.can_transition_to(ConnectionState::Authenticating));
        assert!(ConnectionState::Authenticating.can_transition_to(ConnectionState::Ready));
        assert!(!ConnectionState::Closed.can_transition_to(ConnectionState::Ready));
    }
}
