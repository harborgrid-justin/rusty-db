// # Change Streams
//
// Real-time document change notifications with change stream cursors,
// resume tokens, filtered streams, and document diff generation.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime};
use uuid::Uuid;
use crate::error::Result;
use super::document::{Document, DocumentId};

/// Change event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeEventType {
    /// Document inserted
    Insert,
    /// Document updated
    Update,
    /// Document deleted
    Delete,
    /// Document replaced
    Replace,
    /// Collection dropped
    Drop,
    /// Collection renamed
    Rename,
    /// Database dropped
    DropDatabase,
    /// Invalidate event (collection/database dropped)
    Invalidate,
}

/// Change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub operation_type: ChangeEventType,
    /// Cluster timestamp (for ordering)
    pub cluster_time: u64,
    /// Collection name
    pub collection: String,
    /// Document ID (for insert/update/delete/replace)
    pub document_key: Option<DocumentId>,
    /// Full document (for insert/replace, optional for update)
    pub full_document: Option<Value>,
    /// Update description (for update operations)
    pub update_description: Option<UpdateDescription>,
    /// Namespace information
    pub namespace: Namespace,
    /// Resume token for resuming the stream
    pub resume_token: ResumeToken,
}

impl ChangeEvent {
    /// Create a new change event
    pub fn new(
        operation_type: ChangeEventType,
        collection: String,
        document_key: Option<DocumentId>,
    ) -> Self {
        let cluster_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let resume_token = ResumeToken::new(cluster_time);

        Self {
            id: Uuid::new_v4().to_string(),
            operation_type,
            cluster_time,
            collection: collection.clone(),
            document_key,
            full_document: None,
            update_description: None,
            namespace: Namespace {
                database: "rustydb".to_string(),
                collection,
            },
            resume_token,
        }
    }

    /// Create an insert event
    pub fn insert(collection: String, document: &Document) -> Result<Self> {
        let mut event = Self::new(
            ChangeEventType::Insert,
            collection,
            Some(document.metadata.id.clone()),
        );
        event.full_document = Some(document.as_json()?);
        Ok(event)
    }

    /// Create an update event
    pub fn update(
        collection: String,
        document_id: DocumentId,
        old_doc: &Document,
        new_doc: &Document,
    ) -> Result<Self> {
        let mut event = Self::new(
            ChangeEventType::Update,
            collection,
            Some(document_id),
        );

        let update_desc = UpdateDescription::generate(old_doc, new_doc)?;
        event.update_description = Some(update_desc);
        event.full_document = Some(new_doc.as_json()?);

        Ok(event)
    }

    /// Create a delete event
    pub fn delete(collection: String, document_id: DocumentId) -> Self {
        Self::new(ChangeEventType::Delete, collection, Some(document_id))
    }

    /// Create a replace event
    pub fn replace(collection: String, document: &Document) -> Result<Self> {
        let mut event = Self::new(
            ChangeEventType::Replace,
            collection,
            Some(document.metadata.id.clone()),
        );
        event.full_document = Some(document.as_json()?);
        Ok(event)
    }
}

/// Namespace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    /// Database name
    pub database: String,
    /// Collection name
    pub collection: String,
}

/// Resume token for resuming change streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeToken {
    /// Timestamp component
    pub timestamp: u64,
    /// Random component for uniqueness
    pub random: String,
}

impl ResumeToken {
    /// Create a new resume token
    pub fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            random: Uuid::new_v4().to_string(),
        }
    }

    /// Encode token to string
    pub fn encode(&self) -> String {
        format!("{}:{}", self.timestamp, self.random)
    }

    /// Decode token from string
    pub fn decode(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(crate::error::DbError::InvalidInput(
                "Invalid resume token format".to_string()
            ));
        }

        let timestamp = parts[0].parse::<u64>()
            .map_err(|_| crate::error::DbError::InvalidInput(
                "Invalid timestamp in resume token".to_string()
            ))?;

        Ok(Self {
            timestamp,
            random: parts[1].to_string(),
        })
    }
}

/// Update description for update events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDescription {
    /// Updated fields
    pub updated_fields: HashMap<String, Value>,
    /// Removed fields
    pub removed_fields: Vec<String>,
    /// Truncated arrays (field -> new size)
    pub truncated_arrays: HashMap<String, usize>,
}

