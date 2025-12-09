// # Data Masking Engine
//
// Oracle-like data masking for protecting sensitive data in non-production environments
// and real-time query masking for production access control.
//
// ## Features
//
// - **Static Masking**: One-time masking for database clones
// - **Dynamic Masking**: Real-time masking in query results
// - **Format-Preserving Encryption (FPE)**: Maintain data format
// - **Consistent Masking**: Same input produces same masked output
// - **Custom Functions**: User-defined masking logic
//
// ## Masking Types
//
// - **Full Masking**: Replace with generic value
// - **Partial Masking**: Show subset (e.g., last 4 digits)
// - **Shuffle**: Randomize within same data set
// - **Substitution**: Replace with realistic fake data
// - **Nullify**: Replace with NULL
// - **Hash**: One-way hashing with salt

use crate::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use sha2::{Sha256, Digest};
use rand::Rng;
use regex::Regex;

/// Masking type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaskingType {
    /// Replace entire value with fixed string
    FullMask(String),
    /// Show only last N characters
    PartialMask { show_last: usize },
    /// Show only first N characters
    PartialMaskFirst { show_first: usize },
    /// Randomize value while maintaining format
    Shuffle,
    /// Replace with random value from substitution table
    Substitution { table: String },
    /// Replace with NULL
    Nullify,
    /// One-way hash with salt
    Hash { salt: String },
    /// Format-preserving encryption
    FormatPreserving,
    /// Email masking (preserve domain)
    EmailMask,
    /// Credit card masking (show last 4)
    CreditCardMask,
    /// SSN masking (show last 4)
    SsnMask,
    /// Phone number masking
    PhoneMask,
    /// Custom function
    Custom { function_name: String },
}

impl MaskingType {
    /// Parse masking type from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "FULL_MASK" => Ok(Self::FullMask("***MASKED***".to_string())),
            "PARTIAL_MASK" => Ok(Self::PartialMask { show_last: 4 }),
            "SHUFFLE" => Ok(Self::Shuffle),
            "NULLIFY" => Ok(Self::Nullify),
            "HASH" => Ok(Self::Hash { salt: "default_salt".to_string() }),
            "FPE" | "FORMAT_PRESERVING" => Ok(Self::FormatPreserving),
            "EMAIL_MASK" => Ok(Self::EmailMask),
            "CREDIT_CARD_MASK" | "CC_MASK" => Ok(Self::CreditCardMask),
            "SSN_MASK" => Ok(Self::SsnMask),
            "PHONE_MASK" => Ok(Self::PhoneMask),
            _ => Err(DbError::InvalidInput(format!("Unknown masking type: {}", s))),
        }
    }
}

/// Masking policy defining how data should be masked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingPolicy {
    /// Policy name
    pub name: String,
    /// Column pattern (regex)
    pub column_pattern: String,
    /// Table pattern (optional)
    pub table_pattern: Option<String>,
    /// Masking type
    pub masking_type: MaskingType,
    /// Enabled flag
    pub enabled: bool,
    /// Consistency key (for deterministic masking)
    pub consistency_key: Option<String>,
    /// Created timestamp
    pub created_at: i64,
    /// Priority (higher priority applied first)
    pub priority: i32,
}

impl MaskingPolicy {
    /// Create a new masking policy
    pub fn new(name: String, column_pattern: String, masking_type: MaskingType) -> Self {
        Self {
            name,
            column_pattern,
            table_pattern: None,
            masking_type,
            enabled: true,
            consistency_key: None,
            created_at: chrono::Utc::now().timestamp(),
            priority: 0,
        }
    }

    /// Check if policy applies to a column
    pub fn applies_to(&self, table: &str, column: &str) -> Result<bool> {
        // Check table pattern if specified
        if let Some(ref table_pat) = self.table_pattern {
            let table_regex = Regex::new(table_pat)
                .map_err(|e| DbError::InvalidInput(format!("Invalid table pattern: {}", e)))?;
            if !table_regex.is_match(table) {
                return Ok(false);
            }
        }

        // Check column pattern
        let col_regex = Regex::new(&self.column_pattern)
            .map_err(|e| DbError::InvalidInput(format!("Invalid column pattern: {}", e)))?;
        Ok(col_regex.is_match(column))
    }
}

