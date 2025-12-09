// Replication slot specific errors

use thiserror::Error;

// Replication slot specific errors
#[derive(Error, Debug)]
pub enum SlotError {
    #[error("Slot not found: {slot_name}")]
    SlotNotFound { slot_name: String },

    #[error("Slot already exists: {slot_name}")]
    SlotAlreadyExists { slot_name: String },

    #[error("Invalid slot name: {name} - {reason}")]
    InvalidSlotName { name: String, reason: String },

    #[error("Slot is active and cannot be modified: {slot_name}")]
    SlotActive { slot_name: String },

    #[error("Slot lag exceeded threshold: {lag_bytes} bytes, threshold: {threshold_bytes} bytes")]
    LagExceeded { lag_bytes: u64, threshold_bytes: u64 },

    #[error("Slot consumption failed: {slot_name} - {reason}")]
    ConsumptionFailed { slot_name: String, reason: String },

    #[error("Slot write failed: {slot_name} - {reason}")]
    WriteFailed { slot_name: String, reason: String },

    #[error("Invalid LSN: {lsn} for slot {slot_name}")]
    InvalidLsn { lsn: String, slot_name: String },

    #[error("Slot state corruption: {slot_name} - {reason}")]
    StateCorruption { slot_name: String, reason: String },

    #[error("Too many slots: current={current}, max={max}")]
    TooManySlots { current: usize, max: usize },

    #[error("Slot type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Invalid slot configuration: {reason}")]
    InvalidConfiguration { reason: String },
}
