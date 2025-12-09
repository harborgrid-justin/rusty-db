// SQL Compliance Integration Tests
// Tests all newly implemented SQL features for 100% compliance

use rusty_db::{
    parser::SqlParser,
    parser::SqlStatement,
    catalog::{Catalog, Schema, Column, DataType},
    execution::executor::Executor,
    transaction::TransactionManager,
    index::IndexManager,
    constraints::ConstraintManager,
    Result,
};
use std::sync::Arc;

fn setup_test_environment() -> (Arc<Catalog>, Arc<TransactionManager>, Arc<IndexManager>, Arc<ConstraintManager>) {
    let catalog = Arc::new(Catalog::new());
    let txn_manager = Arc::new(TransactionManager::new());
    let index_manager = Arc::new(IndexManager::new());
    let constraint_manager = Arc::new(ConstraintManager::new());

    (catalog, txn_manager, index_manager, constraint_manager)
}

fn create_test_table(catalog: &Arc<Catalog>) -> Result<()> {
    let schema = Schema::new(
        "users".to_string(),
        vec![
            Column {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default: None,
            },
            Column {
                name: "email".to_string(),
                data_type: DataType::Varchar(255),
                nullable: false,
                default: None,
            },
            Column {
                name: "name".to_string(),
                data_type: DataType::Varchar(100),
                nullable: true,
                default: None,
            },
            Column {
                name: "active".to_string(),
                data_type: DataType::Boolean,
                nullable: false,
                default: Some("true".to_string()),
            },
        ],
    );
    catalog.create_table(schema)
}

#[test]
fn test_select_distinct() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "SELECT DISTINCT email FROM users";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::Select { table, columns, distinct, .. } => {
            assert_eq!(table, "users");
            assert_eq!(columns.len(), 1);
            assert_eq!(columns[0], "email");
            assert!(distinct, "DISTINCT flag should be true");
        }
        _ => panic!("Expected SELECT statement"),
    }

    Ok(())
}

#[test]
fn test_select_without_distinct() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "SELECT email, name FROM users";
    let stmts = parser.parse(sql)?;

    match &stmts[0] {
        SqlStatement::Select { distinct, .. } => {
            assert!(!distinct, "DISTINCT flag should be false");
        }
        _ => panic!("Expected SELECT statement"),
    }

    Ok(())
}

#[test]
fn test_drop_index() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "DROP INDEX idx_users_email";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::DropIndex { name } => {
            assert_eq!(name, "idx_users_email");
        }
        _ => panic!("Expected DROP INDEX statement"),
    }

    Ok(())
}

#[test]
fn test_drop_view() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "DROP VIEW active_users";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::DropView { name } => {
            assert_eq!(name, "active_users");
        }
        _ => panic!("Expected DROP VIEW statement"),
    }

    Ok(())
}

#[test]
fn test_truncate_table() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "TRUNCATE TABLE users";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::TruncateTable { name } => {
            assert_eq!(name, "users");
        }
        _ => panic!("Expected TRUNCATE TABLE statement"),
    }

    Ok(())
}

#[test]
fn test_create_index_parsing() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "CREATE INDEX idx_users_email ON users (email)";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::CreateIndex { name, table, columns, unique } => {
            assert_eq!(name, "idx_users_email");
            assert_eq!(table, "users");
            assert_eq!(columns.len(), 1);
            assert!(!unique);
        }
        _ => panic!("Expected CREATE INDEX statement"),
    }

    Ok(())
}

#[test]
fn test_create_unique_index() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "CREATE UNIQUE INDEX idx_users_email ON users (email)";
    let stmts = parser.parse(sql)?;

    match &stmts[0] {
        SqlStatement::CreateIndex { unique, .. } => {
            assert!(unique, "UNIQUE flag should be true");
        }
        _ => panic!("Expected CREATE INDEX statement"),
    }

    Ok(())
}

#[test]
fn test_create_view_parsing() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "CREATE VIEW active_users AS SELECT * FROM users WHERE active = true";
    let stmts = parser.parse(sql)?;

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        SqlStatement::CreateView { name, query } => {
            assert_eq!(name, "active_users");
            assert!(query.contains("SELECT"), "Query should contain SELECT");
        }
        _ => panic!("Expected CREATE VIEW statement"),
    }

    Ok(())
}

#[test]
fn test_executor_create_view() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();
    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager,
        index_manager,
        constraint_manager,
    );

    // Create view
    let stmt = SqlStatement::CreateView {
        name: "test_view".to_string(),
        query: "SELECT * FROM users".to_string(),
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 0);

    // Verify view was created
    let view = catalog.get_view("test_view")?;
    assert_eq!(view.name, "test_view");

    Ok(())
}

#[test]
fn test_executor_drop_view() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();

    // Create a view first
    catalog.create_view("test_view".to_string(), "SELECT * FROM users".to_string())?;

    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager,
        index_manager,
        constraint_manager,
    );

    // Drop view
    let stmt = SqlStatement::DropView {
        name: "test_view".to_string(),
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 0);

    // Verify view was dropped
    assert!(catalog.get_view("test_view").is_err());

    Ok(())
}

#[test]
fn test_executor_create_index() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();
    create_test_table(&catalog)?;

    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager,
        index_manager.clone(),
        constraint_manager,
    );

    // Create index
    let stmt = SqlStatement::CreateIndex {
        name: "idx_email".to_string(),
        table: "users".to_string(),
        columns: vec!["email".to_string()],
        unique: false,
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 0);

    // Verify index was created
    assert!(index_manager.get_index("idx_email").is_ok());

    Ok(())
}

