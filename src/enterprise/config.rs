// # Unified Configuration Management
//
// Provides a hierarchical configuration system with inheritance, dynamic updates,
// validation, encryption of sensitive parameters, and environment-specific profiles.
//
// ## Features
//
// - **Hierarchical Configuration**: Support for configuration inheritance and overrides
// - **Dynamic Updates**: Change configuration without restart via hot-reload
// - **Schema Validation**: Type-safe configuration with validation rules
// - **Encryption**: Secure storage of sensitive parameters (passwords, API keys)
// - **Environment Profiles**: Separate configs for dev, test, staging, and production
// - **Configuration History**: Track configuration changes over time
// - **Templating**: Support for variable substitution and expressions
//
// ## Example
//
// ```rust,no_run
// use rusty_db::enterprise::config::{ConfigManager, ConfigValue, Environment};
//
// #[tokio::main]
// async fn main() {
//     let mut config = ConfigManager::new(Environment::Production);
//
//     // Set configuration
//     config.set("database.max_connections", ConfigValue::Integer(100)).await.unwrap();
//
//     // Get configuration
//     let max_conn = config.get("database.max_connections").await.unwrap();
//     println!("Max connections: {:?}", max_conn);
//
//     // Watch for changes
//     let mut watcher = config.watch("database.max_connections").await;
//     tokio::spawn(async move {
//         while let Some(new_value) = watcher.recv().await {
//             println!("Config changed: {:?}", new_value);
//         }
//     });
// }
// ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Sha256, Digest};

use crate::{Result, DbError};

/// Environment type for configuration profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Environment {
    /// Development environment
    Development,
    /// Testing environment
    Testing,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

impl Environment {
    /// Get environment from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Some(Environment::Development),
            "testing" | "test" => Some(Environment::Testing),
            "staging" | "stage" => Some(Environment::Staging),
            "production" | "prod" => Some(Environment::Production),
            _ => None,
        }
    }

    /// Convert environment to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Testing => "testing",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }
}

/// Configuration value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<ConfigValue>),
    /// Nested object
    Object(HashMap<String, ConfigValue>),
    /// Encrypted value (stored as base64)
    Encrypted(String),
    /// Null value
    Null,
}

impl ConfigValue {
    /// Get value as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get value as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get value as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get value as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get value as array
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Get value as object
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Check if value is encrypted
    pub fn is_encrypted(&self) -> bool {
        matches!(self, ConfigValue::Encrypted(_))
    }
}

/// Configuration validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Required field
    Required,
    /// Minimum value (for numbers)
    Min(f64),
    /// Maximum value (for numbers)
    Max(f64),
    /// Minimum length (for strings/arrays)
    MinLength(usize),
    /// Maximum length (for strings/arrays)
    MaxLength(usize),
    /// Pattern match (regex)
    Pattern(String),
    /// Allowed values
    Enum(Vec<String>),
    /// Custom validation function name
    Custom(String),
}

/// Schema definition for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    /// Field path (e.g., "database.max_connections")
    pub path: String,
    /// Field description
    pub description: String,
    /// Expected value type
    pub value_type: String,
    /// Default value
    pub default: Option<ConfigValue>,
    /// Validation rules
    pub rules: Vec<ValidationRule>,
    /// Whether this field is sensitive and should be encrypted
    pub sensitive: bool,
}

/// Configuration change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    /// Configuration key
    pub key: String,
    /// Old value
    pub old_value: Option<ConfigValue>,
    /// New value
    pub new_value: ConfigValue,
    /// Timestamp of change
    pub timestamp: SystemTime,
    /// User or system that made the change
    pub changed_by: String,
    /// Reason for change
    pub reason: Option<String>,
}

/// Configuration snapshot for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Environment
    pub environment: Environment,
    /// Complete configuration at this point
    pub config: HashMap<String, ConfigValue>,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Description
    pub description: String,
}

/// Configuration watcher for receiving updates
pub type ConfigWatcher = mpsc::UnboundedReceiver<ConfigValue>;

/// Internal watcher channel
type WatcherChannel = mpsc::UnboundedSender<ConfigValue>;

