// # Blockchain Table Ledger
//
// This module implements immutable ledger tables with cryptographic verification:
// - Insert-only semantics (no updates or deletes)
// - Row chaining with cryptographic hashes
// - Block creation and finalization
// - Merkle tree for block integrity
// - Cross-row hash dependencies
// - Tamper-evident design
// - Historical row versioning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::common::{Value, RowId, TableId};
use crate::Result;
use crate::error::DbError;
use super::crypto::{Hash256, sha256, MerkleTree, HashChain, ChainLink, hash_to_hex};

// ============================================================================
// Type Aliases
// ============================================================================

/// Block identifier
pub type BlockId = u64;

/// Row version number
pub type RowVersion = u64;

// ============================================================================
// Ledger Row
// ============================================================================

/// A row in a blockchain table (immutable once written)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerRow {
    /// Row ID
    pub row_id: RowId,
    /// Table ID
    pub table_id: TableId,
    /// Block this row belongs to
    pub block_id: BlockId,
    /// Version number (for historical tracking)
    pub version: RowVersion,
    /// Row data (column values)
    pub data: Vec<Value>,
    /// Hash of this row's data
    pub data_hash: Hash256,
    /// Hash of previous row (chain)
    pub previous_hash: Hash256,
    /// Combined row hash
    pub row_hash: Hash256,
    /// Creation timestamp
    pub timestamp: u64,
    /// Creator (user/session)
    pub creator: String,
    /// Digital signature (optional)
    pub signature: Option<Vec<u8>>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl LedgerRow {
    /// Create a new ledger row
    pub fn new(
        row_id: RowId,
        table_id: TableId,
        block_id: BlockId,
        version: RowVersion,
        data: Vec<Value>,
        previous_hash: Hash256,
        creator: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Compute data hash
        let data_bytes = bincode::serialize(&data).unwrap();
        let data_hash = sha256(&data_bytes);

        // Compute row hash (combines all fields)
        let row_hash = Self::compute_row_hash(
            row_id,
            table_id,
            block_id,
            version,
            &data_hash,
            &previous_hash,
            timestamp,
        );

        Self {
            row_id,
            table_id,
            block_id,
            version,
            data,
            data_hash,
            previous_hash,
            row_hash,
            timestamp,
            creator,
            signature: None,
            metadata: HashMap::new(),
        }
    }

    /// Compute hash for this row
    fn compute_row_hash(
        row_id: RowId,
        table_id: TableId,
        block_id: BlockId,
        version: RowVersion,
        data_hash: &Hash256,
        previous_hash: &Hash256,
        timestamp: u64,
    ) -> Hash256 {
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;

        hasher.update(&row_id.to_le_bytes());
        hasher.update(&table_id.to_le_bytes());
        hasher.update(&block_id.to_le_bytes());
        hasher.update(&version.to_le_bytes());
        hasher.update(data_hash);
        hasher.update(previous_hash);
        hasher.update(&timestamp.to_le_bytes());

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Verify this row's integrity
    pub fn verify(&self) -> bool {
        // Verify data hash
        let data_bytes = bincode::serialize(&self.data).unwrap();
        let computed_data_hash = sha256(&data_bytes);
        if computed_data_hash != self.data_hash {
            return false;
        }

        // Verify row hash
        let computed_row_hash = Self::compute_row_hash(
            self.row_id,
            self.table_id,
            self.block_id,
            self.version,
            &self.data_hash,
            &self.previous_hash,
            self.timestamp,
        );

        computed_row_hash == self.row_hash
    }

    /// Verify this row chains to a previous row
    pub fn verify_chain(&self, previous: &LedgerRow) -> bool {
        self.previous_hash == previous.row_hash
    }

    /// Add a digital signature to this row
    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = Some(signature);
    }

    /// Get row hash as hex string
    pub fn hash_hex(&self) -> String {
        hash_to_hex(&self.row_hash)
    }
}

// ============================================================================
// Block
// ============================================================================

/// Block status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Block is being built (rows can be added)
    Open,
    /// Block is finalized (immutable)
    Finalized,
    /// Block is verified
    Verified,
}

