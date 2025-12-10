// # Blockchain Verification
//
// This module provides comprehensive verification for blockchain tables:
// - Chain integrity verification
// - Block validation algorithms
// - Proof-of-inclusion verification
// - Tamper detection mechanisms
// - Verification scheduling
// - Parallel verification
// - Verification reports and alerts
// - Recovery procedures for detected issues

use std::collections::HashSet;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, UNIX_EPOCH};
use tokio::task;
use crate::Result;
use super::ledger::{Block, BlockId, BlockchainTable};
use crate::common::RowId;
use super::crypto::Hash256;

// ============================================================================
// Verification Result Types
// ============================================================================

// Result of a verification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    // Whether verification passed
    pub passed: bool,
    // Verification timestamp
    pub timestamp: u64,
    // Details about what was verified
    pub details: VerificationDetails,
    // Issues found (if any)
    pub issues: Vec<VerificationIssue>,
    // Duration of verification
    pub duration_ms: u64,
}

impl VerificationResult {
    // Create a new successful result
    pub fn success(details: VerificationDetails, duration_ms: u64) -> Self {
        Self {
            passed: true,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            details,
            issues: Vec::new(),
            duration_ms,
        }
    }

    // Create a new failed result
    pub fn failure(details: VerificationDetails, issues: Vec<VerificationIssue>, duration_ms: u64) -> Self {
        Self {
            passed: false,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            details,
            issues,
            duration_ms,
        }
    }
}

// Details about what was verified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDetails {
    // Number of blocks verified
    pub blocks_verified: usize,
    // Number of rows verified
    pub rows_verified: usize,
    // Verification type
    pub verification_type: VerificationType,
    // Additional metadata
    pub metadata: HashMap<String, String>,
}

// Type of verification performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationType {
    // Full blockchain verification
    Full,
    // Single block verification
    Block,
    // Row-level verification
    Row,
    // Chain continuity verification
    Chain,
    // Merkle tree verification
    Merkle,
    // Incremental verification
    Incremental,
}

// Issue found during verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    // Severity level
    pub severity: IssueSeverity,
    // Issue type
    pub issue_type: IssueType,
    // Block ID (if applicable)
    pub block_id: Option<BlockId>,
    // Row ID (if applicable)
    pub row_id: Option<RowId>,
    // Human-readable description
    pub description: String,
    // When the issue was detected
    pub detected_at: u64,
}

impl VerificationIssue {
    // Create a new issue
    pub fn new(severity: IssueSeverity, issue_type: IssueType, description: String) -> Self {
        Self {
            severity,
            issue_type,
            block_id: None,
            row_id: None,
            description,
            detected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    // Set block ID
    pub fn with_block(mut self, block_id: BlockId) -> Self {
        self.block_id = Some(block_id);
        self
    }

    // Set row ID
    pub fn with_row(mut self, row_id: RowId) -> Self {
        self.row_id = Some(row_id);
        self
    }
}

// Severity of an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    // Informational only
    Info,
    // Warning - potential issue
    Warning,
    // Error - integrity violation
    Error,
    // Critical - data corruption
    Critical,
}

// Type of issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    // Hash mismatch
    HashMismatch,
    // Chain break
    ChainBreak,
    // Merkle tree invalid
    MerkleInvalid,
    // Signature invalid
    SignatureInvalid,
    // Timestamp anomaly
    TimestampAnomaly,
    // Missing data
    MissingData,
    // Duplicate entry
    DuplicateEntry,
    // Other issue
    Other,
}

// ============================================================================
// Block Verifier
// ============================================================================

// Verifies individual blocks
pub struct BlockVerifier;

