// # Collection Management
//
// Oracle SODA-like collection management with schema validation, statistics, and metadata.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;
use super::document::{Document, DocumentId, IdGenerationType};

// JSON Schema for document validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    // Schema title
    pub title: Option<String>,
    // Schema description
    pub description: Option<String>,
    // Schema type
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    // Required properties
    pub required: Vec<String>,
    // Property definitions
    pub properties: HashMap<String, PropertySchema>,
    // Additional properties allowed
    #[serde(rename = "additionalProperties")]
    pub additional_properties: bool,
    // Minimum properties
    #[serde(rename = "minProperties")]
    pub min_properties: Option<usize>,
    // Maximum properties
    #[serde(rename = "maxProperties")]
    pub max_properties: Option<usize>,
}

impl JsonSchema {
    // Create a new empty schema
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            schema_type: Some("object".to_string()),
            required: Vec::new(),
            properties: HashMap::new(),
            additional_properties: true,
            min_properties: None,
            max_properties: None,
        }
    }

    // Validate a JSON document against this schema
    pub fn validate(&self, doc: &serde_json::Value) -> Result<()> {
        // Check if document is an object
        if self.schema_type.as_deref() == Some("object") {
            if !doc.is_object() {
                return Err(crate::error::DbError::InvalidInput(
                    "Document must be an object".to_string()
                ));
            }
        }

        let obj = doc.as_object().ok_or_else(|| {
            crate::error::DbError::InvalidInput("Document must be an object".to_string())
        })?;

        // Check required properties
        for required_prop in &self.required {
            if !obj.contains_key(required_prop) {
                return Err(crate::error::DbError::InvalidInput(
                    format!("Required property '{}' is missing", required_prop)
                ));
            }
        }

        // Check property count
        if let Some(min) = self.min_properties {
            if obj.len() < min {
                return Err(crate::error::DbError::InvalidInput(
                    format!("Document has {} properties, minimum is {}", obj.len(), min)
                ));
            }
        }

        if let Some(max) = self.max_properties {
            if obj.len() > max {
                return Err(crate::error::DbError::InvalidInput(
                    format!("Document has {} properties, maximum is {}", obj.len(), max)
                ));
            }
        }

        // Validate each property
        for (key, value) in obj {
            if let Some(prop_schema) = self.properties.get(key) {
                prop_schema.validate(value)?;
            } else if !self.additional_properties {
                return Err(crate::error::DbError::InvalidInput(
                    format!("Additional property '{}' is not allowed", key)
                ));
            }
        }

        Ok(())
    }

    // Add a required property
    pub fn add_required(&mut self, property: impl Into<String>) {
        self.required.push(property.into());
    }

    // Add a property schema
    pub fn add_property(&mut self, name: impl Into<String>, schema: PropertySchema) {
        self.properties.insert(name.into(), schema);
    }
}

impl Default for JsonSchema {
    fn default() -> Self {
        Self::new()
    }
}

// Property schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    // Property type
    #[serde(rename = "type")]
    pub property_type: String,
    // Property description
    pub description: Option<String>,
    // Minimum value (for numbers)
    pub minimum: Option<f64>,
    // Maximum value (for numbers)
    pub maximum: Option<f64>,
    // Minimum length (for strings)
    #[serde(rename = "minLength")]
    pub min_length: Option<usize>,
    // Maximum length (for strings)
    #[serde(rename = "maxLength")]
    pub max_length: Option<usize>,
    // Pattern (regex for strings)
    pub pattern: Option<String>,
    // Enum values
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    // Format (e.g., "email", "date-time")
    pub format: Option<String>,
}