/// A block containing multiple ledger rows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block ID
    pub block_id: BlockId,
    /// Table ID
    pub table_id: TableId,
    /// Previous block hash
    pub previous_block_hash: Hash256,
    /// Rows in this block
    pub rows: Vec<LedgerRow>,
    /// Merkle root of all rows
    pub merkle_root: Hash256,
    /// Block hash
    pub block_hash: Hash256,
    /// Block status
    pub status: BlockStatus,
    /// Creation timestamp
    pub created_at: u64,
    /// Finalization timestamp
    pub finalized_at: Option<u64>,
    /// Block creator
    pub creator: String,
    /// Block metadata
    pub metadata: HashMap<String, String>,
}

impl Block {
    /// Create a new empty block
    pub fn new(block_id: BlockId, table_id: TableId, previous_block_hash: Hash256, creator: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            block_id,
            table_id,
            previous_block_hash,
            rows: Vec::new(),
            merkle_root: [0u8; 32],
            block_hash: [0u8; 32],
            status: BlockStatus::Open,
            created_at,
            finalized_at: None,
            creator,
            metadata: HashMap::new(),
        }
    }

    /// Add a row to this block (only if open)
    pub fn add_row(&mut self, row: LedgerRow) -> Result<()> {
        if self.status != BlockStatus::Open {
            return Err(DbError::InvalidOperation("Cannot add row to finalized block".to_string()));
        }

        if row.block_id != self.block_id {
            return Err(DbError::InvalidInput("Row block_id mismatch".to_string()));
        }

        self.rows.push(row);
        Ok(())
    }

    /// Finalize this block (compute hashes, prevent further modifications)
    pub fn finalize(&mut self) -> Result<()> {
        if self.status != BlockStatus::Open {
            return Err(DbError::InvalidOperation("Block already finalized".to_string()));
        }

        if self.rows.is_empty() {
            return Err(DbError::InvalidOperation("Cannot finalize empty block".to_string()));
        }

        // Compute Merkle root
        let row_data: Vec<&[u8]> = self.rows.iter()
            .map(|r| r.row_hash.as_ref())
            .collect();
        let merkle_tree = MerkleTree::build(&row_data)?;
        self.merkle_root = merkle_tree.root();

        // Compute block hash
        self.block_hash = self.compute_block_hash();

        // Update status
        self.status = BlockStatus::Finalized;
        self.finalized_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs());

        Ok(())
    }

    /// Compute hash for this block
    fn compute_block_hash(&self) -> Hash256 {
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;

        hasher.update(&self.block_id.to_le_bytes());
        hasher.update(&self.table_id.to_le_bytes());
        hasher.update(&self.previous_block_hash);
        hasher.update(&self.merkle_root);
        hasher.update(&self.created_at.to_le_bytes());

        let _result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Verify this block's integrity
    pub fn verify(&self) -> Result<bool> {
        if self.status == BlockStatus::Open {
            return Ok(false); // Cannot verify open block
        }

        // Verify all rows
        for row in &self.rows {
            if !row.verify() {
                return Ok(false);
            }
        }

        // Verify row chain
        for _i in 1..self.rows.len() {
            if !self.rows[i].verify_chain(&self.rows[i - 1]) {
                return Ok(false);
            }
        }

        // Verify Merkle root
        let row_data: Vec<&[u8]> = self.rows.iter()
            .map(|r| r.row_hash.as_ref())
            .collect();
        let merkle_tree = MerkleTree::build(&row_data)?;
        if merkle_tree.root() != self.merkle_root {
            return Ok(false);
        }

        // Verify block hash
        let computed_hash = self.compute_block_hash();
        if computed_hash != self.block_hash {
            return Ok(false);
        }

        Ok(true)
    }

    /// Get block hash as hex string
    pub fn hash_hex(&self) -> String {
        hash_to_hex(&self.block_hash)
    }

    /// Get number of rows in block
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

// ============================================================================
// Blockchain Table
// ============================================================================

/// Configuration for blockchain table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// Maximum rows per block (0 = unlimited)
    pub max_rows_per_block: usize,
    /// Auto-finalize blocks when full
    pub auto_finalize: bool,
    /// Require digital signatures
    pub require_signatures: bool,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            max_rows_per_block: 1000,
            auto_finalize: true,
            require_signatures: false,
            enable_compression: false,
        }
    }
}

/// Statistics for blockchain table
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockchainStats {
    /// Total number of blocks
    pub total_blocks: u64,
    /// Total number of rows
    pub total_rows: u64,
    /// Number of finalized blocks
    pub finalized_blocks: u64,
    /// Number of verified blocks
    pub verified_blocks: u64,
    /// Total data size (bytes)
    pub total_size_bytes: u64,
    /// Average block size
    pub avg_block_size: f64,
    /// Average rows per block
    pub avg_rows_per_block: f64,
}