impl BlockVerifier {
    // Verify a single block
    pub fn verify_block(block: &Block) -> Result<VerificationResult> {
        let start = SystemTime::now();
        let mut issues = Vec::new();

        // Check block is finalized
        if block.status == super::ledger::BlockStatus::Open {
            issues.push(VerificationIssue::new(
                IssueSeverity::Error,
                IssueType::Other,
                "Block is not finalized".to_string(),
            ).with_block(block.block_id));
        }

        // Verify each row
        for row in &block.rows {
            if !row.verify() {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Critical,
                    IssueType::HashMismatch,
                    format!("Row {} hash verification failed", row.row_id),
                ).with_block(block.block_id).with_row(row.row_id));
            }
        }

        // Verify row chain
        for i in 1..block.rows.len() {
            if !block.rows[i].verify_chain(&block.rows[i - 1]) {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Critical,
                    IssueType::ChainBreak,
                    format!("Chain break between rows {} and {}", block.rows[i - 1].row_id, block.rows[i].row_id),
                ).with_block(block.block_id));
            }
        }

        // Verify merkle tree
        if !block.rows.is_empty() {
            let row_data: Vec<&[u8]> = block.rows.iter()
                .map(|r| r.row_hash.as_ref())
                .collect();

            match super::crypto::MerkleTree::build(&row_data) {
                Ok(tree) => {
                    if tree.root() != block.merkle_root {
                        issues.push(VerificationIssue::new(
                            IssueSeverity::Critical,
                            IssueType::MerkleInvalid,
                            "Merkle root mismatch".to_string(),
                        ).with_block(block.block_id));
                    }
                }
                Err(_) => {
                    issues.push(VerificationIssue::new(
                        IssueSeverity::Error,
                        IssueType::MerkleInvalid,
                        "Failed to build Merkle tree".to_string(),
                    ).with_block(block.block_id));
                }
            }
        }

        // Verify block hash
        let block_valid = match block.verify() {
            Ok(valid) => valid,
            Err(_) => {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Critical,
                    IssueType::HashMismatch,
                    "Block hash verification failed".to_string(),
                ).with_block(block.block_id));
                false
            }
        };

        if !block_valid {
            issues.push(VerificationIssue::new(
                IssueSeverity::Critical,
                IssueType::HashMismatch,
                "Block integrity check failed".to_string(),
            ).with_block(block.block_id));
        }

        let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;

        let details = VerificationDetails {
            blocks_verified: 1,
            rows_verified: block.rows.len(),
            verification_type: VerificationType::Block,
            metadata: HashMap::new(),
        };

        if issues.is_empty() {
            Ok(VerificationResult::success(details, duration))
        } else {
            Ok(VerificationResult::failure(details, issues, duration))
        }
    }

    // Verify timestamp ordering
    pub fn verify_timestamps(block: &Block) -> Vec<VerificationIssue> {
        let mut issues = Vec::new();

        // Check rows have increasing timestamps
        for i in 1..block.rows.len() {
            if block.rows[i].timestamp < block.rows[i - 1].timestamp {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Warning,
                    IssueType::TimestampAnomaly,
                    format!("Row {} has earlier timestamp than row {}",
                            block.rows[i].row_id, block.rows[i - 1].row_id),
                ).with_block(block.block_id));
            }
        }

        issues
    }
}

// ============================================================================
// Chain Verifier
// ============================================================================

// Verifies blockchain continuity
pub struct ChainVerifier;

