// # Document Model
//
// JSON document representation with BSON support, versioning, and metadata management.
// This module provides the core document abstraction for the document store engine.

use std::fmt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime};
use uuid::Uuid;
use crate::error::Result;

/// Document ID types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentId {
    /// UUID-based document ID (default)
    Uuid(Uuid),
    /// Auto-increment integer ID
    AutoIncrement(u64),
    /// Custom string-based ID
    Custom(String),
}

impl DocumentId {
    /// Generate a new UUID-based ID
    pub fn new_uuid() -> Self {
        DocumentId::Uuid(Uuid::new_v4())
    }

    /// Create an auto-increment ID
    pub fn new_auto_increment(id: u64) -> Self {
        DocumentId::AutoIncrement(id)
    }

    /// Create a custom ID from a string
    pub fn new_custom(id: impl Into<String>) -> Self {
        DocumentId::Custom(id.into())
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            DocumentId::Uuid(uuid) => uuid.to_string(),
            DocumentId::AutoIncrement(id) => id.to_string(),
            DocumentId::Custom(s) => s.clone(),
        }
    }

    /// Parse from string representation
    pub fn from_string(s: &str, id_type: IdGenerationType) -> Result<Self> {
        match id_type {
            IdGenerationType::Uuid => {
                let uuid = Uuid::parse_str(s)
                    .map_err(|e| crate::error::DbError::InvalidInput(format!("Invalid UUID: {}", e)))?;
                Ok(DocumentId::Uuid(uuid))
            }
            IdGenerationType::AutoIncrement => {
                let id = s.parse::<u64>()
                Ok(DocumentId::AutoIncrement(id))
            }
            IdGenerationType::Custom => Ok(DocumentId::Custom(s.to_string())),
        }
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// ID generation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdGenerationType {
    /// Generate UUID v4
    Uuid,
    /// Auto-increment integer
    AutoIncrement,
    /// Custom user-provided ID
    Custom,
}

/// Document version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    /// Version number (starts at 1)
    pub version: u64,
    /// Timestamp when this version was created
    pub created_at: u64,
    /// User who created this version
    pub created_by: Option<String>,
    /// Hash of document content for change detection
    pub content_hash: String,
    /// Parent version (for version history)
    pub parent_version: Option<u64>,
}

impl DocumentVersion {
    /// Create a new version
    pub fn new(version: u64, created_by: Option<String>, content_hash: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version,
            created_at,
            created_by,
            content_hash,
            parent_version: if version > 1 { Some(version - 1) } else { None },
        }
    }

    /// Check if this is the initial version
    pub fn is_initial(&self) -> bool {
        self.version == 1
    }
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document ID
    pub id: DocumentId,
    /// Collection name
    pub collection: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modification timestamp
    pub updated_at: u64,
    /// Document version information
    pub version: DocumentVersion,
    /// Document size in bytes
    pub size: usize,
    /// Content type (e.g., "application/json", "application/bson")
    pub content_type: String,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, serde_json::Value>,
    /// Checksum for integrity verification
    pub checksum: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Time-to-live in seconds (optional)
    pub ttl: Option<u64>,
    /// Expiration timestamp (optional)
    pub expires_at: Option<u64>,
}

