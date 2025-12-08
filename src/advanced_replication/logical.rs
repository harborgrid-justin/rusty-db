//! # Logical Replication
//!
//! Row-level logical replication with table/column filtering,
//! transformation, and schema evolution handling.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::DbError;

/// Logical replication publication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    /// Publication name
    pub name: String,
    /// Tables included in publication
    pub tables: Vec<TablePublication>,
    /// Whether to replicate DDL changes
    pub replicate_ddl: bool,
    /// Whether to replicate truncate
    pub replicate_truncate: bool,
    /// Publication owner
    pub owner: String,
    /// Creation timestamp
    pub created_at: u64,
}

/// Table configuration for publication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePublication {
    /// Table name
    pub table_name: String,
    /// Schema name
    pub schema_name: String,
    /// Columns to replicate (None = all columns)
    pub columns: Option<Vec<String>>,
    /// Row filter predicate
    pub row_filter: Option<String>,
    /// Transformation rules
    pub transformations: Vec<Transformation>,
    /// Whether to replicate inserts
    pub replicate_insert: bool,
    /// Whether to replicate updates
    pub replicate_update: bool,
    /// Whether to replicate deletes
    pub replicate_delete: bool,
}

/// Column transformation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transformation {
    /// Rename a column
    Rename { from: String, to: String },
    /// Apply a function to transform the value
    Function { column: String, function: String },
    /// Cast to different type
    Cast { column: String, target_type: String },
    /// Mask sensitive data
    Mask { column: String, mask_type: MaskType },
    /// Constant value
    Constant { column: String, value: Vec<u8> },
}

/// Type of masking to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaskType {
    /// Replace with NULL
    Null,
    /// Replace with hash
    Hash,
    /// Partial mask (show only last N characters)
    Partial(usize),
    /// Replace with constant
    Constant(Vec<u8>),
}

/// Logical replication subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Subscription name
    pub name: String,
    /// Publication name on source
    pub publication_name: String,
    /// Source connection string
    pub source_connection: String,
    /// Target schema mapping
    pub schema_mapping: HashMap<String, String>,
    /// Subscription state
    pub state: SubscriptionState,
    /// Last processed LSN
    pub last_lsn: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether subscription is enabled
    pub enabled: bool,
}

/// Subscription state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionState {
    /// Initial state
    Initializing,
    /// Copying initial data
    CopyingData,
    /// Catching up with changes
    Syncing,
    /// Actively streaming changes
    Streaming,
    /// Paused
    Paused,
    /// Failed
    Failed(String),
}

/// Logical change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalChange {
    /// Change ID
    pub change_id: String,
    /// Log sequence number
    pub lsn: u64,
    /// Transaction ID
    pub transaction_id: u64,
    /// Schema name
    pub schema: String,
    /// Table name
    pub table: String,
    /// Change type
    pub change_type: ChangeType,
    /// Column names
    pub columns: Vec<String>,
    /// Old row data (for updates and deletes)
    pub old_row: Option<Vec<Option<Vec<u8>>>>,
    /// New row data (for inserts and updates)
    pub new_row: Option<Vec<Option<Vec<u8>>>>,
    /// Timestamp
    pub timestamp: u64,
}

/// Type of logical change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
    Truncate,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
}

/// Logical replication manager
pub struct LogicalReplication {
    /// Publications
    publications: Arc<RwLock<HashMap<String, Publication>>>,
    /// Subscriptions
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    /// Change buffer
    change_buffer: Arc<RwLock<VecDeque<LogicalChange>>>,
    /// Current LSN
    current_lsn: Arc<RwLock<u64>>,
    /// Statistics
    stats: Arc<RwLock<LogicalReplicationStats>>,
    /// Change channel
    change_tx: mpsc::UnboundedSender<LogicalChange>,
    change_rx: Arc<RwLock<mpsc::UnboundedReceiver<LogicalChange>>>,
}

/// Logical replication statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogicalReplicationStats {
    pub total_changes: u64,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub truncates: u64,
    pub transactions: u64,
    pub bytes_replicated: u64,
    pub transformations_applied: u64,
    pub errors: u64,
    pub changes_by_table: HashMap<String, u64>,
}