impl PropertySchema {
    // Create a string property schema
    pub fn string() -> Self {
        Self {
            property_type: "string".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Create a number property schema
    pub fn number() -> Self {
        Self {
            property_type: "number".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Create an integer property schema
    pub fn integer() -> Self {
        Self {
            property_type: "integer".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Create a boolean property schema
    pub fn boolean() -> Self {
        Self {
            property_type: "boolean".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Create an array property schema
    pub fn array() -> Self {
        Self {
            property_type: "array".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Create an object property schema
    pub fn object() -> Self {
        Self {
            property_type: "object".to_string(),
            description: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            format: None,
        }
    }

    // Validate a value against this property schema
    pub fn validate(&self, value: &serde_json::Value) -> Result<()> {
        // Type validation
        match self.property_type.as_str() {
            "string" => {
                if !value.is_string() {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected string, got {:?}", value)
                    ));
                }
                let s = value.as_str().unwrap();

                // Min/max length
                if let Some(min) = self.min_length {
                    if s.len() < min {
                        return Err(crate::error::DbError::InvalidInput(
                            format!("String length {} is less than minimum {}", s.len(), min)
                        ));
                    }
                }
                if let Some(max) = self.max_length {
                    if s.len() > max {
                        return Err(crate::error::DbError::InvalidInput(
                            format!("String length {} exceeds maximum {}", s.len(), max)
                        ));
                    }
                }

                // Pattern matching
                if let Some(pattern) = &self.pattern {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| crate::error::DbError::InvalidInput(
                            format!("Invalid regex pattern: {}", e)
                        ))?;
                    if !re.is_match(s) {
                        return Err(crate::error::DbError::InvalidInput(
                            format!("String '{}' does not match pattern '{}'", s, pattern)
                        ));
                    }
                }
            }
            "number" | "integer" => {
                if !value.is_number() {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected number, got {:?}", value)
                    ));
                }
                let num = value.as_f64().unwrap();

                if self.property_type == "integer" && num.fract() != 0.0 {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected integer, got {}", num)
                    ));
                }

                // Min/max value
                if let Some(min) = self.minimum {
                    if num < min {
                        return Err(crate::error::DbError::InvalidInput(
                            format!("Value {} is less than minimum {}", num, min)
                        ));
                    }
                }
                if let Some(max) = self.maximum {
                    if num > max {
                        return Err(crate::error::DbError::InvalidInput(
                            format!("Value {} exceeds maximum {}", num, max)
                        ));
                    }
                }
            }
            "boolean" => {
                if !value.is_boolean() {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected boolean, got {:?}", value)
                    ));
                }
            }
            "array" => {
                if !value.is_array() {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected array, got {:?}", value)
                    ));
                }
            }
            "object" => {
                if !value.is_object() {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Expected object, got {:?}", value)
                    ));
                }
            }
            _ => {}
        }

        // Enum validation
        if let Some(enum_values) = &self.enum_values {
            if !enum_values.contains(value) {
                return Err(crate::error::DbError::InvalidInput(
                    format!("Value {:?} is not in allowed enum values", value)
                ));
            }
        }

        Ok(())
    }

    // Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    // Set minimum value
    pub fn minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    // Set maximum value
    pub fn maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }

    // Set minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    // Set maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    // Set pattern
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }
}

// Collection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionStats {
    // Total number of documents
    pub document_count: u64,
    // Total size in bytes
    pub total_size: u64,
    // Average document size
    pub avg_document_size: u64,
    // Number of indexes
    pub index_count: usize,
    // Total index size
    pub index_size: u64,
    // Last update time
    pub last_updated: u64,
    // Document count by version
    pub version_distribution: HashMap<u64, u64>,
}

impl CollectionStats {
    // Create new statistics
    pub fn new() -> Self {
        Self {
            document_count: 0,
            total_size: 0,
            avg_document_size: 0,
            index_count: 0,
            index_size: 0,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version_distribution: HashMap::new(),
        }
    }

    // Update statistics after document insertion
    pub fn on_insert(&mut self, doc_size: u64, version: u64) {
        self.document_count += 1;
        self.total_size += doc_size;
        self.avg_document_size = self.total_size / self.document_count;
        *self.version_distribution.entry(version).or_insert(0) += 1;
        self.update_timestamp();
    }