/// Blockchain table with immutable rows
pub struct BlockchainTable {
    /// Table ID
    table_id: TableId,
    /// Table name
    name: String,
    /// Configuration
    config: BlockchainConfig,
    /// All blocks (ordered)
    pub blocks: Arc<RwLock<BTreeMap<BlockId, Block>>>,
    /// Current open block
    current_block: Arc<RwLock<Option<Block>>>,
    /// Row index (row_id -> block_id)
    row_index: Arc<RwLock<HashMap<RowId, BlockId>>>,
    /// Next block ID
    next_block_id: Arc<RwLock<BlockId>>,
    /// Next row ID
    next_row_id: Arc<RwLock<RowId>>,
    /// Hash chain for all rows
    pub hash_chain: Arc<RwLock<HashChain>>,
    /// Statistics
    stats: Arc<RwLock<BlockchainStats>>,
}

impl BlockchainTable {
    /// Create a new blockchain table
    pub fn new(table_id: TableId, name: String, config: BlockchainConfig) -> Self {
        Self {
            table_id,
            name,
            config,
            blocks: Arc::new(RwLock::new(BTreeMap::new())),
            current_block: Arc::new(RwLock::new(None)),
            row_index: Arc::new(RwLock::new(HashMap::new())),
            next_block_id: Arc::new(RwLock::new(0)),
            next_row_id: Arc::new(RwLock::new(0)),
            hash_chain: Arc::new(RwLock::new(HashChain::new(super::crypto::HashAlgorithm::Sha256))),
            stats: Arc::new(RwLock::new(BlockchainStats::default())),
        }
    }

    /// Insert a new row (append-only)
    pub fn insert(&self, data: Vec<Value>, creator: String) -> Result<RowId> {
        let mut current_block = self.current_block.write().unwrap();
        let mut next_row_id = self.next_row_id.write().unwrap();
        let mut row_index = self.row_index.write().unwrap();
        let mut hash_chain = self.hash_chain.write().unwrap();

        // Get or create current block
        if current_block.is_none() ||
           (self.config.max_rows_per_block > 0 &&
            current_block.as_ref().unwrap().rows.len() >= self.config.max_rows_per_block) {

            // Finalize current block if needed
            if let Some(ref mut block) = *current_block {
                if self.config.auto_finalize {
                    block.finalize()?;
                    let mut blocks = self.blocks.write().unwrap();
                    blocks.insert(block.block_id, block.clone());
                }
            }

            // Create new block
            drop(current_block);
            self.create_new_block(creator.clone())?;
            current_block = self.current_block.write().unwrap();
        }

        let block = current_block.as_mut().unwrap();
        let row_id = *next_row_id;
        *next_row_id += 1;

        // Get previous hash from chain
        let previous_hash = hash_chain.latest_link()
            .map(|link| link.link_hash)
            .unwrap_or_else(|| HashChain::genesis_hash());

        // Create row
        let row = LedgerRow::new(
            row_id,
            self.table_id,
            block.block_id,
            0, // version
            data.clone(),
            previous_hash,
            creator,
        );

        // Add to hash chain
        let row_bytes = bincode::serialize(&row).unwrap();
        hash_chain.append(&row_bytes, row.timestamp);

        // Add to block
        block.add_row(row)?;

        // Update index
        row_index.insert(row_id, block.block_id);

        // Update stats
        let mut stats = self.stats.write().unwrap();
        stats.total_rows += 1;

        Ok(row_id)
    }

    /// Create a new block
    fn create_new_block(&self, creator: String) -> Result<()> {
        let mut next_block_id = self.next_block_id.write().unwrap();
        let blocks = self.blocks.read().unwrap();

        let previous_block_hash = if *next_block_id == 0 {
            [0u8; 32] // Genesis hash
        } else {
            blocks.get(&(*next_block_id - 1))
                .map(|b| b.block_hash)
                .ok_or_else(|| DbError::Internal("Previous block not found".to_string()))?
        };

        let block = Block::new(*next_block_id, self.table_id, previous_block_hash, creator);

        let mut current_block = self.current_block.write().unwrap();
        *current_block = Some(block);

        *next_block_id += 1;

        let mut stats = self.stats.write().unwrap();
        stats.total_blocks += 1;

        Ok(())
    }