/// Substitution table for realistic fake data
#[derive(Debug, Clone)]
struct SubstitutionTable {
    /// Table name
    name: String,
    /// Values to substitute
    values: Vec<String>,
}

impl SubstitutionTable {
    /// Get random value from table
    fn get_random(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.values.len());
        &self.values[idx]
    }

    /// Get deterministic value based on hash
    fn get_consistent(&self, input: &str, seed: &str) -> &str {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hasher.update(seed);
        let hash = hasher.finalize();
        let idx = (hash[0] as usize) % self.values.len();
        &self.values[idx]
    }
}

/// Custom masking function
pub type MaskingFunction = Box<dyn Fn(&str) -> Result<String> + Send + Sync>;

/// Main Data Masking Engine
pub struct MaskingEngine {
    /// Masking policies
    policies: RwLock<HashMap<String, MaskingPolicy>>,
    /// Substitution tables
    substitution_tables: RwLock<HashMap<String, SubstitutionTable>>,
    /// Custom masking functions
    custom_functions: RwLock<HashMap<String, MaskingFunction>>,
    /// Consistency cache (for deterministic masking)
    consistency_cache: RwLock<HashMap<String, String>>,
    /// Masking statistics
    stats: RwLock<MaskingStats>,
}

/// Masking statistics
#[derive(Debug, Default)]
struct MaskingStats {
    /// Total values masked
    total_masked: u64,
    /// Masked by policy
    by_policy: HashMap<String, u64>,
    /// Cache hits
    cache_hits: u64,
}

impl MaskingEngine {
    /// Create a new masking engine
pub fn new() -> Result<Self> {
                let mut engine = Self {
                    policies: RwLock::new(HashMap::new()),
                    substitution_tables: RwLock::new(HashMap::new()),
                    custom_functions: RwLock::new(HashMap::new()),
                    consistency_cache: RwLock::new(HashMap::new()),
                    stats: RwLock::new(MaskingStats::default()),
                };
            
                // Initialize default substitution tables
                engine.init_default_substitution_tables();
            
                Ok(engine)
            }

    /// Initialize default substitution tables
    fn init_default_substitution_tables(&mut self) {
        // First names
        let first_names = SubstitutionTable {
            name: "FIRST_NAMES".to_string(),
            values: vec![
                "John", "Jane", "Michael", "Sarah", "David", "Emily",
                "Robert", "Lisa", "William", "Jennifer", "James", "Mary",
            ].iter().map(|s| s.to_string()).collect(),
        };

        // Last names
        let last_names = SubstitutionTable {
            name: "LAST_NAMES".to_string(),
            values: vec![
                "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia",
                "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez",
            ].iter().map(|s| s.to_string()).collect(),
        };

        // Cities
        let cities = SubstitutionTable {
            name: "CITIES".to_string(),
            values: vec![
                "New York", "Los Angeles", "Chicago", "Houston", "Phoenix",
                "Philadelphia", "San Antonio", "San Diego", "Dallas", "San Jose",
            ].iter().map(|s| s.to_string()).collect(),
        };

        let mut tables = self.substitution_tables.write();
        tables.insert("FIRST_NAMES".to_string(), first_names);
        tables.insert("LAST_NAMES".to_string(), last_names);
        tables.insert("CITIES".to_string(), cities);
    }

    /// Create a masking policy
    pub fn create_policy(
        &mut self,
        name: &str,
        column_pattern: &str,
        masking_type: &str,
    ) -> Result<()> {
        let mask_type = MaskingType::from_str(masking_type)?;
        let policy = MaskingPolicy::new(
            name.to_string(),
            column_pattern.to_string(),
            mask_type,
        );

        self.policies.write().insert(name.to_string(), policy);
        Ok(())
    }

    /// Create policy with custom configuration
    pub fn create_policy_custom(&mut self, policy: MaskingPolicy) -> Result<()> {
        // Validate regex patterns
        Regex::new(&policy.column_pattern)
            .map_err(|e| DbError::InvalidInput(format!("Invalid column pattern: {}", e)))?;

        if let Some(ref table_pat) = policy.table_pattern {
            Regex::new(table_pat)
                .map_err(|e| DbError::InvalidInput(format!("Invalid table pattern: {}", e)))?;
        }

        self.policies.write().insert(policy.name.clone(), policy);
        Ok(())
    }