impl DocumentMetadata {
    /// Create new metadata for a document
    pub fn new(
        id: DocumentId,
        collection: String,
        size: usize,
        content_hash: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            collection,
            created_at: now,
            updated_at: now,
            version: DocumentVersion::new(1, None, content_hash.clone()),
            size,
            content_type: "application/json".to_string(),
            custom_fields: HashMap::new(),
            checksum: content_hash,
            tags: Vec::new(),
            ttl: None,
            expires_at: None,
        }
    }

    /// Check if document has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now >= expires_at
        } else {
            false
        }
    }

    /// Set TTL and calculate expiration
    pub fn set_ttl(&mut self, ttl_seconds: u64) {
        self.ttl = Some(ttl_seconds);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.expires_at = Some(now + ttl_seconds);
    }

    /// Update version information
    pub fn increment_version(&mut self, created_by: Option<String>, content_hash: String) {
        let new_version = self.version.version + 1;
        self.version = DocumentVersion::new(new_version, created_by, content_hash.clone());
        self.checksum = content_hash;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Document storage format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentFormat {
    /// JSON format (human-readable)
    Json,
    /// BSON format (binary, efficient)
    Bson,
    /// Compressed JSON
    CompressedJson,
    /// Compressed BSON
    CompressedBson,
}

/// Document content representation
#[derive(Debug, Clone)]
pub enum DocumentContent {
    /// JSON value
    Json(serde_json::Value),
    /// BSON document
    Bson(bson::Document),
    /// Raw bytes (for compressed or chunked content)
    Bytes(Vec<u8>),
}

impl DocumentContent {
    /// Convert to JSON value
    pub fn to_json(&self) -> Result<serde_json::Value> {
        match self {
            DocumentContent::Json(v) => Ok(v.clone()),
            DocumentContent::Bson(doc) => {
                // Convert BSON to JSON via serialization
                let json = serde_json::to_value(doc)
                Ok(json)
            }
            DocumentContent::Bytes(bytes) => {
                let json: serde_json::Value = serde_json::from_slice(bytes)
                Ok(json)
            }
        }
    }

    /// Convert to BSON document
    pub fn to_bson(&self) -> Result<bson::Document> {
        match self {
            DocumentContent::Json(v) => {
                let bson_value = bson::to_bson(v)
                if let bson::Bson::Document(doc) = bson_value {
                    Ok(doc)
                } else {
                }
            }
            DocumentContent::Bson(doc) => Ok(doc.clone()),
            DocumentContent::Bytes(bytes) => {
                let doc = bson::from_slice(bytes)
                Ok(doc)
            }
        }
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            DocumentContent::Json(v) => serde_json::to_vec(v).unwrap_or_default().len(),
            DocumentContent::Bson(doc) => {
                let mut buf = Vec::new();
                doc.to_writer(&mut buf).unwrap_or(());
                buf.len()
            }
            DocumentContent::Bytes(bytes) => bytes.len(),
        }
    }

    /// Calculate content hash
    pub fn hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let bytes = match self {
            DocumentContent::Json(v) => serde_json::to_vec(v).unwrap_or_default(),
            DocumentContent::Bson(doc) => {
                let mut buf = Vec::new();
                doc.to_writer(&mut buf).unwrap_or(());
                buf
            }
            DocumentContent::Bytes(bytes) => bytes.clone(),
        };
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    }
}

/// Main document structure
#[derive(Debug, Clone)]
pub struct Document {
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Document content
    pub content: DocumentContent,
    /// Storage format hint
    pub format: DocumentFormat,
}

impl Document {
    /// Create a new document from JSON
    pub fn from_json(
        id: DocumentId,
        collection: String,
        json: serdejson::Value,
    ) -> Result<Self> {
        let content = DocumentContent::Json(json));
        let content_hash = content.hash();
        let size = content.size();

        Ok(Self {
            metadata: DocumentMetadata::new(id, collection, size, content_hash),
            content,
            format: DocumentFormat::Json,
        })
    }

    /// Create a new document from BSON
    pub fn from_bson(
        id: DocumentId,
        collection: String,
        bson: bson::Document,
    ) -> Result<Self> {
        let content = DocumentContent::Bson(bson);
        let content_hash = content.hash();
        let size = content.size();

        Ok(Self {
            metadata: DocumentMetadata::new(id, collection, size, content_hash),
            content,
            format: DocumentFormat::Bson,
        })
    }

    /// Get document as JSON
    pub fn as_json(&self) -> Result<serde_json::Value> {
        self.content.to_json()
    }

    /// Get document as BSON
    pub fn as_bson(&self) -> Result<bson::Document> {
        self.content.to_bson()
    }

    /// Update document content
    pub fn update_content(&mut self, content: DocumentContent, updated_by: Option<String>) -> Result<()> {
        let content_hash = content.hash();
        let size = content.size();

        self.metadata.increment_version(updated_by, content_hash);
        self.metadata.size = size;
        self.content = content;

        Ok(())
    }

    /// Add a tag to the document
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }

    /// Remove a tag from the document
    pub fn remove_tag(&mut self, tag: &str) {
        self.metadata.tags.retain(|t| t != tag);
    }

    /// Set custom metadata field
    pub fn set_custom_field(&mut self, key: String, value: serde_json::Value) {
        self.metadata.custom_fields.insert(key, value);
    }

    /// Get custom metadata field
    pub fn get_custom_field(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.custom_fields.get(key)
    }
}

