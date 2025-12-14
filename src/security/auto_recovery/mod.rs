// # Auto-Recovery Module
//
// Comprehensive auto-recovery system with modular components.

pub mod checkpoint_management;
pub mod manager;
pub mod recovery_strategies;
pub mod state_restoration;

// Re-export main types
pub use recovery_strategies::{
    CorruptionDetector, CorruptionStats, CrashDetector, CrashStats, DataRepairer, DetectedFailure,
    FailureSeverity, FailureType, PageCorruption, ProcessHealth, RecoveryPlan, RecoveryResult,
    RecoveryState, RecoveryStrategy, RepairStats, ReplicaInfo, RollbackStats, TransactionOperation,
    TransactionRollbackManager, TransactionState,
};

pub use checkpoint_management::{Snapshot, SnapshotStats, StateSnapshotManager};

pub use state_restoration::{
    HealingAction, HealingStats, HealthMetrics, HealthMonitor, HealthScore, SelfHealer,
};

pub use manager::{
    AutoRecoveryConfig, AutoRecoveryManager, ComprehensiveRecoveryStats, RecoveryStatistics,
};
