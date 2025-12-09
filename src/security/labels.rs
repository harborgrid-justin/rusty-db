// # Security Labels Module
//
// Provides mandatory access control (MAC) through security labels,
// data classification levels, compartment-based security, and
// label-based row filtering.
//
// ## Features
//
// - Mandatory Access Control (MAC)
// - Multi-Level Security (MLS)
// - Compartment-based security
// - Label-based row filtering
// - Label dominance comparison
// - Label composition and decomposition

use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use parking_lot::RwLock;
use std::sync::Arc;
use std::cmp::Ordering;
use crate::Result;
use crate::error::DbError;

/// Security classification level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClassificationLevel {
    /// Unclassified
    Unclassified,
    /// Restricted
    Restricted,
    /// Confidential
    Confidential,
    /// Secret
    Secret,
    /// TopSecret
    TopSecret,
    /// Custom level with numeric value
    Custom { name: String, level: u32 },
}

/// Security compartment (category)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Compartment {
    /// Compartment ID
    pub id: String,
    /// Compartment name
    pub name: String,
    /// Compartment description
    pub description: Option<String>,
    /// Parent compartment (for hierarchical compartments)
    pub parent: Option<String>,
}

/// Security label combining classification and compartments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityLabel {
    /// Classification level
    pub classification: ClassificationLevel,
    /// Set of compartments
    pub compartments: HashSet<String>,
    /// Optional groups (for group-based access)
    pub groups: HashSet<String>,
    /// Label metadata
    pub metadata: HashMap<String, String>,
}

impl SecurityLabel {
    /// Create a new security label
    pub fn new(classification: ClassificationLevel) -> Self {
        Self {
            classification,
            compartments: HashSet::new(),
            groups: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a compartment to the label
    pub fn add_compartment(&mut self, compartment: String) {
        self.compartments.insert(compartment);
    }

    /// Add a group to the label
    pub fn add_group(&mut self, group: String) {
        self.groups.insert(group);
    }

    /// Check if this label dominates another label
    pub fn dominates(&self, other: &SecurityLabel) -> bool {
        // Classification must be >= other's classification
        if self.classification < other.classification {
            return false;
        }

        // Must have all of other's compartments
        if !other.compartments.is_subset(&self.compartments) {
            return false;
        }

        // Must have all of other's groups
        if !other.groups.is_subset(&self.groups) {
            return false;
        }

        true
    }

    /// Check if this label is dominated by another label
    pub fn dominated_by(&self, other: &SecurityLabel) -> bool {
        other.dominates(self)
    }

    /// Check if labels are comparable (one dominates the other)
    pub fn comparable(&self, other: &SecurityLabel) -> bool {
        self.dominates(other) || other.dominates(self)
    }

    /// Compare labels (for partial ordering)
    pub fn partial_cmp_label(&self, other: &SecurityLabel) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }

        if self.dominates(other) {
            return Some(Ordering::Greater);
        }

        if other.dominates(self) {
            return Some(Ordering::Less);
        }

        None // Not comparable
    }
}

/// User clearance (maximum label a user can read/write)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserClearance {
    /// User ID
    pub user_id: String,
    /// Maximum read label
    pub max_read: SecurityLabel,
    /// Maximum write label
    pub max_write: SecurityLabel,
    /// Current working label
    pub current_label: SecurityLabel,
    /// Authorized compartments
    pub authorized_compartments: HashSet<String>,
    /// Authorized groups
    pub authorized_groups: HashSet<String>,
}

/// Row-level label assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowLabel {
    /// Table ID
    pub table_id: String,
    /// Row ID/primary key
    pub row_id: String,
    /// Security label
    pub label: SecurityLabel,
    /// Label assignment timestamp
    pub assigned_at: i64,
    /// Label assigned by
    pub assigned_by: String,
}

/// Label policy for a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Table this policy applies to
    pub table_id: String,
    /// Label column name
    pub label_column: String,
    /// Default label for new rows
    pub default_label: Option<SecurityLabel>,
    /// Read access policy
    pub read_policy: LabelAccessPolicy,
    /// Write access policy
    pub write_policy: LabelAccessPolicy,
    /// Whether policy is enabled
    pub enabled: bool,
}

/// Label access policy type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LabelAccessPolicy {
    /// User must dominate row label
    ReadDown,
    /// User label must equal row label
    ReadEqual,
    /// User label can be dominated by row label
    ReadUp,
    /// Custom policy with expression
    Custom { expression: String },
}

/// Label operation
#[derive(Debug, Clone)]
pub enum LabelOperation {
    /// Check if user can read row with label
    CanRead { user_label: SecurityLabel, row_label: SecurityLabel },
    /// Check if user can write row with label
    CanWrite { user_label: SecurityLabel, row_label: SecurityLabel },
    /// Check if user can upgrade label
    CanUpgrade { from_label: SecurityLabel, to_label: SecurityLabel },
    /// Check if user can downgrade label
    CanDowngrade { from_label: SecurityLabel, to_label: SecurityLabel },
}