impl UpdateDescription {
    /// Generate update description by comparing two documents
    pub fn generate(old_doc: &Document, new_doc: &Document) -> Result<Self> {
        let old_json = old_doc.as_json()?;
        let new_json = new_doc.as_json()?;

        let mut updated_fields = HashMap::new();
        let mut removed_fields = Vec::new();
        let truncated_arrays = HashMap::new();

        if let (Value::Object(old_obj), Value::Object(new_obj)) = (&old_json, &new_json) {
            // Find updated and removed fields
            for (key, old_value) in old_obj {
                if let Some(new_value) = new_obj.get(key) {
                    if old_value != new_value {
                        updated_fields.insert(key.clone(), new_value.clone());
                    }
                } else {
                    removed_fields.push(key.clone());
                }
            }

            // Find new fields
            for (key, new_value) in new_obj {
                if !old_obj.contains_key(key) {
                    updated_fields.insert(key.clone(), new_value.clone());
                }
            }
        }

        Ok(Self {
            updated_fields,
            removed_fields,
            truncated_arrays,
        })
    }

    /// Check if this is an empty update
    pub fn is_empty(&self) -> bool {
        self.updated_fields.is_empty()
            && self.removed_fields.is_empty()
            && self.truncated_arrays.is_empty()
    }
}

/// Change stream filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeStreamFilter {
    /// Filter by operation types
    pub operation_types: Option<Vec<ChangeEventType>>,
    /// Filter by collection names
    pub collections: Option<Vec<String>>,
    /// Filter by document IDs
    pub document_ids: Option<Vec<DocumentId>>,
    /// Custom filter function (JSONPath-like)
    pub custom_filter: Option<String>,
}

impl ChangeStreamFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            operation_types: None,
            collections: None,
            document_ids: None,
            custom_filter: None,
        }
    }

    /// Filter by operation types
    pub fn operation_types(mut self, types: Vec<ChangeEventType>) -> Self {
        self.operation_types = Some(types);
        self
    }

    /// Filter by collections
    pub fn collections(mut self, collections: Vec<String>) -> Self {
        self.collections = Some(collections);
        self
    }

    /// Filter by document IDs
    pub fn document_ids(mut self, ids: Vec<DocumentId>) -> Self {
        self.document_ids = Some(ids);
        self
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &ChangeEvent) -> bool {
        // Check operation type
        if let Some(ref types) = self.operation_types {
            if !types.contains(&event.operation_type) {
                return false;
            }
        }

        // Check collection
        if let Some(ref collections) = self.collections {
            if !collections.contains(&event.collection) {
                return false;
            }
        }

        // Check document ID
        if let Some(ref ids) = self.document_ids {
            if let Some(ref doc_key) = event.document_key {
                if !ids.contains(doc_key) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for ChangeStreamFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Change stream cursor for iterating through events
#[derive(Clone)]
pub struct ChangeStreamCursor {
    /// Stream ID
    pub id: String,
    /// Current position in the stream
    position: usize,
    /// Events buffer
    events: Arc<RwLock<VecDeque<ChangeEvent>>>,
    /// Filter for events
    filter: ChangeStreamFilter,
    /// Resume token to start from
    resume_after: Option<ResumeToken>,
    /// Maximum batch size
    batch_size: usize,
}

impl ChangeStreamCursor {
    /// Create a new cursor
    pub fn new(
        events: Arc<RwLock<VecDeque<ChangeEvent>>>,
        filter: ChangeStreamFilter,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            position: 0,
            events,
            filter,
            resume_after: None,
            batch_size: 100,
        }
    }

    /// Resume from a token
    pub fn resume_after(mut self, token: ResumeToken) -> Self {
        self.resume_after = Some(token);
        self
    }

    /// Set batch size
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Get next batch of events
    pub fn next_batch(&mut self) -> Vec<ChangeEvent> {
        let events = self.events.read().unwrap();
        let mut batch = Vec::new();

        // Find starting position if resuming
        let start_pos = if let Some(ref token) = self.resume_after {
            events
                .iter()
                .position(|e| e.resume_token.timestamp > token.timestamp)
                .unwrap_or(0)
        } else {
            self.position
        };

        self.resume_after = None; // Clear resume token after first use

        // Collect matching events
        for event in events.iter().skip(start_pos) {
            if self.filter.matches(event) {
                batch.push(event.clone());
                if batch.len() >= self.batch_size {
                    break;
                }
            }
        }

        self.position = start_pos + batch.len();
        batch
    }

    /// Check if there are more events
    pub fn has_more(&self) -> bool {
        let events = self.events.read().unwrap();
        self.position < events.len()
    }

    /// Get the last resume token
    pub fn get_resume_token(&self) -> Option<ResumeToken> {
        let events = self.events.read().unwrap();
        if self.position > 0 && self.position <= events.len() {
            events.get(self.position - 1).map(|e| e.resume_token.clone())
        } else {
            None
        }
    }
}

/// Change stream manager
pub struct ChangeStreamManager {
    /// All change events (ring buffer)
    events: Arc<RwLock<VecDeque<ChangeEvent>>>,
    /// Maximum number of events to keep
    max_events: usize,
    /// Active cursors
    cursors: Arc<RwLock<HashMap<String, ChangeStreamCursor>>>,
}

impl ChangeStreamManager {
    /// Create a new change stream manager
    pub fn new(maxevents: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(max_events))),
            max_events,
            cursors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a change event
    pub fn add_event(&self, event: ChangeEvent) {
        let mut events = self.events.write().unwrap();

        // Add event
        events.push_back(event);

        // Trim if exceeds max size
        while events.len() > self.max_events {
            events.pop_front();
        }
    }

    /// Create a new change stream cursor
    pub fn watch(&self, filter: ChangeStreamFilter) -> ChangeStreamCursor {
        let cursor = ChangeStreamCursor::new(Arc::clone(&self.events), filter);
        let cursor_id = cursor.id.clone();

        let mut cursors = self.cursors.write().unwrap();
        cursors.insert(cursor_id, cursor.clone());

        cursor
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Clear all events
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }

    /// Get events in time range
    pub fn get_events_range(&self, start_time: u64, end_time: u64) -> Vec<ChangeEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.cluster_time >= start_time && e.cluster_time <= end_time)
            .cloned()
            .collect()
    }
}

