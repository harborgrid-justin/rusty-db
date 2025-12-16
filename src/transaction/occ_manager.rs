// Optimistic Concurrency Control - Compatibility re-exports from occ.rs
//
// This module provides backward compatibility by re-exporting the more
// comprehensive OCC implementation from occ.rs.
//
// The occ.rs module contains the full three-phase OCC protocol with
// multiple validation strategies and comprehensive statistics.

pub use super::occ::{
    Key, OccConfig, OccManager as OptimisticConcurrencyControl, OccStatistics as OCCStats,
    OccStats, OccTransaction, TxnId, TxnState, ValidationStrategy, Value, Version,
};

// Re-export the main manager type
pub use super::occ::OccManager;