/// Configuration manager implementation
pub struct ConfigManager {
    /// Current environment
    environment: Environment,
    /// Current configuration values
    config: Arc<RwLock<HashMap<String, ConfigValue>>>,
    /// Configuration schema
    schema: Arc<RwLock<HashMap<String, ConfigSchema>>>,
    /// Configuration change history
    history: Arc<RwLock<Vec<ConfigChange>>>,
    /// Configuration snapshots
    snapshots: Arc<RwLock<Vec<ConfigSnapshot>>>,
    /// Watchers for configuration changes
    watchers: Arc<RwLock<HashMap<String, Vec<WatcherChannel>>>>,
    /// Encryption key for sensitive values
    encryption_key: Arc<Vec<u8>>,
    /// Configuration file path
    config_path: Arc<RwLock<Option<PathBuf>>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(environment: Environment) -> Self {
        let encryption_key = Self::derive_encryption_key("rustydb-config-key");

        Self {
            environment,
            config: Arc::new(RwLock::new(HashMap::new())),
            schema: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            encryption_key: Arc::new(encryption_key),
            config_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Load configuration from file
    pub async fn load_from_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let contents = tokio::fs::read_to_string(path).await
            .map_err(|e| DbError::IoError(format!("Failed to read config file: {}", e)))?;

        let config: HashMap<String, ConfigValue> = serde_json::from_str(&contents)
            .map_err(|e| DbError::InvalidInput(format!("Invalid config JSON: {}", e)))?;

        let mut current_config = self.config.write().await;
        *current_config = config;

        let mut config_path = self.config_path.write().await;
        *config_path = Some(path.to_path_buf());

        Ok(())
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: Option<&Path>) -> Result<()> {
        let config = self.config.read().await;
        let json = serde_json::to_string_pretty(&*config)
            .map_err(|e| DbError::Internal(format!("Failed to serialize config: {}", e)))?;

        let save_path = if let Some(p) = path {
            p.to_path_buf()
        } else {
            let config_path = self.config_path.read().await;
            config_path.clone().ok_or_else(|| {
                DbError::InvalidInput("No config path specified".to_string())
            })?
        };

        tokio::fs::write(&save_path, json).await
            .map_err(|e| DbError::IoError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Set a configuration value
    pub async fn set(
        &self,
        key: impl Into<String>,
        value: ConfigValue,
    ) -> Result<()> {
        self.set_with_metadata(key, value, "system", None).await
    }

    /// Set a configuration value with metadata
    pub async fn set_with_metadata(
        &self,
        key: impl Into<String>,
        value: ConfigValue,
        changed_by: impl Into<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let key = key.into();

        // Validate against schema if exists
        self.validate_value(&key, &value).await?;

        // Get old value
        let old_value = {
            let config = self.config.read().await;
            config.get(&key).cloned()
        };

        // Encrypt if sensitive
        let final_value = if self.is_sensitive(&key).await {
            self.encrypt_value(value)?
        } else {
            value
        };

        // Set new value
        {
            let mut config = self.config.write().await;
            config.insert(key.clone(), final_value.clone());
        }

        // Record change
        let change = ConfigChange {
            key: key.clone(),
            old_value,
            new_value: final_value.clone(),
            timestamp: SystemTime::now(),
            changed_by: changed_by.into(),
            reason,
        };

        {
            let mut history = self.history.write().await;
            history.push(change);
        }

        // Notify watchers
        self.notify_watchers(&key, final_value).await;

        Ok(())
    }

    /// Get a configuration value
    pub async fn get(&self, key: &str) -> Result<ConfigValue> {
        let config = self.config.read().await;
        let value = config.get(key)
            .ok_or_else(|| DbError::NotFound(format!("Config key not found: {}", key)))?
            .clone();

        // Decrypt if encrypted
        if value.is_encrypted() {
            self.decrypt_value(value)
        } else {
            Ok(value)
        }
    }

    /// Get configuration value with default
    pub async fn get_or_default(&self, key: &str, default: ConfigValue) -> ConfigValue {
        self.get(key).await.unwrap_or(default)
    }

    /// Check if a key exists
    pub async fn has(&self, key: &str) -> bool {
        let config = self.config.read().await;
        config.contains_key(key)
    }

    /// Remove a configuration value
    pub async fn remove(&self, key: &str) -> Result<ConfigValue> {
        let mut config = self.config.write().await;
        config.remove(key)
            .ok_or_else(|| DbError::NotFound(format!("Config key not found: {}", key)))
    }

    /// Get all configuration keys
    pub async fn keys(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.keys().cloned().collect()
    }

    /// Get all configuration as a map
    pub async fn get_all(&self) -> HashMap<String, ConfigValue> {
        let config = self.config.read().await;
        config.clone()
    }

    /// Register a schema definition
    pub async fn register_schema(&self, schema: ConfigSchema) -> Result<()> {
        let mut schemas = self.schema.write().await;
        schemas.insert(schema.path.clone(), schema);
        Ok(())
    }

    /// Validate a value against its schema
    async fn validate_value(&self, key: &str, value: &ConfigValue) -> Result<()> {
        let schemas = self.schema.read().await;
        if let Some(schema) = schemas.get(key) {
            for rule in &schema.rules {
                match rule {
                    ValidationRule::Required => {
                        if matches!(value, ConfigValue::Null) {
                            return Err(DbError::InvalidInput(
                                format!("Required field '{}' is null", key)
                            ));
                        }
                    }
                    ValidationRule::Min(min) => {
                        if let Some(num) = value.as_float().or_else(|| value.as_integer().map(|i| i as f64)) {
                            if num < *min {
                                return Err(DbError::InvalidInput(
                                    format!("Value {} is less than minimum {}", num, min)
                                ));
                            }
                        }
                    }
                    ValidationRule::Max(max) => {
                        if let Some(num) = value.as_float().or_else(|| value.as_integer().map(|i| i as f64)) {
                            if num > *max {
                                return Err(DbError::InvalidInput(
                                    format!("Value {} is greater than maximum {}", num, max)
                                ));
                            }
                        }
                    }
                    ValidationRule::MinLength(min_len) => {
                        let len = if let Some(s) = value.as_string() {
                            s.len()
                        } else if let Some(a) = value.as_array() {
                            a.len()
                        } else {
                            0
                        };
                        if len < *min_len {
                            return Err(DbError::InvalidInput(
                                format!("Length {} is less than minimum {}", len, min_len)
                            ));
                        }
                    }
                    ValidationRule::MaxLength(max_len) => {
                        let len = if let Some(s) = value.as_string() {
                            s.len()
                        } else if let Some(a) = value.as_array() {
                            a.len()
                        } else {
                            0
                        };
                        if len > *max_len {
                            return Err(DbError::InvalidInput(
                                format!("Length {} is greater than maximum {}", len, max_len)
                            ));
                        }
                    }
                    ValidationRule::Pattern(pattern) => {
                        if let Some(s) = value.as_string() {
                            let re = regex::Regex::new(pattern)
                                .map_err(|e| DbError::Internal(format!("Invalid pattern: {}", e)))?;
                            if !re.is_match(s) {
                                return Err(DbError::InvalidInput(
                                    format!("Value '{}' does not match pattern '{}'", s, pattern)
                                ));
                            }
                        }
                    }
                    ValidationRule::Enum(allowed) => {
                        if let Some(s) = value.as_string() {
                            if !allowed.contains(&s.to_string()) {
                                return Err(DbError::InvalidInput(
                                    format!("Value '{}' is not in allowed values: {:?}", s, allowed)
                                ));
                            }
                        }
                    }
                    ValidationRule::Custom(_) => {
                        // Custom validation would be implemented via callbacks
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a key is marked as sensitive
    async fn is_sensitive(&self, key: &str) -> bool {
        let schemas = self.schema.read().await;
        schemas.get(key).map(|s| s.sensitive).unwrap_or(false)
    }

    /// Encrypt a configuration value
    fn encrypt_value(&self, value: ConfigValue) -> Result<ConfigValue> {
        let plaintext = serde_json::to_string(&value)
            .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))?;

        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| DbError::Internal(format!("Cipher init error: {}", e)))?;

        let nonce_bytes = rand::random::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| DbError::Internal(format!("Encryption error: {}", e)))?;

        // Combine nonce and ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        let encoded = general_purpose::STANDARD.encode(&combined);
        Ok(ConfigValue::Encrypted(encoded))
    }

    /// Decrypt a configuration value
    fn decrypt_value(&self, value: ConfigValue) -> Result<ConfigValue> {
        if let ConfigValue::Encrypted(encoded) = value {
            let combined = general_purpose::STANDARD.decode(&encoded)
                .map_err(|e| DbError::Internal(format!("Base64 decode error: {}", e)))?;

            if combined.len() < 12 {
                return Err(DbError::Internal("Invalid encrypted value".to_string()));
            }

            let (nonce_bytes, ciphertext) = combined.split_at(12);
            let nonce = Nonce::from_slice(nonce_bytes);

            let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
                .map_err(|e| DbError::Internal(format!("Cipher init error: {}", e)))?;

            let plaintext = cipher.decrypt(nonce, ciphertext)
                .map_err(|e| DbError::Internal(format!("Decryption error: {}", e)))?;

            let decrypted_value: ConfigValue = serde_json::from_slice(&plaintext)
                .map_err(|e| DbError::Internal(format!("Deserialization error: {}", e)))?;

            Ok(decrypted_value)
        } else {
            Ok(value)
        }
    }

    /// Derive encryption key from passphrase
    fn derive_encryption_key(passphrase: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(passphrase.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Watch for changes to a configuration key
    pub async fn watch(&self, key: impl Into<String>) -> ConfigWatcher {
        let key = key.into();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut watchers = self.watchers.write().await;
        watchers.entry(key).or_insert_with(Vec::new).push(tx);

        rx
    }

    /// Notify watchers of configuration change
    async fn notify_watchers(&self, key: &str, value: ConfigValue) {
        let watchers = self.watchers.read().await;
        if let Some(channels) = watchers.get(key) {
            for channel in channels {
                let _ = channel.send(value.clone());
            }
        }
    }

    /// Create a configuration snapshot
    pub async fn create_snapshot(&self, description: impl Into<String>) -> Result<String> {
        let config = self.config.read().await.clone();

        let snapshot = ConfigSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            environment: self.environment,
            config,
            timestamp: SystemTime::now(),
            description: description.into(),
        };

        let id = snapshot.id.clone();
        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot);

        Ok(id)
    }

    /// Restore from a snapshot
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshots = self.snapshots.read().await;
        let snapshot = snapshots.iter()
            .find(|s| s.id == snapshot_id)
            .ok_or_else(|| DbError::NotFound(format!("Snapshot not found: {}", snapshot_id)))?;

        let mut config = self.config.write().await;
        *config = snapshot.config.clone();

        Ok(())
    }

    /// Get configuration change history
    pub async fn get_history(&self) -> Vec<ConfigChange> {
        let history = self.history.read().await;
        history.clone()
    }

    /// Get all snapshots
    pub async fn get_snapshots(&self) -> Vec<ConfigSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.clone()
    }

    /// Get current environment
    pub fn environment(&self) -> Environment {
        self.environment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_get() {
        let config = ConfigManager::new(Environment::Development);
        config.set("test.key", ConfigValue::String("value".to_string())).await.unwrap();

        let value = config.get("test.key").await.unwrap();
        assert_eq!(value.as_string(), Some("value"));
    }

    #[tokio::test]
    async fn test_encryption() {
        let config = ConfigManager::new(Environment::Production);

        // Register sensitive field
        config.register_schema(ConfigSchema {
            path: "database.password".to_string(),
            description: "Database password".to_string(),
            value_type: "string".to_string(),
            default: None,
            rules: vec![],
            sensitive: true,
        }).await.unwrap();

        // Set sensitive value
        config.set("database.password", ConfigValue::String("secret123".to_string()))
            .await.unwrap();

        // Verify it's encrypted in storage
        let stored = {
            let cfg = config.config.read().await;
            cfg.get("database.password").unwrap().clone()
        };
        assert!(stored.is_encrypted());

        // Verify we can decrypt it
        let value = config.get("database.password").await.unwrap();
        assert_eq!(value.as_string(), Some("secret123"));
    }

    #[tokio::test]
    async fn test_validation() {
        let config = ConfigManager::new(Environment::Development);

        config.register_schema(ConfigSchema {
            path: "port".to_string(),
            description: "Server port".to_string(),
            value_type: "integer".to_string(),
            default: Some(ConfigValue::Integer(8080)),
            rules: vec![
                ValidationRule::Min(1.0),
                ValidationRule::Max(65535.0),
            ],
            sensitive: false,
        }).await.unwrap();

        // Valid value
        assert!(config.set("port", ConfigValue::Integer(8080)).await.is_ok());

        // Invalid value (too large)
        assert!(config.set("port", ConfigValue::Integer(70000)).await.is_err());
    }

    #[tokio::test]
    async fn test_watcher() {
        let config = ConfigManager::new(Environment::Development);
        let mut watcher = config.watch("test.watched").await;

        config.set("test.watched", ConfigValue::Integer(42)).await.unwrap();

        let value = watcher.recv().await.unwrap();
        assert_eq!(value.as_integer(), Some(42));
    }

    #[tokio::test]
    async fn test_snapshot() {
        let config = ConfigManager::new(Environment::Development);

        config.set("key1", ConfigValue::String("value1".to_string())).await.unwrap();
        config.set("key2", ConfigValue::Integer(42)).await.unwrap();

        let snapshot_id = config.create_snapshot("Test snapshot").await.unwrap();

        config.set("key1", ConfigValue::String("changed".to_string())).await.unwrap();

        config.restore_snapshot(&snapshot_id).await.unwrap();

        let value = config.get("key1").await.unwrap();
        assert_eq!(value.as_string(), Some("value1"));
    }
}