impl Default for ChangeStreamManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Document diff generator
pub struct DiffGenerator;

impl DiffGenerator {
    /// Generate a diff between two JSON values
    pub fn diff(old: &Value, new: &Value) -> Diff {
        let mut operations = Vec::new();
        Self::diff_recursive("".to_string(), old, new, &mut operations);

        Diff { operations }
    }

    fn diff_recursive(path: String, old: &Value, new: &Value, operations: &mut Vec<DiffOperation>) {
        match (old, new) {
            (Value::Object(old_obj), Value::Object(new_obj)) => {
                // Check for modified and removed fields
                for (key, old_value) in old_obj {
                    let field_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(new_value) = new_obj.get(key) {
                        if old_value != new_value {
                            Self::diff_recursive(field_path, old_value, new_value, operations);
                        }
                    } else {
                        operations.push(DiffOperation::Remove { path: field_path });
                    }
                }

                // Check for added fields
                for (key, new_value) in new_obj {
                    if !old_obj.contains_key(key) {
                        let field_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        operations.push(DiffOperation::Add {
                            path: field_path,
                            value: new_value.clone(),
                        });
                    }
                }
            }
            (Value::Array(old_arr), Value::Array(new_arr)) => {
                // Simple array diff (could be more sophisticated)
                if old_arr != new_arr {
                    operations.push(DiffOperation::Replace {
                        path,
                        old_value: Value::Array(old_arr.clone()),
                        new_value: Value::Array(new_arr.clone()),
                    });
                }
            }
            _ => {
                if old != new {
                    operations.push(DiffOperation::Replace {
                        path,
                        old_value: old.clone(),
                        new_value: new.clone(),
                    });
                }
            }
        }
    }

    /// Apply a diff to a JSON value
    pub fn apply(value: &mut Value, diff: &Diff) -> Result<()> {
        for operation in &diff.operations {
            operation.apply(value)?;
        }
        Ok(())
    }
}

/// Document diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    /// Diff operations
    pub operations: Vec<DiffOperation>,
}

