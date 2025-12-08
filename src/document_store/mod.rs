//! # JSON Document Store Engine
//!
//! Oracle SODA-like document store with comprehensive JSON support, indexing,
//! aggregation, change streams, and SQL/JSON integration.
//!
//! ## Overview
//!
//! The Document Store Engine provides a complete NoSQL document database
//! implementation with the following features:
//!
//! - **Document Model**: JSON and BSON support with versioning and metadata
//! - **Collections**: Schema validation, statistics, and collection management
//! - **JSONPath**: Full JSONPath query support with filters and array slicing
//! - **Indexing**: B-tree, full-text, compound, partial, and TTL indexes
//! - **Query By Example**: MongoDB-like query syntax with operators
//! - **Aggregation**: Pipeline-based aggregation with multiple stages
//! - **Change Streams**: Real-time change notifications with resume tokens
//! - **SQL/JSON**: Oracle-like SQL/JSON functions and predicates
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rusty_db::document_store::{
//!     DocumentStore,
//!     document::{Document, DocumentId},
//!     collections::CollectionSettings,
//! };
//! use serde_json::json;
//!
//! # fn main() -> rusty_db::Result<()> {
//! // Create a document store
//! let mut store = DocumentStore::new();
//!
//! // Create a collection
//! store.create_collection("users".to_string())?;
//!
//! // Insert a document
//! let doc = Document::from_json(
//!     DocumentId::new_uuid(),
//!     "users".to_string(),
//!     json!({
//!         "name": "Alice",
//!         "age": 30,
//!         "email": "alice@example.com"
//!     }),
//! )?;
//!
//! let doc_id = store.insert("users", doc)?;
//!
//! // Query documents
//! let results = store.find("users", json!({"age": {"$gte": 25}}))?;
//!
//! println!("Found {} documents", results.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Organization
//!
//! - [`document`]: Document model with JSON/BSON support
//! - [`collections`]: Collection management and schema validation
//! - [`jsonpath`]: JSONPath expression engine
//! - [`indexing`]: Document indexing infrastructure
//! - [`qbe`]: Query By Example implementation
//! - [`aggregation`]: Aggregation pipeline
//! - [`changes`]: Change streams and notifications
//! - [`sql_json`]: SQL/JSON integration functions

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use crate::error::Result;

// Public module exports
pub mod document;
pub mod collections;
pub mod jsonpath;
pub mod indexing;
pub mod qbe;
pub mod aggregation;
pub mod changes;
pub mod sql_json;

// Re-export commonly used types
pub use document::{Document, DocumentId, DocumentContent, DocumentMetadata, IdGenerationType};
pub use collections::{Collection, CollectionManager, CollectionSettings, JsonSchema};
pub use jsonpath::{JsonPath, JsonPathEvaluator, query as jsonpath_query};
pub use indexing::{IndexManager, IndexDefinition, IndexType, IndexField};
pub use qbe::{QueryDocument, QueryBuilder, Projection};
pub use aggregation::{Pipeline, PipelineStage, PipelineBuilder};
pub use changes::{ChangeEvent, ChangeStreamManager, ChangeStreamFilter, ChangeEventType};
pub use sql_json::{SqlJsonFunctions, JsonTableColumn, JsonDataType};

/// Main document store interface
///
/// Provides a unified API for document storage, querying, and management.
pub struct DocumentStore {
    /// Collection manager
    collection_manager: CollectionManager,
    /// Index manager
    index_manager: IndexManager,
    /// Change stream manager
    change_stream_manager: ChangeStreamManager,
    /// Documents by collection (in-memory storage)
    collections: Arc<RwLock<HashMap<String, HashMap<DocumentId, Document>>>>,
}

impl DocumentStore {
    /// Create a new document store
    pub fn new() -> Self {
        Self {
            collection_manager: CollectionManager::new(),
            index_manager: IndexManager::new(),
            change_stream_manager: ChangeStreamManager::new(10000),
            collections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a collection
    pub fn create_collection(&mut self, name: String) -> Result<()> {
        self.collection_manager.create_collection(name.clone())?;
        self.collections.write().unwrap().insert(name, HashMap::new());
        Ok(())
    }

    /// Create a collection with settings
    pub fn create_collection_with_settings(
        &mut self,
        name: String,
        settings: CollectionSettings,
    ) -> Result<()> {
        self.collection_manager.create_collection_with_settings(name.clone(), settings)?;
        self.collections.write().unwrap().insert(name, HashMap::new());
        Ok(())
    }

    /// Drop a collection
    pub fn drop_collection(&mut self, name: &str) -> Result<()> {
        self.collection_manager.drop_collection(name)?;
        self.collections.write().unwrap().remove(name);

        // Emit change event
        let event = ChangeEvent::new(
            ChangeEventType::Drop,
            name.to_string(),
            None,
        );
        self.change_stream_manager.add_event(event);

        Ok(())
    }

    /// Insert a document
    pub fn insert(&mut self, collection: &str, document: Document) -> Result<DocumentId> {
        let doc_id = document.metadata.id.clone();

        // Get or create collection documents
        let mut collections = self.collections.write().unwrap();
        let docs = collections.entry(collection.to_string()).or_insert_with(HashMap::new);

        // Insert document
        docs.insert(doc_id.clone(), document.clone());

        // Emit change event
        let event = ChangeEvent::insert(collection.to_string(), &document)?;
        self.change_stream_manager.add_event(event);

        Ok(doc_id)
    }

    /// Find a document by ID
    pub fn find_by_id(&self, collection: &str, id: &DocumentId) -> Result<Document> {
        let collections = self.collections.read().unwrap();
        let docs = collections.get(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        docs.get(id)
            .cloned()
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Document {:?} not found", id)
            ))
    }

    /// Find documents by query
    pub fn find(&self, collection: &str, query: Value) -> Result<Vec<Document>> {
        let query_doc = QueryDocument::from_json(query)?;
        let collections = self.collections.read().unwrap();
        let docs = collections.get(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        let mut results = Vec::new();
        for doc in docs.values() {
            if query_doc.matches(doc)? {
                results.push(doc.clone());
            }
        }

        Ok(results)
    }

    /// Update a document
    pub fn update(&mut self, collection: &str, id: &DocumentId, document: Document) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        let docs = collections.get_mut(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        let old_doc = docs.get(id)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Document {:?} not found", id)
            ))?
            .clone();

        // Update document
        docs.insert(id.clone(), document.clone());

        // Emit change event
        let event = ChangeEvent::update(
            collection.to_string(),
            id.clone(),
            &old_doc,
            &document,
        )?;
        self.change_stream_manager.add_event(event);

        Ok(())
    }

    /// Delete a document
    pub fn delete(&mut self, collection: &str, id: &DocumentId) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        let docs = collections.get_mut(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        if docs.remove(id).is_some() {
            // Emit change event
            let event = ChangeEvent::delete(collection.to_string(), id.clone());
            self.change_stream_manager.add_event(event);

            Ok(())
        } else {
            Err(crate::error::DbError::NotFound(
                format!("Document {:?} not found", id)
            ))
        }
    }

    /// Count documents in a collection
    pub fn count(&self, collection: &str) -> Result<usize> {
        let collections = self.collections.read().unwrap();
        let docs = collections.get(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        Ok(docs.len())
    }

    /// Count documents matching a query
    pub fn count_query(&self, collection: &str, query: Value) -> Result<usize> {
        let results = self.find(collection, query)?;
        Ok(results.len())
    }

    /// Aggregate documents using pipeline
    pub fn aggregate(&self, collection: &str, pipeline: Pipeline) -> Result<Vec<Value>> {
        let collections = self.collections.read().unwrap();
        let docs = collections.get(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        pipeline.execute(docs)
    }

    /// Create an index
    pub fn create_index(&mut self, definition: IndexDefinition) -> Result<()> {
        self.index_manager.create_index(definition)
    }

    /// Drop an index
    pub fn drop_index(&mut self, name: &str) -> Result<()> {
        self.index_manager.drop_index(name)
    }

    /// List all indexes
    pub fn list_indexes(&self) -> Vec<String> {
        self.index_manager.list_indexes()
    }

    /// Watch for changes
    pub fn watch(&self, filter: ChangeStreamFilter) -> changes::ChangeStreamCursor {
        self.change_stream_manager.watch(filter)
    }

    /// Execute JSONPath query
    pub fn jsonpath_query(&self, collection: &str, path: &str) -> Result<Vec<Value>> {
        let collections = self.collections.read().unwrap();
        let docs = collections.get(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        let mut all_results = Vec::new();
        for doc in docs.values() {
            let json = doc.as_json()?;
            let results = jsonpath_query(path, &json)?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    /// Execute JSON_TABLE function
    pub fn json_table(
        &self,
        collection: &str,
        doc_id: &DocumentId,
        row_path: &str,
        columns: Vec<JsonTableColumn>,
    ) -> Result<sql_json::JsonTableResult> {
        let doc = self.find_by_id(collection, doc_id)?;
        let json = doc.as_json()?;
        SqlJsonFunctions::json_table(&json, row_path, columns)
    }

    /// Execute JSON_QUERY function
    pub fn json_query(
        &self,
        collection: &str,
        doc_id: &DocumentId,
        path: &str,
        wrapper: sql_json::JsonWrapper,
    ) -> Result<Option<Value>> {
        let doc = self.find_by_id(collection, doc_id)?;
        let json = doc.as_json()?;
        SqlJsonFunctions::json_query(&json, path, wrapper)
    }

    /// Execute JSON_VALUE function
    pub fn json_value(
        &self,
        collection: &str,
        doc_id: &DocumentId,
        path: &str,
        returning_type: JsonDataType,
    ) -> Result<Option<Value>> {
        let doc = self.find_by_id(collection, doc_id)?;
        let json = doc.as_json()?;
        SqlJsonFunctions::json_value(&json, path, returning_type)
    }

    /// Execute JSON_EXISTS function
    pub fn json_exists(
        &self,
        collection: &str,
        doc_id: &DocumentId,
        path: &str,
    ) -> Result<bool> {
        let doc = self.find_by_id(collection, doc_id)?;
        let json = doc.as_json()?;
        SqlJsonFunctions::json_exists(&json, path)
    }

    /// List all collections
    pub fn list_collections(&self) -> Vec<String> {
        self.collection_manager.list_collections()
    }

    /// Get collection statistics
    pub fn get_stats(&self, collection: &str) -> Result<collections::CollectionStats> {
        let coll = self.collection_manager.get_collection(collection)?;
        Ok(coll.metadata.stats)
    }

    /// Bulk insert documents
    pub fn bulk_insert(&mut self, collection: &str, documents: Vec<Document>) -> Result<Vec<DocumentId>> {
        let mut ids = Vec::new();

        for doc in documents {
            let id = self.insert(collection, doc)?;
            ids.push(id);
        }

        Ok(ids)
    }

    /// Bulk delete documents by query
    pub fn bulk_delete(&mut self, collection: &str, query: Value) -> Result<usize> {
        let docs_to_delete = self.find(collection, query)?;
        let count = docs_to_delete.len();

        for doc in docs_to_delete {
            self.delete(collection, &doc.metadata.id)?;
        }

        Ok(count)
    }

    /// Bulk update documents by query
    pub fn bulk_update(
        &mut self,
        collection: &str,
        query: Value,
        update: Value,
    ) -> Result<usize> {
        let docs_to_update = self.find(collection, query)?;
        let count = docs_to_update.len();

        for doc in docs_to_update {
            // Apply update to document (simplified - would need proper update operators)
            let mut updated_doc = doc.clone();
            if let Value::Object(update_obj) = &update {
                let mut json = updated_doc.as_json()?;
                if let Value::Object(ref mut json_obj) = json {
                    for (key, value) in update_obj {
                        json_obj.insert(key.clone(), value.clone());
                    }
                }
                updated_doc = Document::from_json(
                    updated_doc.metadata.id.clone(),
                    collection.to_string(),
                    json,
                )?;
            }

            self.update(collection, &doc.metadata.id, updated_doc)?;
        }

        Ok(count)
    }

    /// Upsert a document (update or insert)
    pub fn upsert(&mut self, collection: &str, id: DocumentId, document: Document) -> Result<bool> {
        let collections = self.collections.read().unwrap();
        let exists = if let Some(docs) = collections.get(collection) {
            docs.contains_key(&id)
        } else {
            false
        };
        drop(collections);

        if exists {
            self.update(collection, &id, document)?;
            Ok(false) // Updated
        } else {
            self.insert(collection, document)?;
            Ok(true) // Inserted
        }
    }

    /// Find one document matching query
    pub fn find_one(&self, collection: &str, query: Value) -> Result<Option<Document>> {
        let results = self.find(collection, query)?;
        Ok(results.into_iter().next())
    }

    /// Replace a document
    pub fn replace(&mut self, collection: &str, id: &DocumentId, document: Document) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        let docs = collections.get_mut(collection)
            .ok_or_else(|| crate::error::DbError::NotFound(
                format!("Collection '{}' not found", collection)
            ))?;

        if !docs.contains_key(id) {
            return Err(crate::error::DbError::NotFound(
                format!("Document {:?} not found", id)
            ));
        }

        docs.insert(id.clone(), document.clone());

        // Emit change event
        let event = ChangeEvent::replace(collection.to_string(), &document)?;
        self.change_stream_manager.add_event(event);

        Ok(())
    }

    /// Get database statistics
    pub fn database_stats(&self) -> DatabaseStats {
        let collections = self.collections.read().unwrap();
        let mut total_documents = 0;
        let mut total_size = 0;

        for docs in collections.values() {
            total_documents += docs.len();
            for doc in docs.values() {
                total_size += doc.metadata.size;
            }
        }

        DatabaseStats {
            collection_count: collections.len(),
            total_documents,
            total_size,
            index_count: self.index_manager.list_indexes().len(),
            change_event_count: self.change_stream_manager.event_count(),
        }
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of collections
    pub collection_count: usize,
    /// Total number of documents
    pub total_documents: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Number of indexes
    pub index_count: usize,
    /// Number of change events
    pub change_event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_document_store_basics() {
        let mut store = DocumentStore::new();

        // Create collection
        store.create_collection("users".to_string()).unwrap();

        // Insert document
        let doc = Document::from_json(
            DocumentId::new_custom("user1"),
            "users".to_string(),
            json!({
                "name": "Alice",
                "age": 30,
                "email": "alice@example.com"
            }),
        ).unwrap();

        let id = store.insert("users", doc).unwrap();

        // Find document
        let found = store.find_by_id("users", &id).unwrap();
        assert_eq!(found.metadata.id, id);

        // Count
        assert_eq!(store.count("users").unwrap(), 1);

        // Delete
        store.delete("users", &id).unwrap();
        assert_eq!(store.count("users").unwrap(), 0);
    }

    #[test]
    fn test_query_documents() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        // Insert test documents
        for _i in 1..=5 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("user{}", i)),
                "users".to_string(),
                json!({
                    "name": format!("User {}", i),
                    "age": 20 + i * 5,
                }),
            ).unwrap();
            store.insert("users", doc).unwrap();
        }

        // Query documents
        let results = store.find("users", json!({
            "age": {"$gte": 30}
        })).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_aggregation() {
        let mut store = DocumentStore::new();
        store.create_collection("sales".to_string()).unwrap();

        // Insert test data
        for _i in 1..=5 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("sale{}", i)),
                "sales".to_string(),
                json!({
                    "product": if i % 2 == 0 { "A" } else { "B" },
                    "amount": i * 10,
                }),
            ).unwrap();
            store.insert("sales", doc).unwrap();
        }

        // Create pipeline
        let pipeline = PipelineBuilder::new()
            .match_stage(json!({"product": "A"}))
            .build();

        let results = store.aggregate("sales", pipeline).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_change_streams() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        // Create change stream
        let filter = ChangeStreamFilter::new()
            .operation_types(vec![ChangeEventType::Insert]);
        let mut cursor = store.watch(filter);

        // Insert document
        let doc = Document::from_json(
            DocumentId::new_custom("user1"),
            "users".to_string(),
            json!({"name": "Alice"}),
        ).unwrap();
        store.insert("users", doc).unwrap();

        // Get changes
        let changes = cursor.next_batch();
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_database_stats() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_uuid(),
            "users".to_string(),
            json!({"name": "Alice"}),
        ).unwrap();
        store.insert("users", doc).unwrap();

        let _stats = store.database_stats();
        assert_eq!(stats.collection_count, 1);
        assert_eq!(stats.total_documents, 1);
    }
}