/// Document chunk for large document handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Parent document ID
    pub document_id: DocumentId,
    /// Chunk sequence number
    pub chunk_number: u32,
    /// Total number of chunks
    pub total_chunks: u32,
    /// Chunk data
    pub data: Vec<u8>,
    /// Chunk size
    pub size: usize,
    /// Chunk checksum
    pub checksum: String,
}

impl DocumentChunk {
    /// Create a new chunk
    pub fn new(
        document_id: DocumentId,
        chunk_number: u32,
        total_chunks: u32,
        data: Vec<u8>,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = format!("{:x}", hasher.finalize()));
        let size = data.len();

        Self {
            document_id,
            chunk_number,
            total_chunks,
            data,
            size,
            checksum,
        }
    }

    /// Verify chunk integrity
    pub fn verify(&self) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        let calculated_checksum = format!("{:x}", hasher.finalize()));
        calculated_checksum == self.checksum
    }
}

/// Large document handler for chunking
pub struct LargeDocumentHandler {
    /// Maximum chunk size in bytes
    chunk_size: usize,
}

impl LargeDocumentHandler {
    /// Create a new handler with specified chunk size
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Split a document into chunks
    pub fn chunk_document(&self, document: &Document) -> Result<Vec<DocumentChunk>> {
        let bytes = match &document.content {
            DocumentContent::Json(v) => serde_json::to_vec(v)
            DocumentContent::Bson(doc) => {
                let mut buf = Vec::new());
                doc.to_writer(&mut buf)
                buf
            }
            DocumentContent::Bytes(bytes) => bytes.clone(),
        };

        let total_size = bytes.len();
        let total_chunks = (total_size + self.chunk_size - 1) / self.chunk_size;
        let mut chunks = Vec::new();

        for i in 0..total_chunks {
            let start = i * self.chunk_size;
            let end = std::cmp::min(start + self.chunk_size, total_size);
            let chunk_data = bytes[start..end].to_vec();

            let chunk = DocumentChunk::new(
                document.metadata.id.clone(),
                i as u32,
                total_chunks as u32,
                chunk_data,
            );
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    /// Reassemble chunks into a document
    pub fn reassemble_chunks(&self, chunks: Vec<DocumentChunk>) -> Result<Vec<u8>> {
        if chunks.is_empty() {
        }

        // Sort chunks by chunk number
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by_key(|c| c.chunk_number);

        // Verify all chunks are present
        let total_chunks = sorted_chunks[0].total_chunks;
        if sorted_chunks.len() != total_chunks as usize {
                format!("Missing chunks: expected {}, got {}", total_chunks, sorted_chunks.len())
            )));
        }

        // Verify chunk integrity and reassemble
        let mut data = Vec::new();
        for (i, chunk) in sorted_chunks.iter().enumerate() {
            if chunk.chunk_number != i as u32 {
                    format!("Chunk sequence error: expected {}, got {}", i, chunk.chunk_number)
                )));
            }
            if !chunk.verify() {
                    format!("Chunk {} checksum verification failed", i)
                )));
            }
            data.extend_from_slice(&chunk.data);
        }

        Ok(data)
    }
}

/// Document builder for fluent API
pub struct DocumentBuilder {
    id: Option<DocumentId>,
    collection: String,
    content: Option<DocumentContent>,
    format: DocumentFormat,
    tags: Vec<String>,
    custom_fields: HashMap<String, serde_json::Value>,
    ttl: Option<u64>,
}

impl DocumentBuilder {
    /// Create a new builder for a collection
    pub fn new(collection: impl Into<String>) -> Self {
        Self {
            id: None,
            collection: collection.into(),
            content: None,
            format: DocumentFormat::Json,
            tags: Vec::new(),
            custom_fields: HashMap::new(),
            ttl: None,
        }
    }