    /// Drop a masking policy
    pub fn drop_policy(&mut self, name: &str) -> Result<()> {
        self.policies.write().remove(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?;
        Ok(())
    }

    /// Enable a policy
    pub fn enable_policy(&mut self, name: &str) -> Result<()> {
        let mut policies = self.policies.write();
        let policy = policies.get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?;
        policy.enabled = true;
        Ok(())
    }

    /// Disable a policy
    pub fn disable_policy(&mut self, name: &str) -> Result<()> {
        let mut policies = self.policies.write();
        let policy = policies.get_mut(name)
            .ok_or_else(|| DbError::NotFound(format!("Policy not found: {}", name)))?;
        policy.enabled = false;
        Ok(())
    }

    /// Apply masking to a value (dynamic masking)
    pub fn mask_value(
        &self,
        table: &str,
        column: &str,
        value: &str,
    ) -> Result<String> {
        // Find applicable policies (sorted by priority)
        let policies = self.policies.read();
        let mut applicable: Vec<&MaskingPolicy> = policies.values()
            .filter(|p| p.enabled && p.applies_to(table, column).unwrap_or(false))
            .collect();

        if applicable.is_empty() {
            // No masking policy, return original value
            return Ok(value.to_string());
        }

        // Sort by priority (descending)
        applicable.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Apply first matching policy
        let policy = applicable[0];
        let masked = self.apply_masking(&policy.masking_type, value, policy.consistency_key.as_deref())?;

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_masked += 1;
        *stats.by_policy.entry(policy.name.clone()).or_insert(0) += 1;

        Ok(masked)
    }

    /// Apply masking based on type
    fn apply_masking(
        &self,
        masking_type: &MaskingType,
        value: &str,
        consistency_key: Option<&str>,
    ) -> Result<String> {
        // Check consistency cache if key provided
        if let Some(key) = consistency_key {
            let cache_key = format!("{}:{}", key, value);
            let cache = self.consistency_cache.read();
            if let Some(cached) = cache.get(&cache_key) {
                self.stats.write().cache_hits += 1;
                return Ok(cached.clone());
            }
        }

        let masked = match masking_type {
            MaskingType::FullMask(replacement) => replacement.clone(),

            MaskingType::PartialMask { show_last } => {
                if value.len() <= *show_last {
                    "*".repeat(value.len())
                } else {
                    let prefix_len = value.len() - show_last;
                    format!("{}{}", "*".repeat(prefix_len), &value[prefix_len..])
                }
            }

            MaskingType::PartialMaskFirst { show_first } => {
                if value.len() <= *show_first {
                    "*".repeat(value.len())
                } else {
                    format!("{}{}", &value[..*show_first], "*".repeat(value.len() - show_first))
                }
            }

            MaskingType::Shuffle => {
                let mut chars: Vec<char> = value.chars().collect();
                use rand::seq::SliceRandom;
                chars.shuffle(&mut rand::thread_rng());
                chars.iter().collect()
            }

            MaskingType::Substitution { table } => {
                let tables = self.substitution_tables.read();
                let sub_table = tables.get(table)
                    .ok_or_else(|| DbError::NotFound(format!("Substitution table not found: {}", table)))?;

                if let Some(key) = consistency_key {
                    sub_table.get_consistent(value, key).to_string()
                } else {
                    sub_table.get_random().to_string()
                }
            }

            MaskingType::Nullify => "NULL".to_string(),

            MaskingType::Hash { salt } => {
                let mut hasher = Sha256::new();
                hasher.update(value);
                hasher.update(salt);
                format!("{:x}", hasher.finalize())
            }

            MaskingType::FormatPreserving => {
                self.format_preserving_mask(value)
            }

            MaskingType::EmailMask => {
                self.mask_email(value)?
            }

            MaskingType::CreditCardMask => {
                self.mask_credit_card(value)
            }

            MaskingType::SsnMask => {
                self.mask_ssn(value)
            }

            MaskingType::PhoneMask => {
                self.mask_phone(value)
            }

            MaskingType::Custom { function_name } => {
                let functions = self.custom_functions.read();
                let func = functions.get(function_name)
                    .ok_or_else(|| DbError::NotFound(format!("Custom function not found: {}", function_name)))?;
                func(value)?
            }
        };

        // Cache the result if consistency key provided
        if let Some(key) = consistency_key {
            let cache_key = format!("{}:{}", key, value);
            self.consistency_cache.write().insert(cache_key, masked.clone());
        }

        Ok(masked)
    }

    /// Format-preserving masking (maintains data type and format)
    fn format_preserving_mask(&self, value: &str) -> String {
        let mut masked = String::new();
        for ch in value.chars() {
            if ch.is_ascii_digit() {
                masked.push(char::from_digit(rand::thread_rng().gen_range(0..10), 10).unwrap());
            } else if ch.is_ascii_alphabetic() {
                if ch.is_uppercase() {
                    masked.push(char::from(rand::thread_rng().gen_range(b'A'..=b'Z')));
                } else {
                    masked.push(char::from(rand::thread_rng().gen_range(b'a'..=b'z')));
                }
            } else {
                masked.push(ch);
            }
        }
        masked
    }

    /// Mask email address (preserve domain)
    fn mask_email(&self, value: &str) -> Result<String> {
        if let Some(at_pos) = value.find('@') {
            let (local, domain) = value.split_at(at_pos);
            let masked_local = if local.len() <= 2 {
                "*".repeat(local.len())
            } else {
                format!("{}{}", &local[..1], "*".repeat(local.len() - 1))
            };
            Ok(format!("{}{}", masked_local, domain))
        } else {
            Ok("***@***.***".to_string())
        }
    }

    /// Mask credit card (show last 4 digits)
    fn mask_credit_card(&self, value: &str) -> String {
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 4 {
            let prefix_len = digits.len() - 4;
            format!("{}{}",  "*".repeat(prefix_len), &digits[prefix_len..])
        } else {
            "*".repeat(digits.len())
        }
    }

    /// Mask SSN (show last 4 digits)
    fn mask_ssn(&self, value: &str) -> String {
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 9 {
            format!("***-**-{}", &digits[5..])
        } else {
            "***-**-****".to_string()
        }
    }

    /// Mask phone number
    fn mask_phone(&self, value: &str) -> String {
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 4 {
            let prefix_len = digits.len() - 4;
            format!("({}) ***-{}", "*".repeat(3), &digits[prefix_len..])
        } else {
            "(***) ***-****".to_string()
        }
    }

    /// Register a custom masking function
    pub fn register_custom_function(
        &mut self,
        name: String,
        function: MaskingFunction,
    ) {
        self.custom_functions.write().insert(name, function);
    }

    /// Add a substitution table
    pub fn add_substitution_table(&mut self, name: String, values: Vec<String>) -> Result<()> {
        if values.is_empty() {
            return Err(DbError::InvalidInput("Substitution table cannot be empty".to_string()));
        }

        let table = SubstitutionTable { name: name.clone(), values };
        self.substitution_tables.write().insert(name, table);
        Ok(())
    }

    /// List all policies
    pub fn list_policies(&self) -> Vec<String> {
        self.policies.read().keys().cloned().collect()
    }

    /// Get policy details
    pub fn get_policy(&self, name: &str) -> Option<MaskingPolicy> {
        self.policies.read().get(name).cloned()
    }

    /// Get masking statistics
    pub fn get_stats(&self) -> (u64, u64, HashMap<String, u64>) {
        let stats = self.stats.read();
        (stats.total_masked, stats.cache_hits, stats.by_policy.clone())
    }

    /// Clear consistency cache
    pub fn clear_cache(&mut self) {
        self.consistency_cache.write().clear();
    }

    /// Batch mask values for static masking
    pub fn batch_mask(
        &self,
        table: &str,
        column: &str,
        values: Vec<String>,
    ) -> Result<Vec<String>> {
        values.iter()
            .map(|v| self.mask_value(table, column, v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking_type_parsing() {
        assert_eq!(
            MaskingType::from_str("FULL_MASK").unwrap(),
            MaskingType::FullMask("***MASKED***".to_string())
        );
        assert!(matches!(
            MaskingType::from_str("PARTIAL_MASK").unwrap(),
            MaskingType::PartialMask { show_last: 4 }
        ));
    }

    #[test]
    fn test_create_policy() {
        let mut engine = MaskingEngine::new().unwrap();
        engine.create_policy("mask_ssn", ".*ssn.*", "SSN_MASK").unwrap();

        let policies = engine.list_policies();
        assert!(policies.contains(&"mask_ssn".to_string()));
    }

    #[test]
    fn test_partial_masking() {
        let engine = MaskingEngine::new().unwrap();
        let masked = engine.apply_masking(
            &MaskingType::PartialMask { show_last: 4 },
            "1234567890",
            None,
        ).unwrap();

        assert_eq!(masked, "******7890");
    }

    #[test]
    fn test_email_masking() {
        let engine = MaskingEngine::new().unwrap();
        let masked = engine.mask_email("john.doe@example.com").unwrap();
        assert!(masked.ends_with("@example.com"));
        assert!(masked.starts_with("j"));
    }

    #[test]
    fn test_credit_card_masking() {
        let engine = MaskingEngine::new().unwrap();
        let masked = engine.mask_credit_card("4111-1111-1111-1234");
        assert!(masked.ends_with("1234"));
        assert!(masked.contains("*"));
    }

    #[test]
    fn test_ssn_masking() {
        let engine = MaskingEngine::new().unwrap();
        let masked = engine.mask_ssn("123-45-6789");
        assert_eq!(masked, "***-**-6789");
    }

    #[test]
    fn test_hash_masking() {
        let engine = MaskingEngine::new().unwrap();
        let masked1 = engine.apply_masking(
            &MaskingType::Hash { salt: "test".to_string() },
            "secret",
            None,
        ).unwrap();
        let masked2 = engine.apply_masking(
            &MaskingType::Hash { salt: "test".to_string() },
            "secret",
            None,
        ).unwrap();

        // Same input should produce same hash
        assert_eq!(masked1, masked2);
        assert_ne!(masked1, "secret");
    }

    #[test]
    fn test_consistent_masking() {
        let engine = MaskingEngine::new().unwrap();
        let consistency_key = Some("dataset1");

        let masked1 = engine.apply_masking(
            &MaskingType::Substitution { table: "FIRST_NAMES".to_string() },
            "original_value",
            consistency_key,
        ).unwrap();

        let masked2 = engine.apply_masking(
            &MaskingType::Substitution { table: "FIRST_NAMES".to_string() },
            "original_value",
            consistency_key,
        ).unwrap();

        // Same input with same consistency key should produce same output
        assert_eq!(masked1, masked2);
    }

    #[test]
    fn test_format_preserving() {
        let engine = MaskingEngine::new().unwrap();
        let original = "ABC-123-XYZ";
        let masked = engine.format_preserving_mask(original);

        // Should maintain format
        assert_eq!(masked.len(), original.len());
        assert_eq!(masked.chars().nth(3), Some('-'));
        assert_eq!(masked.chars().nth(7), Some('-'));
    }

    #[test]
    fn test_dynamic_masking_workflow() {
        let mut engine = MaskingEngine::new().unwrap();

        // Create policy for SSN columns
        engine.create_policy("ssn_policy", "(?i)ssn|social", "SSN_MASK").unwrap();

        // Mask a value
        let masked = engine.mask_value("customers", "ssn", "123-45-6789").unwrap();
        assert_eq!(masked, "***-**-6789");

        // Non-matching column should not be masked
        let not_masked = engine.mask_value("customers", "name", "John Doe").unwrap();
        assert_eq!(not_masked, "John Doe");
    }

    #[test]
    fn test_custom_function() {
        let mut engine = MaskingEngine::new().unwrap();

        // Register custom function
        let custom_fn: MaskingFunction = Box::new(|value: &str| {
            Ok(format!("CUSTOM_{}", value.to_uppercase()))
        });
        engine.register_custom_function("my_mask".to_string(), custom_fn);

        // Use custom function
        let masked = engine.apply_masking(
            &MaskingType::Custom { function_name: "my_mask".to_string() },
            "test",
            None,
        ).unwrap();

        assert_eq!(masked, "CUSTOM_TEST");
    }
}
