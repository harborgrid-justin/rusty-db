// Optimistic Concurrency Control (OCC) implementation.
//
// This module provides optimistic concurrency control as an alternative
// to traditional locking. Transactions proceed without locks and validate
// at commit time.
//
// # OCC Phases
//
// 1. **Read Phase**: Execute transaction, tracking read/write sets.
// 2. **Validation Phase**: Check for conflicts with other transactions.
// 3. **Write Phase**: Apply changes if validation succeeds.
//
// # Example
//
// ```rust,ignore
// let occ = OptimisticConcurrencyControl::new();
// occ.read(txn_id, "key1".to_string())?;
// if occ.validate(txn_id) {
//     occ.write(txn_id, "key1".to_string())?;
// }
// ```

use std::fmt;
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::common::TransactionId;

use super::error::{TransactionError, TransactionResult};

/// Optimistic Concurrency Control manager.
///
/// Tracks read and write versions for validation-based
/// concurrency control.
pub struct OptimisticConcurrencyControl {
    /// Read versions: (txn_id, key) -> version at read time.
    read_versions: Arc<RwLock<HashMap<(TransactionId, String), u64>>>,
    /// Current version for each key.
    write_versions: Arc<RwLock<HashMap<String, u64>>>,
    /// Statistics.
    stats: Arc<RwLock<OCCStats>>,
}

/// OCC statistics.
#[derive(Debug, Default, Clone)]
pub struct OCCStats {
    /// Total validations performed.
    pub validations: u64,
    /// Successful validations.
    pub validations_passed: u64,
    /// Failed validations.
    pub validations_failed: u64,
    /// Total reads tracked.
    pub reads: u64,
    /// Total writes performed.
    pub writes: u64,
}

impl OptimisticConcurrencyControl {
    /// Creates a new OCC manager.
    pub fn new() -> Self {
        Self {
            read_versions: Arc::new(RwLock::new(HashMap::new())),
            write_versions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(OCCStats::default())),
        }
    }

    /// Records a read operation and returns the current version.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction ID.
    /// * `key` - The key being read.
    ///
    /// # Returns
    ///
    /// The version number of the key at read time.
    pub fn read(&self, txn_id: TransactionId, key: String) -> TransactionResult<u64> {
        let write_versions = self.write_versions.read();
        let version = write_versions.get(&key).copied().unwrap_or(0);

        let mut read_versions = self.read_versions.write();
        read_versions.insert((txn_id, key), version);

        self.stats.write().reads += 1;

        Ok(version)
    }

    /// Validates a transaction for commit.
    ///
    /// Checks if any keys read by the transaction have been modified
    /// since they were read.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction to validate.
    ///
    /// # Returns
    ///
    /// `true` if validation passes, `false` if conflicts exist.
    pub fn validate(&self, txn_id: TransactionId) -> bool {
        let read_versions = self.read_versions.read();
        let write_versions = self.write_versions.read();

        let mut stats = self.stats.write();
        stats.validations += 1;

        // Check if any read keys have been updated
        for ((tid, key), read_version) in read_versions.iter() {
            if *tid == txn_id {
                if let Some(&current_version) = write_versions.get(key) {
                    if current_version != *read_version {
                        stats.validations_failed += 1;
                        return false;
                    }
                }
            }
        }

        stats.validations_passed += 1;
        true
    }

    /// Validates and returns the conflicting key if validation fails.
    pub fn validate_with_conflict(&self, txn_id: TransactionId) -> Result<(), String> {
        let read_versions = self.read_versions.read();
        let write_versions = self.write_versions.read();

        for ((tid, key), read_version) in read_versions.iter() {
            if *tid == txn_id {
                if let Some(&current_version) = write_versions.get(key) {
                    if current_version != *read_version {
                        return Err(key.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Performs a write operation.
    ///
    /// Should be called only after successful validation.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction ID.
    /// * `key` - The key to write.
    pub fn write(&self, txn_id: TransactionId, key: String) -> TransactionResult<()> {
        // Validate before writing
        if !self.validate(txn_id) {
            return Err(TransactionError::ValidationFailed {
                txn_id,
                key: key.clone(),
            });
        }

        // Increment version
        let mut write_versions = self.write_versions.write();
        let version = write_versions.entry(key).or_insert(0);
        *version += 1;

        self.stats.write().writes += 1;

        Ok(())
    }

    /// Writes without re-validating (use after separate validation).
    pub fn write_unchecked(&self, key: String) {
        let mut write_versions = self.write_versions.write();
        let version = write_versions.entry(key).or_insert(0);
        *version += 1;
        self.stats.write().writes += 1;
    }

    /// Cleans up tracking data for a completed transaction.
    pub fn cleanup(&self, txn_id: TransactionId) {
        let mut read_versions = self.read_versions.write();
        read_versions.retain(|(tid, _), _| *tid != txn_id);
    }

    /// Gets the current version of a key.
    pub fn get_version(&self, key: &str) -> u64 {
        self.write_versions.read().get(key).copied().unwrap_or(0)
    }

    /// Returns OCC statistics.
    pub fn stats(&self) -> OCCStats {
        self.stats.read().clone()
    }

    /// Resets all state.
    pub fn clear(&self) {
        self.read_versions.write().clear();
        self.write_versions.write().clear();
    }
}

impl Default for OptimisticConcurrencyControl {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for OptimisticConcurrencyControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("OptimisticConcurrencyControl")
            .field("validations", &stats.validations)
            .field("passed", &stats.validations_passed)
            .field("failed", &stats.validations_failed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_and_validate() {
        let occ = OptimisticConcurrencyControl::new();

        // Read a key
        let version = occ.read(1, "key1".to_string()).unwrap();
        assert_eq!(version, 0);

        // Validation should pass (no concurrent modifications)
        assert!(occ.validate(1));
    }

    #[test]
    fn test_validation_failure() {
        let occ = OptimisticConcurrencyControl::new();

        // Transaction 1 reads key1
        occ.read(1, "key1".to_string()).unwrap();

        // Transaction 2 writes key1
        occ.write_unchecked("key1".to_string());

        // Transaction 1's validation should fail
        assert!(!occ.validate(1));
    }

    #[test]
    fn test_write_with_validation() {
        let occ = OptimisticConcurrencyControl::new();

        occ.read(1, "key1".to_string()).unwrap();
        let result = occ.write(1, "key1".to_string());
        assert!(result.is_ok());

        assert_eq!(occ.get_version("key1"), 1);
    }

    #[test]
    fn test_cleanup() {
        let occ = OptimisticConcurrencyControl::new();

        occ.read(1, "key1".to_string()).unwrap();
        occ.read(1, "key2".to_string()).unwrap();

        occ.cleanup(1);

        // Stats should still show reads
        let stats = occ.stats();
        assert_eq!(stats.reads, 2);
    }

    #[test]
    fn test_statistics() {
        let occ = OptimisticConcurrencyControl::new();

        occ.read(1, "key1".to_string()).unwrap();
        occ.validate(1);

        let stats = occ.stats();
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.validations, 1);
        assert_eq!(stats.validations_passed, 1);
    }
}