    /// Set document ID
    pub fn id(mut self, id: DocumentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set JSON content
    pub fn json(mut self, json: serdejson::Value) -> Self {
        self.content = Some(DocumentContent::Json(json));
        self.format = DocumentFormat::Json;
        self
    }

    /// Set BSON content
    pub fn bson(mut self, bson: bson::Document) -> Self {
        self.content = Some(DocumentContent::Bson(bson));
        self.format = DocumentFormat::Bson;
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add custom metadata field
    pub fn custom_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom_fields.insert(key.into(), value);
        self
    }

    /// Set TTL
    pub fn ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl = Some(ttl_seconds);
        self
    }

    /// Build the document
    pub fn build(self) -> Result<Document> {
        let content = self.content.ok_or_else(|| {
        })?;

        let id = self.id.unwrap_or_else(DocumentId::new_uuid);
        let content_hash = content.hash();
        let size = content.size();

        let mut metadata = DocumentMetadata::new(id, self.collection, size, content_hash);
        metadata.tags = self.tags;
        metadata.custom_fields = self.custom_fields;
        if let Some(ttl) = self.ttl {
            metadata.set_ttl(ttl);
        }

        Ok(Document {
            metadata,
            content,
            format: self.format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_document_id_generation() {
        let uuid_id = DocumentId::new_uuid();
        assert!(matches!(uuid_id, DocumentId::Uuid(_)));

        let auto_id = DocumentId::new_auto_increment(42);
        assert_eq!(auto_id, DocumentId::AutoIncrement(42));

        let custom_id = DocumentId::new_custom("my-custom-id");
        assert_eq!(custom_id, DocumentId::Custom("my-custom-id".to_string()));
    }

    #[test]
    fn test_document_creation() {
        let json_doc = json!({
            "name": "John Doe",
            "age": 30,
            "email": "john@example.com"
        });

        let doc = Document::from_json(
            DocumentId::new_uuid(),
            "users".to_string(),
            json_doc.clone(),
        ).unwrap();

        assert_eq!(doc.metadata.collection, "users");
        assert_eq!(doc.metadata.version.version, 1);

        let retrieved_json = doc.as_json().unwrap();
        assert_eq!(retrieved_json, json_doc);
    }

    #[test]
    fn test_document_versioning() {
        let json_doc = json!({"value": 1});
        let mut doc = Document::from_json(
            DocumentId::new_uuid(),
            "test".to_string(),
            json_doc,
        ).unwrap();

        assert_eq!(doc.metadata.version.version, 1);

        let new_content = DocumentContent::Json(json!({"value": 2}));
        doc.update_content(new_content, Some("user1".to_string())).unwrap();

        assert_eq!(doc.metadata.version.version, 2);
        assert_eq!(doc.metadata.version.created_by, Some("user1".to_string()));
    }

    #[test]
    fn test_document_chunking() {
        let large_json = json!({
            "data": "x".repeat(10000)
        });

        let doc = Document::from_json(
            DocumentId::new_uuid(),
            "large_docs".to_string(),
            large_json,
        ).unwrap();

        let handler = LargeDocumentHandler::new(4096);
        let chunks = handler.chunk_document(&doc).unwrap();

        assert!(chunks.len() > 1);

        for chunk in &chunks {
            assert!(chunk.verify());
        }

        let reassembled = handler.reassemble_chunks(chunks).unwrap();
        assert!(!reassembled.is_empty());
    }

    #[test]
    fn test_document_builder() {
        let doc = DocumentBuilder::new("users")
            .id(DocumentId::new_custom("user-123"))
            .json(json!({"name": "Alice"}))
            .tag("premium")
            .tag("active")
            .custom_field("source", json!("api"))
            .ttl(3600)
            .build()
            .unwrap();

        assert_eq!(doc.metadata.id, DocumentId::Custom("user-123".to_string()));
        assert_eq!(doc.metadata.tags.len(), 2);
        assert_eq!(doc.metadata.ttl, Some(3600));
    }
}