impl ChainVerifier {
    // Verify entire blockchain chain
    pub fn verify_chain(table: &BlockchainTable) -> Result<VerificationResult> {
        let start = SystemTime::now();
        let mut issues = Vec::new();

        // Get all blocks in order
        let blocks = table.blocks.read().unwrap();
        let block_count = blocks.len();
        let mut row_count = 0;

        if block_count == 0 {
            let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;
            let details = VerificationDetails {
                blocks_verified: 0,
                rows_verified: 0,
                verification_type: VerificationType::Chain,
                metadata: HashMap::new(),
            };
            return Ok(VerificationResult::success(details, duration));
        }

        // Verify first block
        if let Some(first_block) = blocks.get(&0) {
            let genesis_hash = [0u8; 32];
            if first_block.previous_block_hash != genesis_hash {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Critical,
                    IssueType::ChainBreak,
                    "First block does not reference genesis hash".to_string(),
                ).with_block(0));
            }
            row_count += first_block.rows.len();
        } else {
            issues.push(VerificationIssue::new(
                IssueSeverity::Critical,
                IssueType::MissingData,
                "Block 0 is missing".to_string(),
            ));
        }

        // Verify block chain continuity
        for block_id in 1..block_count as BlockId {
            let prev_block = blocks.get(&(block_id - 1));
            let curr_block = blocks.get(&block_id);

            match (prev_block, curr_block) {
                (Some(prev), Some(curr)) => {
                    if curr.previous_block_hash != prev.block_hash {
                        issues.push(VerificationIssue::new(
                            IssueSeverity::Critical,
                            IssueType::ChainBreak,
                            format!("Block {} does not chain to block {}", block_id, block_id - 1),
                        ).with_block(block_id));
                    }
                    row_count += curr.rows.len();
                }
                (None, _) => {
                    issues.push(VerificationIssue::new(
                        IssueSeverity::Critical,
                        IssueType::MissingData,
                        format!("Block {} is missing", block_id - 1),
                    ).with_block(block_id - 1));
                }
                (_, None) => {
                    issues.push(VerificationIssue::new(
                        IssueSeverity::Critical,
                        IssueType::MissingData,
                        format!("Block {} is missing", block_id),
                    ).with_block(block_id));
                }
            }
        }

        let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;

        let details = VerificationDetails {
            blocks_verified: block_count,
            rows_verified: row_count,
            verification_type: VerificationType::Chain,
            metadata: HashMap::new(),
        };

        if issues.is_empty() {
            Ok(VerificationResult::success(details, duration))
        } else {
            Ok(VerificationResult::failure(details, issues, duration))
        }
    }

    // Verify no duplicate rows exist
    pub fn verify_no_duplicates(table: &BlockchainTable) -> Vec<VerificationIssue> {
        let mut issues = Vec::new();
        let mut seen_row_ids = HashSet::new();

        let blocks = table.blocks.read().unwrap();

        for block in blocks.values() {
            for row in &block.rows {
                if !seen_row_ids.insert(row.row_id) {
                    issues.push(VerificationIssue::new(
                        IssueSeverity::Critical,
                        IssueType::DuplicateEntry,
                        format!("Duplicate row ID {}", row.row_id),
                    ).with_block(block.block_id).with_row(row.row_id));
                }
            }
        }

        issues
    }
}

// ============================================================================
// Parallel Verifier
// ============================================================================

// Performs parallel verification for performance
pub struct ParallelVerifier;

impl ParallelVerifier {
    // Verify all blocks in parallel
    pub async fn verify_all_blocks(table: &BlockchainTable) -> Result<VerificationResult> {
        let start = SystemTime::now();
        let blocks = table.blocks.read().unwrap();
        let block_count = blocks.len();

        if block_count == 0 {
            let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;
            let details = VerificationDetails {
                blocks_verified: 0,
                rows_verified: 0,
                verification_type: VerificationType::Full,
                metadata: HashMap::new(),
            };
            return Ok(VerificationResult::success(details, duration));
        }

        // Clone blocks for parallel processing
        let block_vec: Vec<Block> = blocks.values().cloned().collect();
        drop(blocks);

        // Verify blocks in parallel
        let mut tasks = Vec::new();
        for block in block_vec {
            let task = task::spawn(async move {
                BlockVerifier::verify_block(&block)
            });
            tasks.push(task);
        }

        // Collect results
        let mut all_issues = Vec::new();
        let mut total_rows = 0;

        for task in tasks {
            match task.await {
                Ok(Ok(result)) => {
                    total_rows += result.details.rows_verified;
                    all_issues.extend(result.issues);
                }
                Ok(Err(e)) => {
                    all_issues.push(VerificationIssue::new(
                        IssueSeverity::Error,
                        IssueType::Other,
                        format!("Verification error: {}", e),
                    ));
                }
                Err(e) => {
                    all_issues.push(VerificationIssue::new(
                        IssueSeverity::Error,
                        IssueType::Other,
                        format!("Task error: {}", e),
                    ));
                }
            }
        }

        // Verify chain continuity (sequential)
        let chain_result = ChainVerifier::verify_chain(table)?;
        all_issues.extend(chain_result.issues);

        // Verify no duplicates
        let dup_issues = ChainVerifier::verify_no_duplicates(table);
        all_issues.extend(dup_issues);

        let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;

        let details = VerificationDetails {
            blocks_verified: block_count,
            rows_verified: total_rows,
            verification_type: VerificationType::Full,
            metadata: HashMap::new(),
        };

        if all_issues.is_empty() {
            Ok(VerificationResult::success(details, duration))
        } else {
            Ok(VerificationResult::failure(details, all_issues, duration))
        }
    }
}