/// Label manager
pub struct LabelManager {
    /// Defined compartments
    compartments: Arc<RwLock<HashMap<String, Compartment>>>,
    /// User clearances
    clearances: Arc<RwLock<HashMap<String, UserClearance>>>,
    /// Row labels
    row_labels: Arc<RwLock<HashMap<String, HashMap<String, RowLabel>>>>, // table_id -> row_id -> label
    /// Label policies
    policies: Arc<RwLock<HashMap<String, LabelPolicy>>>,
    /// Label statistics
    statistics: Arc<RwLock<LabelStatistics>>,
}

impl LabelManager {
    /// Create a new label manager
    pub fn new() -> Self {
        Self {
            compartments: Arc::new(RwLock::new(HashMap::new())),
            clearances: Arc::new(RwLock::new(HashMap::new())),
            row_labels: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(LabelStatistics::default())),
        }
    }

    /// Register a compartment
    pub fn register_compartment(&self, compartment: Compartment) -> Result<()> {
        let mut compartments = self.compartments.write();

        if compartments.contains_key(&compartment.id) {
            return Err(DbError::AlreadyExists(
                format!("Compartment {} already exists", compartment.id)
            ));
        }

        // Validate parent exists if specified
        if let Some(ref parent) = compartment.parent {
            if !compartments.contains_key(parent) {
                return Err(DbError::NotFound(
                    format!("Parent compartment {} not found", parent)
                ));
            }
        }

        compartments.insert(compartment.id.clone(), compartment);
        Ok(())
    }

    /// Get a compartment
    pub fn get_compartment(&self, id: &str) -> Result<Compartment> {
        self.compartments.read()
            .get(id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Compartment {} not found", id)))
    }

    /// Get all compartments
    pub fn get_all_compartments(&self) -> Vec<Compartment> {
        self.compartments.read().values().cloned().collect()
    }

    /// Set user clearance
    pub fn set_user_clearance(&self, clearance: UserClearance) -> Result<()> {
        // Validate compartments exist
        let compartments = self.compartments.read();
        for comp in &clearance.authorized_compartments {
            if !compartments.contains_key(comp) {
                return Err(DbError::NotFound(format!("Compartment {} not found", comp)));
            }
        }

        // Validate current label is within clearance
        if !clearance.max_read.dominates(&clearance.current_label) {
            return Err(DbError::InvalidInput(
                "Current label exceeds max read clearance".to_string()
            ));
        }

        self.clearances.write().insert(clearance.user_id.clone(), clearance);
        Ok(())
    }

    /// Get user clearance
    pub fn get_user_clearance(&self, user_id: &str) -> Result<UserClearance> {
        self.clearances.read()
            .get(user_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Clearance not found for user {}", user_id)))
    }

    /// Set label for a row
    pub fn set_row_label(
        &self,
        table_id: String,
        row_id: String,
        label: SecurityLabel,
        assigned_by: String,
    ) -> Result<()> {
        // Validate compartments in label
        let compartments = self.compartments.read();
        for comp in &label.compartments {
            if !compartments.contains_key(comp) {
                return Err(DbError::NotFound(format!("Compartment {} not found", comp)));
            }
        }

        let row_label = RowLabel {
            table_id: table_id.clone(),
            row_id: row_id.clone(),
            label,
            assigned_at: current_timestamp(),
            assigned_by,
        };

        let mut row_labels = self.row_labels.write();
        let table_labels = row_labels.entry(table_id).or_insert_with(HashMap::new);
        table_labels.insert(row_id, row_label);

        // Update statistics
        self.statistics.write().total_labeled_rows += 1;

        Ok(())
    }

    /// Get label for a row
    pub fn get_row_label(&self, table_id: &str, row_id: &str) -> Option<SecurityLabel> {
        self.row_labels.read()
            .get(table_id)
            .and_then(|table| table.get(row_id))
            .map(|row_label| row_label.label.clone())
    }

    /// Check if user can read a row
    pub fn can_read_row(
        &self,
        user_id: &str,
        table_id: &str,
        row_id: &str,
    ) -> Result<bool> {
        let clearance = self.get_user_clearance(user_id)?;

        // Get row label
        let row_label = match self.get_row_label(table_id, row_id) {
            Some(label) => label,
            None => return Ok(true), // No label = unrestricted
        };

        // Get policy for table
        let policy = self.get_table_policy(table_id)?;

        match policy.read_policy {
            LabelAccessPolicy::ReadDown => {
                // User must dominate row label
                Ok(clearance.current_label.dominates(&row_label))
            }
            LabelAccessPolicy::ReadEqual => {
                // Labels must be equal
                Ok(clearance.current_label == row_label)
            }
            LabelAccessPolicy::ReadUp => {
                // User label can be dominated
                Ok(row_label.dominates(&clearance.current_label))
            }
            LabelAccessPolicy::Custom { .. } => {
                // Would evaluate custom expression
                Ok(true)
            }
        }
    }

    /// Check if user can write a row
    pub fn can_write_row(
        &self,
        user_id: &str,
        table_id: &str,
        row_id: &str,
    ) -> Result<bool> {
        let clearance = self.get_user_clearance(user_id)?;

        let row_label = match self.get_row_label(table_id, row_id) {
            Some(label) => label,
            None => return Ok(true),
        };

        let policy = self.get_table_policy(table_id)?;

        match policy.write_policy {
            LabelAccessPolicy::ReadDown => {
                Ok(clearance.current_label.dominates(&row_label) &&
                   clearance.max_write.dominates(&row_label))
            }
            LabelAccessPolicy::ReadEqual => {
                Ok(clearance.current_label == row_label)
            }
            LabelAccessPolicy::ReadUp => {
                Ok(row_label.dominates(&clearance.current_label))
            }
            LabelAccessPolicy::Custom { .. } => {
                Ok(true)
            }
        }
    }

    /// Filter rows based on user's label clearance
    pub fn filter_readable_rows(
        &self,
        user_id: &str,
        table_id: &str,
        row_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        let clearance = self.get_user_clearance(user_id)?;
        let policy = self.get_table_policy(table_id)?;

        let row_labels = self.row_labels.read();
        let table_labels = row_labels.get(table_id);

        let mut readable = Vec::new();

        for row_id in row_ids {
            let can_read = if let Some(table) = table_labels {
                if let Some(row_label_entry) = table.get(&row_id) {
                    match policy.read_policy {
                        LabelAccessPolicy::ReadDown => {
                            clearance.current_label.dominates(&row_label_entry.label)
                        }
                        LabelAccessPolicy::ReadEqual => {
                            clearance.current_label == row_label_entry.label
                        }
                        LabelAccessPolicy::ReadUp => {
                            row_label_entry.label.dominates(&clearance.current_label)
                        }
                        LabelAccessPolicy::Custom { .. } => true,
                    }
                } else {
                    true // No label = readable
                }
            } else {
                true // No labels for table
            };

            if can_read {
                readable.push(row_id);
            }
        }

        Ok(readable)
    }

    /// Create a label policy for a table
    pub fn create_label_policy(&self, policy: LabelPolicy) -> Result<()> {
        let mut policies = self.policies.write();

        if policies.contains_key(&policy.id) {
            return Err(DbError::AlreadyExists(
                format!("Label policy {} already exists", policy.id)
            ));
        }

        policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    /// Get label policy for a table
    pub fn get_table_policy(&self, table_id: &str) -> Result<LabelPolicy> {
        self.policies.read()
            .values()
            .find(|p| p.table_id == table_id && p.enabled)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("No active label policy for table {}", table_id)))
    }

    /// Update user's current working label
    pub fn set_user_label(&self, user_id: &str, label: SecurityLabel) -> Result<()> {
        let mut clearances = self.clearances.write();
        let clearance = clearances.get_mut(user_id)
            .ok_or_else(|| DbError::NotFound(format!("Clearance not found for user {}", user_id)))?;

        // Validate new label is within clearance
        if !clearance.max_read.dominates(&label) {
            return Err(DbError::InvalidInput(
                "Label exceeds user's maximum clearance".to_string()
            ));
        }

        // Validate all compartments are authorized
        if !label.compartments.is_subset(&clearance.authorized_compartments) {
            return Err(DbError::InvalidInput(
                "User not authorized for all compartments".to_string()
            ));
        }

        clearance.current_label = label;
        Ok(())
    }

    /// Combine two labels (least upper bound)
    pub fn combine_labels(&self, label1: &SecurityLabel, label2: &SecurityLabel) -> SecurityLabel {
        let classification = if label1.classification > label2.classification {
            label1.classification.clone()
        } else {
            label2.classification.clone()
        };

        let mut compartments = label1.compartments.clone();
        compartments.extend(label2.compartments.iter().cloned());

        let mut groups = label1.groups.clone();
        groups.extend(label2.groups.iter().cloned());

        SecurityLabel {
            classification,
            compartments,
            groups,
            metadata: HashMap::new(),
        }
    }

    /// Intersect two labels (greatest lower bound)
    pub fn intersect_labels(&self, label1: &SecurityLabel, label2: &SecurityLabel) -> SecurityLabel {
        let classification = if label1.classification < label2.classification {
            label1.classification.clone()
        } else {
            label2.classification.clone()
        };

        let compartments: HashSet<String> = label1.compartments
            .intersection(&label2.compartments)
            .cloned()
            .collect();

        let groups: HashSet<String> = label1.groups
            .intersection(&label2.groups)
            .cloned()
            .collect();

        SecurityLabel {
            classification,
            compartments,
            groups,
            metadata: HashMap::new(),
        }
    }

    /// Get statistics
    pub fn get_statistics(&self) -> LabelStatistics {
        self.statistics.read().clone()
    }

    /// Update statistics
    pub fn update_statistics(&self) {
        let mut stats = self.statistics.write();

        stats.total_compartments = self.compartments.read().len();
        stats.total_clearances = self.clearances.read().len();
        stats.total_policies = self.policies.read().len();

        let row_labels = self.row_labels.read();
        stats.total_labeled_rows = row_labels.values()
            .map(|table| table.len())
            .sum();

        // Count rows by classification
        stats.rows_by_classification.clear();
        for table_labels in row_labels.values() {
            for row_label in table_labels.values() {
                let classification = format!("{:?}", row_label.label.classification);
                *stats.rows_by_classification.entry(classification).or_insert(0) += 1;
            }
        }
    }
}