impl Diff {
    /// Check if diff is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Get operation count
    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

/// Diff operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum DiffOperation {
    /// Add a field
    #[serde(rename = "add")]
    Add { path: String, value: Value },

    /// Remove a field
    #[serde(rename = "remove")]
    Remove { path: String },

    /// Replace a value
    #[serde(rename = "replace")]
    Replace {
        path: String,
        old_value: Value,
        new_value: Value,
    },
}

impl DiffOperation {
    /// Apply this operation to a JSON value
    pub fn apply(&self, value: &mut Value) -> Result<()> {
        match self {
            DiffOperation::Add { path, value: new_value } => {
                Self::set_value_at_path(value, path, new_value.clone())
            }
            DiffOperation::Remove { path } => {
                Self::remove_value_at_path(value, path)
            }
            DiffOperation::Replace { path, new_value, .. } => {
                Self::set_value_at_path(value, path, new_value.clone())
            }
        }
    }

    fn set_value_at_path(root: &mut Value, path: &str, new_value: Value) -> Result<()> {
        if path.is_empty() {
            *root = new_value;
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part, set the value
                if let Value::Object(obj) = current {
                    obj.insert(part.to_string(), new_value);
                    return Ok(());
                }
            } else {
                // Navigate to the next level
                if let Value::Object(obj) = current {
                    current = obj.entry(part.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                } else {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Cannot navigate path: {}", path)
                    ));
                }
            }
        }

        Ok(())
    }

    fn remove_value_at_path(root: &mut Value, path: &str) -> Result<()> {
        if path.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part, remove the value
                if let Value::Object(obj) = current {
                    obj.remove(*part);
                    return Ok(());
                }
            } else {
                // Navigate to the next level
                if let Value::Object(obj) = current {
                    current = obj.get_mut(*part)
                        .ok_or_else(|| crate::error::DbError::NotFound(
                            format!("Path not found: {}", path)
                        ))?;
                } else {
                    return Err(crate::error::DbError::InvalidInput(
                        format!("Cannot navigate path: {}", path)
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_change_event_creation() {
        let doc = Document::from_json(
            DocumentId::new_custom("1"),
            "users".to_string(),
            json!({"name": "Alice"}),
        ).unwrap();

        let event = ChangeEvent::insert("users".to_string(), &doc).unwrap();

        assert_eq!(event.operation_type, ChangeEventType::Insert);
        assert_eq!(event.collection, "users");
    }

    #[test]
    fn test_resume_token() {
        let token = ResumeToken::new(12345);
        let encoded = token.encode();
        let decoded = ResumeToken::decode(&encoded).unwrap();

        assert_eq!(token.timestamp, decoded.timestamp);
    }

    #[test]
    fn test_update_description() {
        let old_doc = Document::from_json(
            DocumentId::new_custom("1"),
            "users".to_string(),
            json!({"name": "Alice", "age": 30}),
        ).unwrap();

        let new_doc = Document::from_json(
            DocumentId::new_custom("1"),
            "users".to_string(),
            json!({"name": "Alice", "age": 31, "city": "NYC"}),
        ).unwrap();

        let desc = UpdateDescription::generate(&old_doc, &new_doc).unwrap();

        assert!(desc.updated_fields.contains_key("age"));
        assert!(desc.updated_fields.contains_key("city"));
    }

    #[test]
    fn test_diff_generator() {
        let old = json!({"name": "Alice", "age": 30});
        let new = json!({"name": "Alice", "age": 31, "city": "NYC"});

        let diff = DiffGenerator::diff(&old, &new);

        assert!(!diff.is_empty());
        assert!(diff.len() >= 2);
    }

    #[test]
    fn test_change_stream_filter() {
        let filter = ChangeStreamFilter::new()
            .operation_types(vec![ChangeEventType::Insert, ChangeEventType::Update]);

        let insert_event = ChangeEvent::new(
            ChangeEventType::Insert,
            "users".to_string(),
            Some(DocumentId::new_custom("1")),
        );

        let delete_event = ChangeEvent::new(
            ChangeEventType::Delete,
            "users".to_string(),
            Some(DocumentId::new_custom("2")),
        );

        assert!(filter.matches(&insert_event));
        assert!(!filter.matches(&delete_event));
    }
}