// ============================================================================
// Verification Scheduler
// ============================================================================

// Schedules periodic verification
pub struct VerificationScheduler {
    // Verification interval
    interval: Duration,
    // Last verification time
    last_verification: Arc<RwLock<Option<SystemTime>>>,
    // Verification history
    history: Arc<RwLock<Vec<VerificationResult>>>,
}

impl VerificationScheduler {
    // Create a new scheduler
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_verification: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Check if verification is due
    pub fn is_due(&self) -> bool {
        let last = self.last_verification.read().unwrap();
        match *last {
            None => true,
            Some(time) => {
                SystemTime::now().duration_since(time).unwrap() >= self.interval
            }
        }
    }

    // Run verification
    pub async fn run_verification(&self, table: &BlockchainTable) -> Result<VerificationResult> {
        let result = ParallelVerifier::verify_all_blocks(table).await?;

        // Update last verification time
        let mut last = self.last_verification.write().unwrap();
        *last = Some(SystemTime::now());

        // Add to history
        let mut history = self.history.write().unwrap();
        history.push(result.clone());

        // Keep only last 100 results
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(result)
    }

    // Get verification history
    pub fn get_history(&self) -> Vec<VerificationResult> {
        self.history.read().unwrap().clone()
    }

    // Get last verification result
    pub fn last_result(&self) -> Option<VerificationResult> {
        self.history.read().unwrap().last().cloned()
    }
}

// ============================================================================
// Tamper Detection
// ============================================================================

// Detects tampering attempts
pub struct TamperDetector;

impl TamperDetector {
    // Detect any tampering in the blockchain
    pub fn detect_tampering(table: &BlockchainTable) -> Vec<VerificationIssue> {
        let mut issues = Vec::new();

        // Verify hash chain
        let hash_chain = table.hash_chain.read().unwrap();
        match hash_chain.verify() {
            Ok(true) => {}
            Ok(false) => {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Critical,
                    IssueType::ChainBreak,
                    "Hash chain verification failed - possible tampering detected".to_string(),
                ));
            }
            Err(e) => {
                issues.push(VerificationIssue::new(
                    IssueSeverity::Error,
                    IssueType::Other,
                    format!("Hash chain verification error: {}", e),
                ));
            }
        }

        // Verify all blocks
        let blocks = table.blocks.read().unwrap();
        for block in blocks.values() {
            match BlockVerifier::verify_block(block) {
                Ok(result) => {
                    if !result.passed {
                        issues.extend(result.issues);
                    }
                }
                Err(e) => {
                    issues.push(VerificationIssue::new(
                        IssueSeverity::Error,
                        IssueType::Other,
                        format!("Block verification error: {}", e),
                    ).with_block(block.block_id));
                }
            }
        }

        issues
    }

    // Generate tamper detection report
    pub fn generate_report(table: &BlockchainTable) -> TamperReport {
        let issues = Self::detect_tampering(table);
        let stats = table.get_stats();
        let issues_found = issues.len();
        let status = if issues.is_empty() {
            TamperStatus::Clean
        } else {
            TamperStatus::Compromised
        };

        TamperReport {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            total_blocks: stats.total_blocks,
            total_rows: stats.total_rows,
            issues_found,
            issues,
            status,
        }
    }
}