impl LogicalReplication {
    /// Create a new logical replication manager
    pub fn new() -> Self {
        let (change_tx, change_rx) = mpsc::unbounded_channel();

        Self {
            publications: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            change_buffer: Arc::new(RwLock::new(VecDeque::new())),
            current_lsn: Arc::new(RwLock::new(0)),
            stats: Arc::new(RwLock::new(LogicalReplicationStats::default())),
            change_tx,
            change_rx: Arc::new(RwLock::new(change_rx)),
        }
    }

    /// Create a publication
    pub fn create_publication(&self, publication: Publication) -> Result<()> {
        let mut pubs = self.publications.write();

        if pubs.contains_key(&publication.name) {
            return Err(DbError::Replication(
                format!("Publication {} already exists", publication.name)
            ));
        }

        pubs.insert(publication.name.clone(), publication);
        Ok(())
    }

    /// Drop a publication
    pub fn drop_publication(&self, name: &str) -> Result<()> {
        let mut pubs = self.publications.write();

        pubs.remove(name)
            .ok_or_else(|| DbError::Replication(
                format!("Publication {} not found", name)
            ))?;

        Ok(())
    }

    /// Add table to publication
    pub fn add_table_to_publication(
        &self,
        pub_name: &str,
        table: TablePublication,
    ) -> Result<()> {
        let mut pubs = self.publications.write();

        let pub_entry = pubs.get_mut(pub_name)
            .ok_or_else(|| DbError::Replication(
                format!("Publication {} not found", pub_name)
            ))?;

        pub_entry.tables.push(table);
        Ok(())
    }

    /// Create a subscription
    pub fn create_subscription(&self, subscription: Subscription) -> Result<()> {
        let mut subs = self.subscriptions.write();

        if subs.contains_key(&subscription.name) {
            return Err(DbError::Replication(
                format!("Subscription {} already exists", subscription.name)
            ));
        }

        subs.insert(subscription.name.clone(), subscription);
        Ok(())
    }

