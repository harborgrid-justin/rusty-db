// # Blockchain Tables Module
//
// This module implements immutable blockchain tables with cryptographic verification,
// audit trails, retention policies, and compliance features.
//
// ## Overview
//
// The blockchain tables feature provides:
// - **Immutable Ledger**: Insert-only tables with cryptographic hash chaining
// - **Cryptographic Security**: SHA-256 hashing, Merkle trees, digital signatures
// - **Integrity Verification**: Automated tamper detection and chain verification
// - **Retention Management**: Time-based retention policies and legal holds
// - **Audit Trail**: Complete audit logging for compliance
//
// ## Modules
//
// - `crypto`: Cryptographic primitives (hashing, signatures, Merkle trees)
// - `ledger`: Blockchain table implementation with immutable rows and blocks
// - `verification`: Chain integrity verification and tamper detection
// - `retention`: Retention policy enforcement and legal hold management
// - `audit_trail`: Comprehensive audit logging and reporting
//
// ## Usage Example
//
// ```rust,no_run
// use rusty_db::blockchain::{
//     BlockchainTable, BlockchainConfig,
//     AuditLogger, AuditConfig,
//     RetentionManager, RetentionPolicy, RetentionPeriod,
// };
// use rusty_db::common::Value;
//
// # fn example() -> rusty_db::Result<()> {
// // Create a blockchain table
// let config = BlockchainConfig::default();
// let table = BlockchainTable::new(1, "transactions".to_string(), config);
//
// // Insert immutable rows
// let data = vec![Value::Integer(100), Value::String("Transaction 1".to_string())];
// let row_id = table.insert(data, "user1".to_string())?;
//
// // Finalize the current block
// table.finalize_current_block()?;
//
// // Verify integrity
// let is_valid = table.verify_all()?;
// assert!(is_valid);
//
// // Set up retention policy
// let retention_mgr = RetentionManager::new();
// let _policy = RetentionPolicy::new(
//     "policy1".to_string(),
//     "7 Year Retention".to_string(),
//     RetentionPeriod::Years(7),
//     "Financial records retention".to_string(),
// );
// retention_mgr.add_policy(policy)?;
// retention_mgr.assign_policy_to_table(1, "policy1".to_string())?;
//
// // Set up audit logging
// let audit_config = AuditConfig::default();
// let audit_logger = AuditLogger::new(audit_config);
//
// # Ok(())
// # }
// ```
//
// ## Features
//
// ### Immutable Ledger
//
// - Append-only semantics (no updates or deletes)
// - Cryptographic hash chaining between rows
// - Block-based organization
// - Merkle tree verification
//
// ### Cryptographic Security
//
// - SHA-256/SHA-512 hashing
// - Digital signatures (Ed25519-like)
// - Merkle proofs for inclusion verification
// - Zero-knowledge proof concepts
// - Cryptographic accumulators
//
// ### Verification & Tamper Detection
//
// - Automated integrity verification
// - Parallel verification for performance
// - Tamper detection and reporting
// - Recovery procedures
//
// ### Retention & Compliance
//
// - Time-based retention policies
// - Legal hold implementation
// - Compliance reporting
// - Retention inheritance
//
// ### Audit Trail
//
// - Complete audit logging
// - User action tracking
// - Query logging
// - Change attribution
// - Export to compliance formats

pub mod crypto;
pub mod ledger;
pub mod verification;
pub mod retention;
pub mod audit_trail;

// Re-export commonly used types
pub use crypto::{
    Hash256, HashAlgorithm,
    sha256, sha512, hmac_sha256,
    HashChain, ChainLink,
    MerkleTree, MerkleProof, MerkleNode,
    KeyPair, Signature, PublicKey, PrivateKey,
    Accumulator, Commitment, RangeProof,
    hash_to_hex, hex_to_hash,
};

pub use ledger::{
    BlockchainTable, BlockchainConfig, BlockchainStats,
    Block, BlockId, BlockStatus,
    LedgerRow, RowVersion,
    RowFilter, RowHistory,
    export_block, import_block, export_blockchain,
};

pub use verification::{
    VerificationResult, VerificationDetails, VerificationType,
    VerificationIssue, IssueSeverity, IssueType,
    BlockVerifier, ChainVerifier, ParallelVerifier,
    VerificationScheduler,
    TamperDetector, TamperReport, TamperStatus,
    RecoveryManager, RecoveryAction, RecoveryActionType, RecoveryPriority,
};

pub use retention::{
    RetentionPeriod, RetentionPolicy,
    LegalHold, LegalHoldStatus,
    RetentionManager, RetentionEnforcer,
    ExpirationReport,
    ComplianceReport, ComplianceReporter,
    PolicyCompliance, LegalHoldCompliance, RetentionStatistics,
};

pub use audit_trail::{
    AuditEvent, AuditEventType, AuditSeverity,
    AuditLogger, AuditConfig,
    AuditFilter, AuditReport,
    QueryAuditEntry, QueryAuditLogger,
};