    // Update statistics after document deletion
    pub fn on_delete(&mut self, doc_size: u64, version: u64) {
        if self.document_count > 0 {
            self.document_count -= 1;
            self.total_size = self.total_size.saturating_sub(doc_size);
            if self.document_count > 0 {
                self.avg_document_size = self.total_size / self.document_count;
            } else {
                self.avg_document_size = 0;
            }
            if let Some(count) = self.version_distribution.get_mut(&version) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.version_distribution.remove(&version);
                }
            }
            self.update_timestamp();
        }
    }

    // Update statistics after document update
    pub fn on_update(&mut self, old_size: u64, new_size: u64, oldversion: u64, newversion: u64) {
        self.total_size = self.total_size.saturating_sub(old_size) + new_size;
        if self.document_count > 0 {
            self.avg_document_size = self.total_size / self.document_count;
        }

        // Update version distribution
        if let Some(count) = self.version_distribution.get_mut(&oldversion) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.version_distribution.remove(&oldversion);
            }
        }
        *self.version_distribution.entry(newversion).or_insert(0) += 1;

        self.update_timestamp();
    }

    fn update_timestamp(&mut self) {
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

impl Default for CollectionStats {
    fn default() -> Self {
        Self::new()
    }
}

// Collection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSettings {
    // ID generation strategy
    pub id_generation: IdGenerationType,
    // Enable versioning
    pub versioning_enabled: bool,
    // Maximum document size in bytes
    pub max_document_size: usize,
    // Enable compression
    pub compression_enabled: bool,
    // Default TTL in seconds
    pub default_ttl: Option<u64>,
    // Enable schema validation
    pub schema_validation_enabled: bool,
    // Validation action on failure
    pub validation_action: ValidationAction,
    // Case sensitivity for queries
    pub case_sensitive: bool,
    // Enable audit logging
    pub audit_enabled: bool,
}

impl CollectionSettings {
    // Create default settings
    pub fn new() -> Self {
        Self {
            id_generation: IdGenerationType::Uuid,
            versioning_enabled: true,
            max_document_size: 16 * 1024 * 1024, // 16 MB
            compression_enabled: false,
            default_ttl: None,
            schema_validation_enabled: false,
            validation_action: ValidationAction::Error,
            case_sensitive: false,
            audit_enabled: false,
        }
    }
}

impl Default for CollectionSettings {
    fn default() -> Self {
        Self::new()
    }
}

// Validation action when schema validation fails
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationAction {
    // Reject the operation with an error
    Error,
    // Allow the operation but log a warning
    Warn,
}

