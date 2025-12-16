// Recovery management - Compatibility re-exports from recovery.rs
//
// This module provides backward compatibility by re-exporting the more
// comprehensive recovery implementation from recovery.rs.
//
// The recovery.rs module contains the full ARIES-style recovery manager
// with fuzzy checkpointing, point-in-time recovery, and media recovery.

pub use super::recovery::{
    ARIESRecoveryManager as RecoveryManager, CheckpointConfig, CheckpointStats,
    FuzzyCheckpointManager, MediaRecoveryManager, PointInTimeRecovery, RecoveryConfig,
    RecoveryState, RecoveryStats,
};

// Re-export for backward compatibility
pub use super::recovery::ARIESRecoveryManager;
