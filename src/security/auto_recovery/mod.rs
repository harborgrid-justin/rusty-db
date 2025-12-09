// # Auto-Recovery Module
//
// Comprehensive auto-recovery system with modular components.

pub mod recovery_strategies;
pub mod checkpoint_management;
pub mod state_restoration;
pub mod manager;

// Re-export main types
pub use recovery_strategies::{
    FailureSeverity, FailureType, RecoveryState, RecoveryStrategy,
    DetectedFailure, RecoveryPlan, RecoveryResult,
    CrashDetector, CrashStats, ProcessHealth,
    TransactionRollbackManager, TransactionState, TransactionOperation, RollbackStats,
    CorruptionDetector, PageCorruption, CorruptionStats,
    DataRepairer, ReplicaInfo, RepairStats,
};

pub use checkpoint_management::{
    StateSnapshotManager, Snapshot, SnapshotStats,
};

pub use state_restoration::{
    HealthMonitor, HealthScore, HealthMetrics,
    SelfHealer, HealingAction, HealingStats,
};

pub use manager::{
    AutoRecoveryManager, AutoRecoveryConfig,
    RecoveryStatistics, ComprehensiveRecoveryStats,
};