#[test]
fn test_executor_drop_index() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();

    // Create an index first
    index_manager.create_index("test_idx".to_string(), rusty_db::index::IndexType::BTree)?;

    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager,
        index_manager.clone(),
        constraint_manager,
    );

    // Drop index
    let stmt = SqlStatement::DropIndex {
        name: "test_idx".to_string(),
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 0);

    // Verify index was dropped
    assert!(index_manager.get_index("test_idx").is_err());

    Ok(())
}

#[test]
fn test_executor_truncate_table() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();
    create_test_table(&catalog)?;

    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager,
        index_manager,
        constraint_manager,
    );

    // Truncate table
    let stmt = SqlStatement::TruncateTable {
        name: "users".to_string(),
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 0);

    Ok(())
}

#[test]
fn test_composite_index() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "CREATE INDEX idx_users_name_email ON users (name, email)";
    let stmts = parser.parse(sql)?;

    match &stmts[0] {
        SqlStatement::CreateIndex { columns, .. } => {
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0], "name");
            assert_eq!(columns[1], "email");
        }
        _ => panic!("Expected CREATE INDEX statement"),
    }

    Ok(())
}

#[test]
fn test_view_creation_and_retrieval() -> Result<()> {
    let catalog = Arc::new(Catalog::new());

    // Create a view
    catalog.create_view(
        "active_users".to_string(),
        "SELECT id, email FROM users WHERE active = true".to_string(),
    )?;

    // Retrieve the view
    let view = catalog.get_view("active_users")?;
    assert_eq!(view.name, "active_users");
    assert!(view.query.contains("WHERE active = true"));

    // List views
    let views = catalog.list_views();
    assert_eq!(views.len(), 1);
    assert!(views.contains(&"active_users".to_string()));

    Ok(())
}

#[test]
fn test_duplicate_view_error() -> Result<()> {
    let catalog = Arc::new(Catalog::new());

    // Create first view
    catalog.create_view("test_view".to_string(), "SELECT * FROM users".to_string())?;

    // Attempt to create duplicate view
    let result = catalog.create_view("test_view".to_string(), "SELECT * FROM users".to_string());
    assert!(result.is_err(), "Should not allow duplicate view names");

    Ok(())
}

#[test]
fn test_drop_nonexistent_view_error() -> Result<()> {
    let catalog = Arc::new(Catalog::new());

    // Attempt to drop non-existent view
    let result = catalog.drop_view("nonexistent");
    assert!(result.is_err(), "Should error when dropping non-existent view");

    Ok(())
}

#[test]
fn test_all_sql_operations_integration() -> Result<()> {
    let parser = SqlParser::new();

    // Test all supported SQL operations
    let operations = vec![
        "CREATE TABLE test (id INT)",
        "DROP TABLE test",
        "SELECT DISTINCT id FROM test",
        "INSERT INTO test (id) VALUES (1)",
        "UPDATE test SET id = 2",
        "DELETE FROM test WHERE id = 1",
        "CREATE INDEX idx_id ON test (id)",
        "DROP INDEX idx_id",
        "CREATE VIEW v AS SELECT * FROM test",
        "DROP VIEW v",
        "TRUNCATE TABLE test",
    ];

    for sql in operations {
        let result = parser.parse(sql);
        assert!(result.is_ok(), "Failed to parse: {}", sql);
    }

    Ok(())
}

#[test]
fn test_enterprise_constraint_validation() -> Result<()> {
    let (catalog, txn_manager, index_manager, constraint_manager) = setup_test_environment();
    create_test_table(&catalog)?;

    let executor = Executor::new_with_managers(
        catalog,
        txn_manager,
        index_manager,
        constraint_manager,
    );

    // Test INSERT with constraint validation
    let stmt = SqlStatement::Insert {
        table: "users".to_string(),
        columns: vec!["id".to_string(), "email".to_string(), "name".to_string()],
        values: vec![
            vec!["1".to_string(), "test@example.com".to_string(), "Test User".to_string()],
        ],
    };

    let result = executor.execute(stmt)?;
    assert_eq!(result.affected_rows, 1);

    Ok(())
}

#[test]
fn test_select_where_clause() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "SELECT * FROM users WHERE id > 10 AND active = true";
    let stmts = parser.parse(sql)?;

    match &stmts[0] {
        SqlStatement::Select { filter, .. } => {
            assert!(filter.is_some(), "Filter should be present");
            let filter_str = filter.as_ref().unwrap();
            assert!(filter_str.contains("id > 10"));
            assert!(filter_str.contains("active = true"));
        }
        _ => panic!("Expected SELECT statement"),
    }

    Ok(())
}

#[test]
fn test_select_group_by_having() -> Result<()> {
    let parser = SqlParser::new();
    let sql = "SELECT name, COUNT(*) FROM users GROUP BY name HAVING COUNT(*) > 5";
    let stmts = parser.parse(sql)?;

    match &stmts[0] {
        SqlStatement::Select { group_by, having, .. } => {
            assert_eq!(group_by.len(), 1);
            assert_eq!(group_by[0], "name");
            
            assert!(having.is_some());
            let having_str = having.as_ref().unwrap();
            assert!(having_str.contains("COUNT(*) > 5"));
        }
        _ => panic!("Expected SELECT statement"),
    }

    Ok(())
}
