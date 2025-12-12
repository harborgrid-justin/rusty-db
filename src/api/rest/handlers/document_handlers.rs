// Document Store API Handlers
//
// REST API endpoints for document database operations including:
// - Collection management
// - Document CRUD operations
// - Query By Example (QBE)
// - Aggregation pipelines
// - Change streams

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::rest::types::{ApiState, ApiError, ApiResult};
use crate::document_store::{
    DocumentStore, Document, DocumentId, CollectionSettings,
    PipelineBuilder, ChangeStreamFilter, ChangeEventType,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub schema_validation: Option<serde_json::Value>,
    pub max_documents: Option<usize>,
    pub capped: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CollectionResponse {
    pub name: String,
    pub document_count: usize,
    pub size_bytes: usize,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentQueryRequest {
    pub filter: serde_json::Value,
    pub projection: Option<Vec<String>>,
    pub sort: Option<HashMap<String, i32>>,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentQueryResponse {
    pub documents: Vec<serde_json::Value>,
    pub count: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InsertDocumentRequest {
    pub document: serde_json::Value,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InsertDocumentResponse {
    pub id: String,
    pub inserted: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkInsertRequest {
    pub documents: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkInsertResponse {
    pub inserted_ids: Vec<String>,
    pub inserted_count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateDocumentRequest {
    pub filter: serde_json::Value,
    pub update: serde_json::Value,
    pub upsert: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateDocumentResponse {
    pub matched_count: usize,
    pub modified_count: usize,
    pub upserted_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteDocumentRequest {
    pub filter: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteDocumentResponse {
    pub deleted_count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AggregationRequest {
    pub pipeline: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AggregationResponse {
    pub results: Vec<serde_json::Value>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangeStreamRequest {
    pub operation_types: Option<Vec<String>>,
    pub collection_filter: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangeStreamResponse {
    pub cursor_id: String,
    pub changes: Vec<ChangeEvent>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangeEvent {
    pub operation_type: String,
    pub collection: String,
    pub document_id: String,
    pub timestamp: i64,
    pub document: Option<serde_json::Value>,
}

// ============================================================================
// Handler Functions
// ============================================================================

// Global document store instance (simplified - in production would use proper state management)
lazy_static::lazy_static! {
    static ref DOCUMENT_STORE: parking_lot::RwLock<DocumentStore> = parking_lot::RwLock::new(DocumentStore::new());
}

/// Create a new collection
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections",
    request_body = CreateCollectionRequest,
    responses(
        (status = 201, description = "Collection created", body = CollectionResponse),
        (status = 409, description = "Collection already exists", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn create_collection(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateCollectionRequest>,
) -> ApiResult<(StatusCode, Json<CollectionResponse>)> {
    let mut store = DOCUMENT_STORE.write();

    let mut settings = CollectionSettings::default();
    if let Some(max_docs) = request.max_documents {
        settings.max_documents = Some(max_docs);
    }
    if let Some(capped) = request.capped {
        settings.capped = capped;
    }

    store.create_collection_with_settings(request.name.clone(), settings)
        .map_err(|e| ApiError::new("COLLECTION_CREATE_FAILED", format!("Failed to create collection: {}", e)))?;

    let count = store.count(&request.name).unwrap_or(0);

    Ok((StatusCode::CREATED, Json(CollectionResponse {
        name: request.name,
        document_count: count,
        size_bytes: 0,
        created_at: chrono::Utc::now().timestamp(),
    })))
}

/// List all collections
#[utoipa::path(
    get,
    path = "/api/v1/documents/collections",
    responses(
        (status = 200, description = "Collections listed", body = Vec<String>),
    ),
    tag = "documents"
)]
pub async fn list_collections(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<String>>> {
    let store = DOCUMENT_STORE.read();
    Ok(Json(store.list_collections()))
}

/// Get collection information
#[utoipa::path(
    get,
    path = "/api/v1/documents/collections/{name}",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    responses(
        (status = 200, description = "Collection info", body = CollectionResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn get_collection(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<CollectionResponse>> {
    let store = DOCUMENT_STORE.read();

    let count = store.count(&name)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Collection '{}' not found", name)))?;

    let stats = store.get_stats(&name)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Collection '{}' not found", name)))?;

    Ok(Json(CollectionResponse {
        name,
        document_count: count,
        size_bytes: stats.total_size as usize,
        created_at: stats.created_at as i64,
    }))
}

/// Drop a collection
#[utoipa::path(
    delete,
    path = "/api/v1/documents/collections/{name}",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    responses(
        (status = 204, description = "Collection dropped"),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn drop_collection(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let mut store = DOCUMENT_STORE.write();

    store.drop_collection(&name)
        .map_err(|e| ApiError::new("DROP_FAILED", format!("Failed to drop collection: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Find documents matching a query
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/find",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = DocumentQueryRequest,
    responses(
        (status = 200, description = "Documents found", body = DocumentQueryResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn find_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(query): Json<DocumentQueryRequest>,
) -> ApiResult<Json<DocumentQueryResponse>> {
    let store = DOCUMENT_STORE.read();

    let mut documents = store.find(&collection, query.filter)
        .map_err(|e| ApiError::new("QUERY_FAILED", format!("Query failed: {}", e)))?;

    // Apply skip
    if let Some(skip) = query.skip {
        documents = documents.into_iter().skip(skip).collect();
    }

    // Apply limit
    let has_more = if let Some(limit) = query.limit {
        let total = documents.len();
        documents.truncate(limit);
        total > limit
    } else {
        false
    };

    let docs_json: Vec<serde_json::Value> = documents.iter()
        .map(|doc| doc.as_json().unwrap_or(serde_json::Value::Null))
        .collect();

    Ok(Json(DocumentQueryResponse {
        count: docs_json.len(),
        documents: docs_json,
        has_more,
    }))
}

/// Insert a document
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/insert",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = InsertDocumentRequest,
    responses(
        (status = 201, description = "Document inserted", body = InsertDocumentResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn insert_document(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(request): Json<InsertDocumentRequest>,
) -> ApiResult<(StatusCode, Json<InsertDocumentResponse>)> {
    let mut store = DOCUMENT_STORE.write();

    let doc_id = if let Some(id) = request.id {
        DocumentId::new_custom(id)
    } else {
        DocumentId::new_uuid()
    };

    let document = Document::from_json(doc_id.clone(), collection.clone(), request.document)
        .map_err(|e| ApiError::new("INVALID_DOCUMENT", format!("Invalid document: {}", e)))?;

    let id = store.insert(&collection, document)
        .map_err(|e| ApiError::new("INSERT_FAILED", format!("Insert failed: {}", e)))?;

    Ok((StatusCode::CREATED, Json(InsertDocumentResponse {
        id: format!("{:?}", id),
        inserted: true,
    })))
}

/// Bulk insert documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/bulk-insert",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = BulkInsertRequest,
    responses(
        (status = 201, description = "Documents inserted", body = BulkInsertResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn bulk_insert_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(request): Json<BulkInsertRequest>,
) -> ApiResult<(StatusCode, Json<BulkInsertResponse>)> {
    let mut store = DOCUMENT_STORE.write();

    let documents: Result<Vec<Document>, _> = request.documents
        .into_iter()
        .map(|json| {
            let id = DocumentId::new_uuid();
            Document::from_json(id, collection.clone(), json)
        })
        .collect();

    let documents = documents
        .map_err(|e| ApiError::new("INVALID_DOCUMENT", format!("Invalid document: {}", e)))?;

    let ids = store.bulk_insert(&collection, documents)
        .map_err(|e| ApiError::new("BULK_INSERT_FAILED", format!("Bulk insert failed: {}", e)))?;

    let id_strings: Vec<String> = ids.iter().map(|id| format!("{:?}", id)).collect();

    Ok((StatusCode::CREATED, Json(BulkInsertResponse {
        inserted_count: id_strings.len(),
        inserted_ids: id_strings,
    })))
}

/// Update documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/update",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = UpdateDocumentRequest,
    responses(
        (status = 200, description = "Documents updated", body = UpdateDocumentResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn update_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(request): Json<UpdateDocumentRequest>,
) -> ApiResult<Json<UpdateDocumentResponse>> {
    let mut store = DOCUMENT_STORE.write();

    let modified = store.bulk_update(&collection, request.filter, request.update)
        .map_err(|e| ApiError::new("UPDATE_FAILED", format!("Update failed: {}", e)))?;

    Ok(Json(UpdateDocumentResponse {
        matched_count: modified,
        modified_count: modified,
        upserted_id: None,
    }))
}

/// Delete documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/delete",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = DeleteDocumentRequest,
    responses(
        (status = 200, description = "Documents deleted", body = DeleteDocumentResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn delete_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(request): Json<DeleteDocumentRequest>,
) -> ApiResult<Json<DeleteDocumentResponse>> {
    let mut store = DOCUMENT_STORE.write();

    let deleted = store.bulk_delete(&collection, request.filter)
        .map_err(|e| ApiError::new("DELETE_FAILED", format!("Delete failed: {}", e)))?;

    Ok(Json(DeleteDocumentResponse {
        deleted_count: deleted,
    }))
}

/// Aggregate documents using pipeline
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/aggregate",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = AggregationRequest,
    responses(
        (status = 200, description = "Aggregation completed", body = AggregationResponse),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn aggregate_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
    Json(request): Json<AggregationRequest>,
) -> ApiResult<Json<AggregationResponse>> {
    let store = DOCUMENT_STORE.read();

    // Build pipeline from JSON stages
    let mut builder = PipelineBuilder::new();
    for stage in request.pipeline {
        if let Some(obj) = stage.as_object() {
            if let Some(match_val) = obj.get("$match") {
                builder = builder.match_stage(match_val.clone());
            }
            // Add more stage types as needed
        }
    }

    let pipeline = builder.build();

    let results = store.aggregate(&collection, pipeline)
        .map_err(|e| ApiError::new("AGGREGATION_FAILED", format!("Aggregation failed: {}", e)))?;

    Ok(Json(AggregationResponse {
        count: results.len(),
        results,
    }))
}

/// Get document count
#[utoipa::path(
    get,
    path = "/api/v1/documents/collections/{name}/count",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    responses(
        (status = 200, description = "Document count", body = usize),
        (status = 404, description = "Collection not found", body = ApiError),
    ),
    tag = "documents"
)]
pub async fn count_documents(
    State(_state): State<Arc<ApiState>>,
    Path(collection): Path<String>,
) -> ApiResult<Json<usize>> {
    let store = DOCUMENT_STORE.read();

    let count = store.count(&collection)
        .map_err(|e| ApiError::new("COUNT_FAILED", format!("Count failed: {}", e)))?;

    Ok(Json(count))
}

/// Watch for changes in a collection
#[utoipa::path(
    post,
    path = "/api/v1/documents/collections/{name}/watch",
    params(
        ("name" = String, Path, description = "Collection name")
    ),
    request_body = ChangeStreamRequest,
    responses(
        (status = 200, description = "Change stream created", body = ChangeStreamResponse),
    ),
    tag = "documents"
)]
pub async fn watch_collection(
    State(_state): State<Arc<ApiState>>,
    Path(_collection): Path<String>,
    Json(request): Json<ChangeStreamRequest>,
) -> ApiResult<Json<ChangeStreamResponse>> {
    let store = DOCUMENT_STORE.read();

    let mut filter = ChangeStreamFilter::new();
    if let Some(op_types) = request.operation_types {
        let types: Vec<ChangeEventType> = op_types.iter().filter_map(|s| {
            match s.as_str() {
                "insert" => Some(ChangeEventType::Insert),
                "update" => Some(ChangeEventType::Update),
                "delete" => Some(ChangeEventType::Delete),
                "replace" => Some(ChangeEventType::Replace),
                _ => None,
            }
        }).collect();
        filter = filter.operation_types(types);
    }

    let mut cursor = store.watch(filter);
    let changes = cursor.next_batch();

    let change_events: Vec<ChangeEvent> = changes.iter().map(|c| {
        ChangeEvent {
            operation_type: format!("{:?}", c.operation_type),
            collection: c.collection.clone(),
            document_id: c.document_key.as_ref().map(|k| format!("{:?}", k)).unwrap_or_default(),
            timestamp: c.cluster_time as i64,
            document: c.full_document.clone(),
        }
    }).collect();

    Ok(Json(ChangeStreamResponse {
        cursor_id: uuid::Uuid::new_v4().to_string(),
        changes: change_events,
    }))
}