    /// Drop a subscription
    pub fn drop_subscription(&self, name: &str) -> Result<()> {
        let mut subs = self.subscriptions.write();

        subs.remove(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        Ok(())
    }

    /// Enable a subscription
    pub fn enable_subscription(&self, name: &str) -> Result<()> {
        let mut subs = self.subscriptions.write();

        let sub = subs.get_mut(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        sub.enabled = true;
        Ok(())
    }

    /// Disable a subscription
    pub fn disable_subscription(&self, name: &str) -> Result<()> {
        let mut subs = self.subscriptions.write();

        let sub = subs.get_mut(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        sub.enabled = false;
        Ok(())
    }

    /// Capture a change from the write-ahead log
    pub async fn capture_change(&self, change: LogicalChange) -> Result<()> {
        // Check if this change matches any publication
        let publications = self.publications.read();
        let mut matched = false;

        for publication in publications.values() {
            if self.matches_publication(&change, publication) {
                matched = true;
                break;
            }
        }

        if !matched {
            return Ok(()); // Change not published
        }

        // Increment LSN
        {
            let mut lsn = self.current_lsn.write();
            *lsn += 1;
        }

        // Queue change for replication
        self.change_tx.send(change.clone())
            .map_err(|e| DbError::Replication(format!("Failed to queue change: {}", e)))?;

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_changes += 1;

            match change.change_type {
                ChangeType::Insert => stats.inserts += 1,
                ChangeType::Update => stats.updates += 1,
                ChangeType::Delete => stats.deletes += 1,
                ChangeType::Truncate => stats.truncates += 1,
                ChangeType::CommitTransaction => stats.transactions += 1,
                _ => {}
            }

            let table_key = format!("{}.{}", change.schema, change.table);
            *stats.changes_by_table.entry(table_key).or_insert(0) += 1;
        }

        Ok(())
    }

    /// Check if a change matches a publication
    fn matches_publication(&self, change: &LogicalChange, publication: &Publication) -> bool {
        for table_pub in &publication.tables {
            if table_pub.schema_name == change.schema &&
               table_pub.table_name == change.table {
                // Check operation type
                match change.change_type {
                    ChangeType::Insert if !table_pub.replicate_insert => continue,
                    ChangeType::Update if !table_pub.replicate_update => continue,
                    ChangeType::Delete if !table_pub.replicate_delete => continue,
                    ChangeType::Truncate if !publication.replicate_truncate => continue,
                    _ => {}
                }

                return true;
            }
        }

        false
    }

    /// Apply transformations to a change
    pub fn apply_transformations(
        &self,
        change: &LogicalChange,
        table_pub: &TablePublication,
    ) -> Result<LogicalChange> {
        let mut transformed = change.clone();

        // Apply column filtering
        if let Some(columns) = &table_pub.columns {
            let column_set: HashSet<_> = columns.iter().collect();
            let mut new_columns = Vec::new();
            let mut new_old_row = Vec::new();
            let mut new_new_row = Vec::new();

            for (i, col) in change.columns.iter().enumerate() {
                if column_set.contains(col) {
                    new_columns.push(col.clone());

                    if let Some(ref old_row) = change.old_row {
                        if i < old_row.len() {
                            new_old_row.push(old_row[i].clone());
                        }
                    }

                    if let Some(ref new_row) = change.new_row {
                        if i < new_row.len() {
                            new_new_row.push(new_row[i].clone());
                        }
                    }
                }
            }

            transformed.columns = new_columns;
            transformed.old_row = if new_old_row.is_empty() { None } else { Some(new_old_row) };
            transformed.new_row = if new_new_row.is_empty() { None } else { Some(new_new_row) };
        }

        // Apply transformations
        for trans in &table_pub.transformations {
            transformed = self.apply_transformation(transformed, trans)?;

            let mut stats = self.stats.write();
            stats.transformations_applied += 1;
        }

        Ok(transformed)
    }

    /// Apply a single transformation
    fn apply_transformation(
        &self,
        mut change: LogicalChange,
        trans: &Transformation,
    ) -> Result<LogicalChange> {
        match trans {
            Transformation::Rename { from, to } => {
                for col in &mut change.columns {
                    if col == from {
                        *col = to.clone();
                    }
                }
            }
            Transformation::Mask { column, mask_type } => {
                if let Some(idx) = change.columns.iter().position(|c| c == column) {
                    let masked_value = match mask_type {
                        MaskType::Null => None,
                        MaskType::Hash => {
                            // Simple hash transformation
                            change.new_row.as_ref()
                                .and_then(|row| row.get(idx))
                                .and_then(|v| v.as_ref())
                                .map(|v| {
                                    use sha2::{Sha256, Digest};
                                    let mut hasher = Sha256::new();
                                    hasher.update(v);
                                    hasher.finalize().to_vec()
                                })
                        }
                        MaskType::Partial(n) => {
                            change.new_row.as_ref()
                                .and_then(|row| row.get(idx))
                                .and_then(|v| v.as_ref())
                                .map(|v| {
                                    let s = String::from_utf8_lossy(v);
                                    let len = s.len();
                                    if len > *n {
                                        let masked = "*".repeat(len - n) + &s[len - n..];
                                        masked.into_bytes()
                                    } else {
                                        v.clone()
                                    }
                                })
                        }
                        MaskType::Constant(val) => Some(val.clone()),
                    };

                    if let Some(ref mut new_row) = change.new_row {
                        if idx < new_row.len() {
                            new_row[idx] = masked_value;
                        }
                    }
                }
            }
            _ => {
                // Other transformations would be implemented here
            }
        }

        Ok(change)
    }

    /// Stream changes to a subscription
    pub async fn stream_changes(&self, subscription_name: &str) -> Result<Vec<LogicalChange>> {
        let sub = {
            let subs = self.subscriptions.read();
            subs.get(subscription_name)
                .ok_or_else(|| DbError::Replication(
                    format!("Subscription {} not found", subscription_name)
                ))?
                .clone()
        };

        if !sub.enabled {
            return Err(DbError::Replication(
                format!("Subscription {} is disabled", subscription_name)
            ));
        }

        // Get changes from buffer
        let mut changes = Vec::new();
        let mut buffer = self.change_buffer.write();

        while let Some(change) = buffer.pop_front() {
            if change.lsn > sub.last_lsn {
                changes.push(change);
            }
        }

        Ok(changes)
    }

    /// Update subscription LSN
    pub fn update_subscription_lsn(&self, name: &str, lsn: u64) -> Result<()> {
        let mut subs = self.subscriptions.write();

        let sub = subs.get_mut(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        sub.last_lsn = lsn;
        Ok(())
    }

    /// Get current LSN
    pub fn get_current_lsn(&self) -> u64 {
        *self.current_lsn.read()
    }

    /// Get subscription state
    pub fn get_subscription_state(&self, name: &str) -> Result<SubscriptionState> {
        let subs = self.subscriptions.read();

        let sub = subs.get(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        Ok(sub.state.clone())
    }

    /// Update subscription state
    pub fn update_subscription_state(&self, name: &str, state: SubscriptionState) -> Result<()> {
        let mut subs = self.subscriptions.write();

        let sub = subs.get_mut(name)
            .ok_or_else(|| DbError::Replication(
                format!("Subscription {} not found", name)
            ))?;

        sub.state = state;
        Ok(())
    }

    /// Handle schema evolution
    pub fn handle_schema_change(
        &self,
        schema: &str,
        table: &str,
        change_type: SchemaChangeType,
    ) -> Result<()> {
        // Check which publications are affected
        let publications = self.publications.read();

        for publication in publications.values() {
            for table_pub in &publication.tables {
                if table_pub.schema_name == schema && table_pub.table_name == table {
                    match change_type {
                        SchemaChangeType::AddColumn(ref col_name) => {
                            // Column additions are generally safe
                            println!("Column {} added to {}.{}", col_name, schema, table);
                        }
                        SchemaChangeType::DropColumn(ref col_name) => {
                            // Check if this column is being replicated
                            if let Some(ref cols) = table_pub.columns {
                                if cols.contains(col_name) {
                                    return Err(DbError::Replication(
                                        format!("Cannot drop column {} being replicated", col_name)
                                    ));
                                }
                            }
                        }
                        SchemaChangeType::RenameColumn { ref from, ref to } => {
                            println!("Column {}.{}.{} renamed to {}", schema, table, from, to);
                        }
                        SchemaChangeType::ChangeType { ref column, ref old_type, ref new_type } => {
                            println!("Column {}.{}.{} type changed from {} to {}",
                                   schema, table, column, old_type, new_type);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> LogicalReplicationStats {
        self.stats.read().clone()
    }

    /// Get all publications
    pub fn get_publications(&self) -> Vec<Publication> {
        self.publications.read().values().cloned().collect()
    }

    /// Get all subscriptions
    pub fn get_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.read().values().cloned().collect()
    }
}

/// Type of schema change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaChangeType {
    AddColumn(String),
    DropColumn(String),
    RenameColumn { from: String, to: String },
    ChangeType { column: String, old_type: String, new_type: String },
}

impl Default for LogicalReplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_publication() {
        let lr = LogicalReplication::new();

        let publication = Publication {
            name: "pub1".to_string(),
            tables: vec![],
            replicate_ddl: false,
            replicate_truncate: true,
            owner: "admin".to_string(),
            created_at: 0,
        };

        lr.create_publication(publication).unwrap();

        let pubs = lr.get_publications();
        assert_eq!(pubs.len(), 1);
        assert_eq!(pubs[0].name, "pub1");
    }

    #[test]
    fn test_create_subscription() {
        let lr = LogicalReplication::new();

        let subscription = Subscription {
            name: "sub1".to_string(),
            publication_name: "pub1".to_string(),
            source_connection: "host=localhost".to_string(),
            schema_mapping: HashMap::new(),
            state: SubscriptionState::Initializing,
            last_lsn: 0,
            created_at: 0,
            enabled: true,
        };

        lr.create_subscription(subscription).unwrap();

        let subs = lr.get_subscriptions();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "sub1");
    }
}


