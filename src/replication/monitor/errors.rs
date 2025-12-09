// Health monitoring specific errors

use thiserror::Error;

/// Health monitoring specific errors
#[derive(Error, Debug)]
pub enum HealthMonitorError {
    #[error("Replica not found: {replica_id}")]
    ReplicaNotFound { replica_id: String },

    #[error("Health check failed: {replica_id} - {reason}")]
    HealthCheckFailed { replica_id: String, reason: String },

    #[error("Metrics collection failed: {reason}")]
    MetricsCollectionFailed { reason: String },

    #[error("Alert delivery failed: {reason}")]
    AlertDeliveryFailed { reason: String },

    #[error("Invalid health threshold: {reason}")]
    InvalidThreshold { reason: String },

    #[error("Monitoring configuration error: {reason}")]
    ConfigurationError { reason: String },
}