impl Default for LabelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Label statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStatistics {
    pub total_compartments: usize,
    pub total_clearances: usize,
    pub total_policies: usize,
    pub total_labeled_rows: usize,
    pub rows_by_classification: HashMap<String, usize>,
}

impl Default for LabelStatistics {
    fn default() -> Self {
        Self {
            total_compartments: 0,
            total_clearances: 0,
            total_policies: 0,
            total_labeled_rows: 0,
            rows_by_classification: HashMap::new(),
        }
    }
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_dominance() {
        let mut label1 = SecurityLabel::new(ClassificationLevel::Secret);
        label1.add_compartment("NATO".to_string());
        label1.add_compartment("INTEL".to_string());

        let mut label2 = SecurityLabel::new(ClassificationLevel::Confidential);
        label2.add_compartment("NATO".to_string());

        assert!(label1.dominates(&label2));
        assert!(!label2.dominates(&label1));
    }

    #[test]
    fn test_compartment_registration() {
        let manager = LabelManager::new();

        let compartment = Compartment {
            id: "NATO".to_string(),
            name: "NATO Restricted".to_string(),
            description: Some("NATO alliance information".to_string()),
            parent: None,
        };

        assert!(manager.register_compartment(compartment).is_ok());
    }

    #[test]
    fn test_row_label_filtering() {
        let manager = LabelManager::new();

        // Register compartment
        let compartment = Compartment {
            id: "SECRET".to_string(),
            name: "Secret".to_string(),
            description: None,
            parent: None,
        };
        manager.register_compartment(compartment).unwrap();

        // Set user clearance
        let mut max_read = SecurityLabel::new(ClassificationLevel::Secret);
        max_read.add_compartment("SECRET".to_string());

        let clearance = UserClearance {
            user_id: "user1".to_string(),
            max_read: max_read.clone(),
            max_write: max_read.clone(),
            current_label: max_read,
            authorized_compartments: HashSet::from(["SECRET".to_string()]),
            authorized_groups: HashSet::new(),
        };
        manager.set_user_clearance(clearance).unwrap();

        // Create policy
        let policy = LabelPolicy {
            id: "pol1".to_string(),
            name: "Test Policy".to_string(),
            table_id: "table1".to_string(),
            label_column: "label".to_string(),
            default_label: None,
            read_policy: LabelAccessPolicy::ReadDown,
            write_policy: LabelAccessPolicy::ReadDown,
            enabled: true,
        };
        manager.create_label_policy(policy).unwrap();

        // Set row labels
        let mut row1_label = SecurityLabel::new(ClassificationLevel::Confidential);
        row1_label.add_compartment("SECRET".to_string());

        manager.set_row_label(
            "table1".to_string(),
            "row1".to_string(),
            row1_label,
            "admin".to_string(),
        ).unwrap();

        // Test read access
        assert!(manager.can_read_row("user1", "table1", "row1").unwrap());
    }

    #[test]
    fn test_label_combination() {
        let manager = LabelManager::new();

        let mut label1 = SecurityLabel::new(ClassificationLevel::Secret);
        label1.add_compartment("A".to_string());

        let mut label2 = SecurityLabel::new(ClassificationLevel::Confidential);
        label2.add_compartment("B".to_string());

        let combined = manager.combine_labels(&label1, &label2);

        assert_eq!(combined.classification, ClassificationLevel::Secret);
        assert!(combined.compartments.contains("A"));
        assert!(combined.compartments.contains("B"));
    }
}
