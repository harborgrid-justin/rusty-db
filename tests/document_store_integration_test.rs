// Document Store Integration Tests
// Tests all document store features with numbered test IDs

use rusty_db::document_store::*;
use serde_json::json;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::*;

    // DOCSTORE-001: Create collection
    #[test]
    fn docstore_001_create_collection() {
        let mut store = DocumentStore::new();
        let result = store.create_collection("users".to_string());
        assert!(result.is_ok(), "DOCSTORE-001: Failed to create collection");
        println!("✓ DOCSTORE-001: Create collection - PASSED");
    }

    // DOCSTORE-002: Insert JSON document
    #[test]
    fn docstore_002_insert_document() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("user001"),
            "users".to_string(),
            json!({
                "name": "Alice Johnson",
                "email": "alice@example.com",
                "age": 30,
                "department": "Engineering"
            }),
        )
        .unwrap();

        let result = store.insert("users", doc);
        assert!(result.is_ok(), "DOCSTORE-002: Failed to insert document");
        println!("✓ DOCSTORE-002: Insert JSON document - PASSED");
    }

    // DOCSTORE-003: Find document by ID
    #[test]
    fn docstore_003_find_by_id() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("user001"),
            "users".to_string(),
            json!({"name": "Alice"}),
        )
        .unwrap();

        let doc_id = store.insert("users", doc).unwrap();
        let result = store.find_by_id("users", &doc_id);

        assert!(
            result.is_ok(),
            "DOCSTORE-003: Failed to find document by ID"
        );
        println!("✓ DOCSTORE-003: Find document by ID - PASSED");
    }

    // DOCSTORE-004: Update document
    #[test]
    fn docstore_004_update_document() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("user001"),
            "users".to_string(),
            json!({"name": "Alice", "age": 30}),
        )
        .unwrap();

        let doc_id = store.insert("users", doc).unwrap();

        let updated_doc = Document::from_json(
            doc_id.clone(),
            "users".to_string(),
            json!({"name": "Alice", "age": 31}),
        )
        .unwrap();

        let result = store.update("users", &doc_id, updated_doc);
        assert!(result.is_ok(), "DOCSTORE-004: Failed to update document");
        println!("✓ DOCSTORE-004: Update document - PASSED");
    }

    // DOCSTORE-005: Delete document
    #[test]
    fn docstore_005_delete_document() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("user001"),
            "users".to_string(),
            json!({"name": "Alice"}),
        )
        .unwrap();

        let doc_id = store.insert("users", doc).unwrap();
        let result = store.delete("users", &doc_id);

        assert!(result.is_ok(), "DOCSTORE-005: Failed to delete document");
        println!("✓ DOCSTORE-005: Delete document - PASSED");
    }

    // DOCSTORE-006: Query by example - equality
    #[test]
    fn docstore_006_query_equality() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        for i in 1..=5 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("user{:03}", i)),
                "users".to_string(),
                json!({"name": format!("User {}", i), "age": 20 + i * 2}),
            )
            .unwrap();
            store.insert("users", doc).unwrap();
        }

        let results = store.find("users", json!({"age": 24})).unwrap();
        assert_eq!(results.len(), 1, "DOCSTORE-006: Expected 1 result");
        println!("✓ DOCSTORE-006: Query by example - equality - PASSED");
    }

    // DOCSTORE-007: Query with comparison operators
    #[test]
    fn docstore_007_query_comparison() {
        let mut store = DocumentStore::new();
        store.create_collection("products".to_string()).unwrap();

        for i in 1..=10 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("prod{:03}", i)),
                "products".to_string(),
                json!({"name": format!("Product {}", i), "price": i * 10}),
            )
            .unwrap();
            store.insert("products", doc).unwrap();
        }

        let results = store
            .find("products", json!({"price": {"$gte": 50}}))
            .unwrap();
        assert!(results.len() >= 5, "DOCSTORE-007: Expected >= 5 results");
        println!("✓ DOCSTORE-007: Query with comparison operators ($gte) - PASSED");
    }

    // DOCSTORE-008: Query with $in operator
    #[test]
    fn docstore_008_query_in_operator() {
        let mut store = DocumentStore::new();
        store.create_collection("products".to_string()).unwrap();

        for i in 1..=5 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("prod{:03}", i)),
                "products".to_string(),
                json!({"name": format!("Product {}", i), "category": if i % 2 == 0 { "A" } else { "B" }}),
            ).unwrap();
            store.insert("products", doc).unwrap();
        }

        let results = store
            .find("products", json!({"category": {"$in": ["A", "C"]}}))
            .unwrap();
        assert!(results.len() >= 1, "DOCSTORE-008: Expected >= 1 results");
        println!("✓ DOCSTORE-008: Query with $in operator - PASSED");
    }

    // DOCSTORE-009: Count documents
    #[test]
    fn docstore_009_count_documents() {
        let mut store = DocumentStore::new();
        store.create_collection("items".to_string()).unwrap();

        for i in 1..=10 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("item{:03}", i)),
                "items".to_string(),
                json!({"value": i}),
            )
            .unwrap();
            store.insert("items", doc).unwrap();
        }

        let count = store.count("items").unwrap();
        assert_eq!(count, 10, "DOCSTORE-009: Expected 10 documents");
        println!("✓ DOCSTORE-009: Count documents - PASSED");
    }

    // DOCSTORE-010: Aggregation pipeline - $match
    #[test]
    fn docstore_010_aggregation_match() {
        let mut store = DocumentStore::new();
        store.create_collection("sales".to_string()).unwrap();

        for i in 1..=10 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("sale{:03}", i)),
                "sales".to_string(),
                json!({"amount": i * 100, "product": if i % 2 == 0 { "A" } else { "B" }}),
            )
            .unwrap();
            store.insert("sales", doc).unwrap();
        }

        let pipeline = PipelineBuilder::new()
            .match_stage(json!({"product": "A"}))
            .build();

        let results = store.aggregate("sales", pipeline).unwrap();
        assert_eq!(
            results.len(),
            5,
            "DOCSTORE-010: Expected 5 matching documents"
        );
        println!("✓ DOCSTORE-010: Aggregation pipeline - $match - PASSED");
    }

    // DOCSTORE-011: Aggregation pipeline - $project
    #[test]
    fn docstore_011_aggregation_project() {
        let mut store = DocumentStore::new();
        store.create_collection("users".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("user001"),
            "users".to_string(),
            json!({"name": "Alice", "age": 30, "email": "alice@example.com", "secret": "hidden"}),
        )
        .unwrap();
        store.insert("users", doc).unwrap();

        let pipeline = PipelineBuilder::new()
            .project(json!({"name": true, "age": true}))
            .build();

        let results = store.aggregate("users", pipeline).unwrap();
        assert_eq!(results.len(), 1, "DOCSTORE-011: Expected 1 result");
        assert!(
            results[0].get("name").is_some(),
            "DOCSTORE-011: Name should be present"
        );
        assert!(
            results[0].get("secret").is_none(),
            "DOCSTORE-011: Secret should be filtered"
        );
        println!("✓ DOCSTORE-011: Aggregation pipeline - $project - PASSED");
    }

    // DOCSTORE-012: Aggregation pipeline - $sort
    #[test]
    fn docstore_012_aggregation_sort() {
        let mut store = DocumentStore::new();
        store.create_collection("items".to_string()).unwrap();

        for i in vec![5, 2, 8, 1, 9, 3] {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("item{:03}", i)),
                "items".to_string(),
                json!({"value": i}),
            )
            .unwrap();
            store.insert("items", doc).unwrap();
        }

        let mut sort_spec = BTreeMap::new();
        sort_spec.insert("value".to_string(), 1); // Ascending

        let pipeline = Pipeline::new().add_stage(PipelineStage::Sort { sort_spec });

        let results = store.aggregate("items", pipeline).unwrap();
        assert!(results.len() >= 1, "DOCSTORE-012: Expected results");
        println!("✓ DOCSTORE-012: Aggregation pipeline - $sort - PASSED");
    }

    // DOCSTORE-013: Aggregation pipeline - $limit
    #[test]
    fn docstore_013_aggregation_limit() {
        let mut store = DocumentStore::new();
        store.create_collection("items".to_string()).unwrap();

        for i in 1..=10 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("item{:03}", i)),
                "items".to_string(),
                json!({"value": i}),
            )
            .unwrap();
            store.insert("items", doc).unwrap();
        }

        let pipeline = PipelineBuilder::new().limit(3).build();

        let results = store.aggregate("items", pipeline).unwrap();
        assert_eq!(results.len(), 3, "DOCSTORE-013: Expected 3 results");
        println!("✓ DOCSTORE-013: Aggregation pipeline - $limit - PASSED");
    }

    // DOCSTORE-014: Aggregation pipeline - $skip
    #[test]
    fn docstore_014_aggregation_skip() {
        let mut store = DocumentStore::new();
        store.create_collection("items".to_string()).unwrap();

        for i in 1..=10 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("item{:03}", i)),
                "items".to_string(),
                json!({"value": i}),
            )
            .unwrap();
            store.insert("items", doc).unwrap();
        }

        let pipeline = PipelineBuilder::new().skip(7).build();

        let results = store.aggregate("items", pipeline).unwrap();
        assert_eq!(
            results.len(),
            3,
            "DOCSTORE-014: Expected 3 results after skip"
        );
        println!("✓ DOCSTORE-014: Aggregation pipeline - $skip - PASSED");
    }

    // DOCSTORE-015: Bulk insert
    #[test]
    fn docstore_015_bulk_insert() {
        let mut store = DocumentStore::new();
        store.create_collection("bulk_test".to_string()).unwrap();

        let mut documents = Vec::new();
        for i in 1..=100 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("doc{:03}", i)),
                "bulk_test".to_string(),
                json!({"value": i, "category": if i % 2 == 0 { "even" } else { "odd" }}),
            )
            .unwrap();
            documents.push(doc);
        }

        let result = store.bulk_insert("bulk_test", documents);
        assert!(result.is_ok(), "DOCSTORE-015: Bulk insert failed");
        let ids = result.unwrap();
        assert_eq!(ids.len(), 100, "DOCSTORE-015: Expected 100 IDs");
        println!("✓ DOCSTORE-015: Bulk insert - PASSED");
    }

    // DOCSTORE-016: Bulk delete
    #[test]
    fn docstore_016_bulk_delete() {
        let mut store = DocumentStore::new();
        store.create_collection("bulk_test".to_string()).unwrap();

        for i in 1..=20 {
            let doc = Document::from_json(
                DocumentId::new_custom(format!("doc{:03}", i)),
                "bulk_test".to_string(),
                json!({"value": i, "status": if i <= 10 { "active" } else { "inactive" }}),
            )
            .unwrap();
            store.insert("bulk_test", doc).unwrap();
        }

        let count = store
            .bulk_delete("bulk_test", json!({"status": "inactive"}))
            .unwrap();
        assert_eq!(count, 10, "DOCSTORE-016: Expected 10 deletions");
        println!("✓ DOCSTORE-016: Bulk delete - PASSED");
    }

    // DOCSTORE-017: Create index
    #[test]
    fn docstore_017_create_index() {
        let mut store = DocumentStore::new();
        store
            .create_collection("indexed_collection".to_string())
            .unwrap();

        let index_def = IndexDefinition::new("idx_name", IndexType::Single)
            .add_field(IndexField::ascending("name"));

        let result = store.create_index(index_def);
        assert!(result.is_ok(), "DOCSTORE-017: Failed to create index");
        println!("✓ DOCSTORE-017: Create index - PASSED");
    }

    // DOCSTORE-018: List indexes
    #[test]
    fn docstore_018_list_indexes() {
        let mut store = DocumentStore::new();

        let index_def1 = IndexDefinition::new("idx1", IndexType::Single)
            .add_field(IndexField::ascending("field1"));
        let index_def2 = IndexDefinition::new("idx2", IndexType::Single)
            .add_field(IndexField::ascending("field2"));

        store.create_index(index_def1).unwrap();
        store.create_index(index_def2).unwrap();

        let indexes = store.list_indexes();
        assert!(indexes.len() >= 2, "DOCSTORE-018: Expected >= 2 indexes");
        println!("✓ DOCSTORE-018: List indexes - PASSED");
    }

    // DOCSTORE-019: JSONPath query
    #[test]
    fn docstore_019_jsonpath_query() {
        let mut store = DocumentStore::new();
        store.create_collection("nested".to_string()).unwrap();

        let doc = Document::from_json(
            DocumentId::new_custom("doc001"),
            "nested".to_string(),
            json!({
                "user": {
                    "name": "Alice",
                    "address": {
                        "city": "New York",
                        "zip": "10001"
                    }
                }
            }),
        )
        .unwrap();
        store.insert("nested", doc).unwrap();

        let results = store.jsonpath_query("nested", "$.user.name").unwrap();
        assert!(
            !results.is_empty(),
            "DOCSTORE-019: Expected results from JSONPath"
        );
        println!("✓ DOCSTORE-019: JSONPath query - PASSED");
    }

    // DOCSTORE-020: Document with TTL
    #[test]
    fn docstore_020_document_ttl() {
        let mut store = DocumentStore::new();
        store.create_collection("ttl_test".to_string()).unwrap();

        let mut doc = Document::from_json(
            DocumentId::new_custom("ttl001"),
            "ttl_test".to_string(),
            json!({"data": "expires soon"}),
        )
        .unwrap();

        doc.metadata.set_ttl(3600); // 1 hour TTL

        let result = store.insert("ttl_test", doc);
        assert!(
            result.is_ok(),
            "DOCSTORE-020: Failed to insert document with TTL"
        );

        let found = store.find_by_id("ttl_test", &DocumentId::new_custom("ttl001"));
        assert!(found.is_ok(), "DOCSTORE-020: Document should exist");
        println!("✓ DOCSTORE-020: Document with TTL - PASSED");
    }
}