    /// Finalize the current block
    pub fn finalize_current_block(&self) -> Result<()> {
        let mut current_block = self.current_block.write().unwrap();

        if let Some(ref mut block) = *current_block {
            block.finalize()?;

            let mut blocks = self.blocks.write().unwrap();
            blocks.insert(block.block_id, block.clone());

            let mut stats = self.stats.write().unwrap();
            stats.finalized_blocks += 1;

            *current_block = None;
        }

        Ok(())
    }

    /// Get a row by ID
    pub fn get_row(&self, row_id: RowId) -> Result<Option<LedgerRow>> {
        let row_index = self.row_index.read().unwrap();
        let blocks = self.blocks.read().unwrap();
        let current_block = self.current_block.read().unwrap();

        if let Some(&block_id) = row_index.get(&row_id) {
            // Check finalized blocks
            if let Some(block) = blocks.get(&block_id) {
                for row in &block.rows {
                    if row.row_id == row_id {
                        return Ok(Some(row.clone()));
                    }
                }
            }

            // Check current block
            if let Some(ref block) = *current_block {
                if block.block_id == block_id {
                    for row in &block.rows {
                        if row.row_id == row_id {
                            return Ok(Some(row.clone()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get all rows in a block
    pub fn get_block(&self, block_id: BlockId) -> Result<Option<Block>> {
        let blocks = self.blocks.read().unwrap();
        Ok(blocks.get(&block_id).cloned())
    }

    /// Get current block (may be open)
    pub fn get_current_block(&self) -> Option<Block> {
        self.current_block.read().unwrap().clone()
    }

    /// Query rows with optional filters
    pub fn query_rows(&self, filter: Option<RowFilter>) -> Result<Vec<LedgerRow>> {
        let blocks = self.blocks.read().unwrap();
        let current_block = self.current_block.read().unwrap();
        let mut results = Vec::new();

        // Query finalized blocks
        for block in blocks.values() {
            for row in &block.rows {
                if filter.as_ref().map_or(true, |f| f.matches(row)) {
                    results.push(row.clone());
                }
            }
        }

        // Query current block
        if let Some(ref block) = *current_block {
            for row in &block.rows {
                if filter.as_ref().map_or(true, |f| f.matches(row)) {
                    results.push(row.clone());
                }
            }
        }

        Ok(results)
    }

    /// Verify all blocks in the table
    pub fn verify_all(&self) -> Result<bool> {
        let blocks = self.blocks.read().unwrap();

        // Verify each block
        for block in blocks.values() {
            if !block.verify()? {
                return Ok(false);
            }
        }

        // Verify block chain
        let mut previous_hash = [0u8; 32]; // Genesis
        for block_id in 0..blocks.len() as u64 {
            if let Some(block) = blocks.get(&block_id) {
                if block.previous_block_hash != previous_hash {
                    return Ok(false);
                }
                previous_hash = block.block_hash;
            }
        }

        // Verify hash chain
        let hash_chain = self.hash_chain.read().unwrap();
        hash_chain.verify()
    }

    /// Get table statistics
    pub fn get_stats(&self) -> BlockchainStats {
        let mut stats = self.stats.read().unwrap().clone();

        let blocks = self.blocks.read().unwrap();
        if !blocks.is_empty() {
            stats.avg_rows_per_block = stats.total_rows as f64 / stats.total_blocks as f64;
        }

        stats
    }

    /// Get table name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get table ID
    pub fn table_id(&self) -> TableId {
        self.table_id
    }

    /// Get configuration
    pub fn config(&self) -> &BlockchainConfig {
        &self.config
    }

    /// Get block count
    pub fn block_count(&self) -> usize {
        self.blocks.read().unwrap().len()
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.row_index.read().unwrap().len()
    }
}

// ============================================================================
// Row Filter
// ============================================================================

/// Filter for querying rows
#[derive(Debug, Clone)]
pub struct RowFilter {
    /// Filter by block ID
    pub block_id: Option<BlockId>,
    /// Filter by row ID range
    pub row_id_range: Option<(RowId, RowId)>,
    /// Filter by timestamp range
    pub timestamp_range: Option<(u64, u64)>,
    /// Filter by creator
    pub creator: Option<String>,
}

impl RowFilter {
    /// Check if a row matches this filter
    pub fn matches(&self, row: &LedgerRow) -> bool {
        if let Some(block_id) = self.block_id {
            if row.block_id != block_id {
                return false;
            }
        }

        if let Some((min, max)) = self.row_id_range {
            if row.row_id < min || row.row_id > max {
                return false;
            }
        }

        if let Some((min, max)) = self.timestamp_range {
            if row.timestamp < min || row.timestamp > max {
                return false;
            }
        }

        if let Some(ref creator) = self.creator {
            if &row.creator != creator {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Historical Versioning
// ============================================================================

/// Historical version of a row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowHistory {
    /// Original row ID
    pub row_id: RowId,
    /// All versions of this row
    pub versions: Vec<LedgerRow>,
}

impl RowHistory {
    /// Create new history
    pub fn new(row_id: RowId) -> Self {
        Self {
            row_id,
            versions: Vec::new(),
        }
    }

    /// Add a version
    pub fn add_version(&mut self, row: LedgerRow) {
        self.versions.push(row);
    }

    /// Get latest version
    pub fn latest(&self) -> Option<&LedgerRow> {
        self.versions.last()
    }

    /// Get version by index
    pub fn get_version(&self, version: RowVersion) -> Option<&LedgerRow> {
        self.versions.iter().find(|r| r.version == version)
    }

    /// Get version count
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

// ============================================================================
// Export/Import
// ============================================================================

/// Export a block to bytes
pub fn export_block(block: &Block) -> Result<Vec<u8>> {
    bincode::serialize(block)
        .map_err(|e| DbError::Serialization(format!("Block serialization failed: {}", e)))
}

/// Import a block from bytes
pub fn import_block(data: &[u8]) -> Result<Block> {
    bincode::deserialize(data)
        .map_err(|e| DbError::Serialization(format!("Block deserialization failed: {}", e)))
}

/// Export entire blockchain to bytes
pub fn export_blockchain(table: &BlockchainTable) -> Result<Vec<u8>> {
    let blocks = table.blocks.read().unwrap();
    let all_blocks: Vec<Block> = blocks.values().cloned().collect();

    bincode::serialize(&all_blocks)
        .map_err(|e| DbError::Serialization(format!("Blockchain serialization failed: {}", e)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_row() {
        let data = vec![Value::Integer(1), Value::String("test".to_string())];
        let row = LedgerRow::new(0, 1, 0, 0, data, [0u8; 32], "user1".to_string());

        assert_eq!(row.row_id, 0);
        assert!(row.verify());
    }

    #[test]
    fn test_block() {
        let mut block = Block::new(0, 1, [0u8; 32], "user1".to_string());

        let data1 = vec![Value::Integer(1)];
        let row1 = LedgerRow::new(0, 1, 0, 0, data1, [0u8; 32], "user1".to_string());
        block.add_row(row1).unwrap();

        let data2 = vec![Value::Integer(2)];
        let row2 = LedgerRow::new(1, 1, 0, 0, data2, block.rows[0].row_hash, "user1".to_string());
        block.add_row(row2).unwrap();

        block.finalize().unwrap();
        assert!(block.verify().unwrap());
    }

    #[test]
    fn test_blockchain_table() {
        let config = BlockchainConfig::default();
        let table = BlockchainTable::new(1, "test_ledger".to_string(), config);

        let data1 = vec![Value::Integer(100)];
        let row_id1 = table.insert(data1, "user1".to_string()).unwrap();

        let data2 = vec![Value::Integer(200)];
        let row_id2 = table.insert(data2, "user1".to_string()).unwrap();

        assert_eq!(table.row_count(), 2);

        let row = table.get_row(row_id1).unwrap().unwrap();
        assert_eq!(row.row_id, row_id1);

        table.finalize_current_block().unwrap();
        assert!(table.verify_all().unwrap());
    }

    #[test]
    fn test_row_filter() {
        let data = vec![Value::Integer(1)];
        let row = LedgerRow::new(5, 1, 2, 0, data, [0u8; 32], "alice".to_string());

        let filter = RowFilter {
            block_id: Some(2),
            row_id_range: None,
            timestamp_range: None,
            creator: None,
        };

        assert!(filter.matches(&row));

        let filter2 = RowFilter {
            block_id: Some(3),
            row_id_range: None,
            timestamp_range: None,
            creator: None,
        };

        assert!(!filter2.matches(&row));
    }
}