// Collection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    // Collection name
    pub name: String,
    // Collection description
    pub description: Option<String>,
    // Creation timestamp
    pub created_at: u64,
    // Last modification timestamp
    pub updated_at: u64,
    // Collection settings
    pub settings: CollectionSettings,
    // JSON schema for validation
    pub schema: Option<JsonSchema>,
    // Collection statistics
    pub stats: CollectionStats,
    // Custom metadata
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl CollectionMetadata {
    // Create new collection metadata
    pub fn new(name: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            name,
            description: None,
            created_at: now,
            updated_at: now,
            settings: CollectionSettings::new(),
            schema: None,
            stats: CollectionStats::new(),
            custom_fields: HashMap::new(),
        }
    }

    // Set schema
    pub fn set_schema(&mut self, schema: JsonSchema) {
        self.schema = Some(schema);
        self.settings.schema_validation_enabled = true;
        self.update_timestamp();
    }

    // Remove schema
    pub fn remove_schema(&mut self) {
        self.schema = None;
        self.settings.schema_validation_enabled = false;
        self.update_timestamp();
    }

    fn update_timestamp(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

// Collection of documents
#[derive(Clone)]
pub struct Collection {
    // Collection metadata
    pub metadata: CollectionMetadata,
    // Documents in the collection (in-memory storage for now)
    documents: Arc<RwLock<HashMap<DocumentId, Document>>>,
    // Auto-increment counter
    auto_increment_counter: Arc<RwLock<u64>>,
}

impl Collection {
    // Create a new collection
    pub fn new(name: String) -> Self {
        Self {
            metadata: CollectionMetadata::new(name),
            documents: Arc::new(RwLock::new(HashMap::new())),
            auto_increment_counter: Arc::new(RwLock::new(0)),
        }
    }

    // Create a collection with settings
    pub fn with_settings(name: String, settings: CollectionSettings) -> Self {
        let mut metadata = CollectionMetadata::new(name);
        metadata.settings = settings;

        Self {
            metadata,
            documents: Arc::new(RwLock::new(HashMap::new())),
            auto_increment_counter: Arc::new(RwLock::new(0)),
        }
    }

    // Insert a document into the collection
    pub fn insert(&mut self, mut document: Document) -> Result<DocumentId> {
        // Validate against schema if enabled
        if self.metadata.settings.schema_validation_enabled {
            if let Some(schema) = &self.metadata.schema {
                let json = document.as_json()?;
                if let Err(e) = schema.validate(&json) {
                    match self.metadata.settings.validation_action {
                        ValidationAction::Error => return Err(e),
                        ValidationAction::Warn => {
                            eprintln!("Schema validation warning: {}", e);
                        }
                    }
                }
            }
        }

        // Check document size
        if document.metadata.size > self.metadata.settings.max_document_size {
            return Err(crate::error::DbError::InvalidInput(
                format!("Document size {} exceeds maximum {}",
                    document.metadata.size,
                    self.metadata.settings.max_document_size)
            ));
        }

        // Apply default TTL if not set
        if document.metadata.ttl.is_none() {
            if let Some(default_ttl) = self.metadata.settings.default_ttl {
                document.metadata.set_ttl(default_ttl);
            }
        }

        let doc_id = document.metadata.id.clone();
        let doc_size = document.metadata.size as u64;
        let doc_version = document.metadata.version.version;

        // Insert document
        let mut docs = self.documents.write().unwrap();
        docs.insert(doc_id.clone(), document);

        // Update statistics
        self.metadata.stats.on_insert(doc_size, doc_version);

        Ok(doc_id)
    }

    // Get a document by ID
    pub fn get(&self, id: &DocumentId) -> Option<Document> {
        let docs = self.documents.read().unwrap();
        docs.get(id)
            .cloned()
    }

    // Update a document
    pub fn update(&mut self, id: &DocumentId, document: Document) -> Result<()> {
        // Validate against schema if enabled
        if self.metadata.settings.schema_validation_enabled {
            if let Some(schema) = &self.metadata.schema {
                let json = document.as_json()?;
                if let Err(e) = schema.validate(&json) {
                    match self.metadata.settings.validation_action {
                        ValidationAction::Error => return Err(e),
                        ValidationAction::Warn => {
                            eprintln!("Schema validation warning: {}", e);
                        }
                    }
                }
            }
        }

        let mut docs = self.documents.write().unwrap();
        if let Some(old_doc) = docs.get(id) {
            let old_size = old_doc.metadata.size as u64;
            let old_version = old_doc.metadata.version.version;
            let new_size = document.metadata.size as u64;
            let new_version = document.metadata.version.version;

            docs.insert(id.clone(), document);

            // Update statistics
            self.metadata.stats.on_update(old_size, new_size, old_version, new_version);

            Ok(())
        } else {
            Err(crate::error::DbError::NotFound(
                format!("Document with ID {:?} not found", id)
            ))
        }
    }

    // Delete a document
    pub fn delete(&mut self, id: &DocumentId) -> Result<()> {
        let mut docs = self.documents.write().unwrap();
        if let Some(doc) = docs.remove(id) {
            let doc_size = doc.metadata.size as u64;
            let doc_version = doc.metadata.version.version;

            // Update statistics
            self.metadata.stats.on_delete(doc_size, doc_version);

            Ok(())
        } else {
            Err(crate::error::DbError::NotFound(
                format!("Document with ID {:?} not found", id)
            ))
        }
    }

    // Count documents in collection
    pub fn count(&self) -> usize {
        self.documents.read().unwrap().len()
    }

    // Get all document IDs
    pub fn get_all_ids(&self) -> Vec<DocumentId> {
        self.documents.read().unwrap().keys().cloned().collect()
    }

    // Generate next auto-increment ID
    pub fn next_auto_increment_id(&self) -> u64 {
        let mut counter = self.auto_increment_counter.write().unwrap();
        *counter += 1;
        *counter
    }

    // Clear all documents from the collection
    pub fn clear(&mut self) {
        self.documents.write().unwrap().clear();
        self.metadata.stats = CollectionStats::new();
    }

    // Remove expired documents (TTL cleanup)
    pub fn cleanup_expired(&mut self) -> usize {
        let mut docs = self.documents.write().unwrap();
        let mut expired_ids = Vec::new();

        for (id, doc) in docs.iter() {
            if doc.metadata.is_expired() {
                expired_ids.push(id.clone());
            }
        }

        let count = expired_ids.len();
        for id in expired_ids {
            if let Some(doc) = docs.remove(&id) {
                let doc_size = doc.metadata.size as u64;
                let doc_version = doc.metadata.version.version;
                self.metadata.stats.on_delete(doc_size, doc_version);
            }
        }

        count
    }
}

// Collection manager for managing multiple collections
pub struct CollectionManager {
    // Collections indexed by name
    collections: Arc<RwLock<BTreeMap<String, Collection>>>,
}

impl CollectionManager {
    // Create a new collection manager
    pub fn new() -> Self {
        Self {
            collections: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    // Create a new collection
    pub fn create_collection(&self, name: String) -> Result<()> {
        let mut collections = self.collections.write().unwrap();

        if collections.contains_key(&name) {
            return Err(crate::error::DbError::AlreadyExists(
                format!("Collection '{}' already exists", name)
            ));
        }

        collections.insert(name.clone(), Collection::new(name));
        Ok(())
    }

    // Create a collection with settings
    pub fn create_collection_with_settings(
        &self,
        name: String,
        settings: CollectionSettings,
    ) -> Result<()> {
        let mut collections = self.collections.write().unwrap();

        if collections.contains_key(&name) {
            return Err(crate::error::DbError::AlreadyExists(
                format!("Collection '{}' already exists", name)
            ));
        }

        collections.insert(name.clone(), Collection::with_settings(name, settings));
        Ok(())
    }

    // Drop a collection
    pub fn drop_collection(&self, name: &str) -> Result<()> {
        let mut collections = self.collections.write().unwrap();

        if collections.remove(name).is_some() {
            Ok(())
        } else {
            Err(crate::error::DbError::NotFound(
                format!("Collection '{}' not found", name)
            ))
        }
    }

    // Get a collection
    pub fn get_collection(&self, name: &str) -> Result<Collection> {
        let collections = self.collections.read().unwrap();
        collections.get(name)
            .cloned()
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", name)
            ))
    }

    // Check if collection exists
    pub fn collection_exists(&self, name: &str) -> bool {
        self.collections.read().unwrap().contains_key(name)
    }

    // List all collection names
    pub fn list_collections(&self) -> Vec<String> {
        self.collections.read().unwrap().keys().cloned().collect()
    }

    // Get collection count
    pub fn collection_count(&self) -> usize {
        self.collections.read().unwrap().len()
    }

    // Rename a collection
    pub fn rename_collection(&self, old_name: &str, new_name: String) -> Result<()> {
        let mut collections = self.collections.write().unwrap();

        if collections.contains_key(&new_name) {
            return Err(crate::error::DbError::AlreadyExists(
                format!("Collection '{}' already exists", new_name)
            ));
        }

        if let Some(mut collection) = collections.remove(old_name) {
            collection.metadata.name = new_name.clone();
            collections.insert(new_name, collection);
            Ok(())
        } else {
            Err(crate::error::DbError::NotFound(
                format!("Collection '{}' not found", old_name)
            ))
        }
    }
}

impl Default for CollectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_schema_validation() {
        let mut schema = JsonSchema::new();
        schema.add_required("name");
        schema.add_required("age");
        schema.add_property("name", PropertySchema::string().min_length(1));
        schema.add_property("age", PropertySchema::integer().minimum(0.0).maximum(150.0));

        // Valid document
        let valid_doc = json!({
            "name": "Alice",
            "age": 30
        });
        assert!(schema.validate(&valid_doc).is_ok());

        // Missing required field
        let invalid_doc = json!({
            "name": "Bob"
        });
        assert!(schema.validate(&invalid_doc).is_err());

        // Invalid age
        let invalid_doc = json!({
            "name": "Charlie",
            "age": 200
        });
        assert!(schema.validate(&invalid_doc).is_err());
    }

    #[test]
    fn test_collection_operations() {
        let mut collection = Collection::new("users".to_string());

        let doc = Document::from_json(
            DocumentId::new_uuid(),
            "users".to_string(),
            json!({"name": "Alice"}),
        ).unwrap();

        let id = collection.insert(doc).unwrap();
        assert_eq!(collection.count(), 1);

        let retrieved = collection.get(&id).unwrap();
        assert_eq!(retrieved.metadata.id, id);

        collection.delete(&id).unwrap();
        assert_eq!(collection.count(), 0);
    }

    #[test]
    fn test_collection_manager() {
        let manager = CollectionManager::new();

        manager.create_collection("users".to_string()).unwrap();
        assert!(manager.collection_exists("users"));

        assert_eq!(manager.list_collections(), vec!["users".to_string()]);

        manager.drop_collection("users").unwrap();
        assert!(!manager.collection_exists("users"));
    }
}