// Tamper detection report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TamperReport {
    // Report timestamp
    pub timestamp: u64,
    // Total blocks checked
    pub total_blocks: u64,
    // Total rows checked
    pub total_rows: u64,
    // Number of issues found
    pub issues_found: usize,
    // Issues detected
    pub issues: Vec<VerificationIssue>,
    // Overall status
    pub status: TamperStatus,
}

// Tamper status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TamperStatus {
    // No tampering detected
    Clean,
    // Tampering detected
    Compromised,
    // Unable to verify
    Unknown,
}

// ============================================================================
// Recovery Procedures
// ============================================================================

// Recovery actions for verification issues
pub struct RecoveryManager;

impl RecoveryManager {
    // Analyze issues and suggest recovery actions
    pub fn analyze_issues(issues: &[VerificationIssue]) -> Vec<RecoveryAction> {
        let mut actions = Vec::new();

        for issue in issues {
            match issue.issue_type {
                IssueType::HashMismatch => {
                    actions.push(RecoveryAction {
                        action_type: RecoveryActionType::RestoreFromBackup,
                        description: format!("Restore corrupted block {} from backup",
                                           issue.block_id.unwrap_or(0)),
                        priority: RecoveryPriority::High,
                    });
                }
                IssueType::ChainBreak => {
                    actions.push(RecoveryAction {
                        action_type: RecoveryActionType::RebuildChain,
                        description: "Rebuild chain from known good state".to_string(),
                        priority: RecoveryPriority::Critical,
                    });
                }
                IssueType::MissingData => {
                    actions.push(RecoveryAction {
                        action_type: RecoveryActionType::RestoreFromBackup,
                        description: format!("Restore missing block {} from backup",
                                           issue.block_id.unwrap_or(0)),
                        priority: RecoveryPriority::High,
                    });
                }
                _ => {
                    actions.push(RecoveryAction {
                        action_type: RecoveryActionType::Investigate,
                        description: format!("Investigate issue: {}", issue.description),
                        priority: RecoveryPriority::Medium,
                    });
                }
            }
        }

        actions
    }
}

// Recovery action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    // Type of action
    pub action_type: RecoveryActionType,
    // Description
    pub description: String,
    // Priority
    pub priority: RecoveryPriority,
}

// Type of recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryActionType {
    // Restore from backup
    RestoreFromBackup,
    // Rebuild chain
    RebuildChain,
    // Manual investigation needed
    Investigate,
    // Quarantine affected data
    Quarantine,
}

// Recovery priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecoveryPriority {
    // Low priority
    Low,
    // Medium priority
    Medium,
    // High priority
    High,
    // Critical - immediate action required
    Critical,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Value;

    #[test]
    fn test_block_verification() {
        let mut block = Block::new(0, 1, [0u8; 32], "user1".to_string());

        let data = vec![Value::Integer(1)];
        let row = LedgerRow::new(
            0, 1, 0, 0, data, [0u8; 32], "user1".to_string()
        );
        block.add_row(row).unwrap();

        block.finalize().unwrap();

        let result = BlockVerifier::verify_block(&block).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_verification_issue() {
        let issue = VerificationIssue::new(
            IssueSeverity::Critical,
            IssueType::HashMismatch,
            "Test issue".to_string(),
        ).with_block(5).with_row(10);

        assert_eq!(issue.severity, IssueSeverity::Critical);
        assert_eq!(issue.block_id, Some(5));
        assert_eq!(issue.row_id, Some(10));
    }
}
