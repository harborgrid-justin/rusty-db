use crate::Result;
use crate::execution::QueryResult;
use crate::parser::SqlStatement;
use crate::catalog::{Catalog, Schema};
use crate::transaction::TransactionManager;
use std::sync::Arc;

/// Query executor
pub struct Executor {
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
}

impl Executor {
    pub fn new(catalog: Arc<Catalog>, txn_manager: Arc<TransactionManager>) -> Self {
        Self {
            catalog,
            txn_manager,
        }
    }
    
    pub fn execute(&self, stmt: SqlStatement) -> Result<QueryResult> {
        match stmt {
            SqlStatement::CreateTable { name, columns } => {
                let schema = Schema::new(name.clone(), columns);
                self.catalog.create_table(schema)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::DropTable { name } => {
                self.catalog.drop_table(&name)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::Select { table, columns, .. } => {
                let schema = self.catalog.get_table(&table)?;
                
                // Simple implementation - return empty result with schema
                let result_columns = if columns.contains(&"*".to_string()) {
                    schema.columns.iter().map(|c| c.name.clone()).collect()
                } else {
                    columns
                };
                
                Ok(QueryResult::new(result_columns, Vec::new()))
            }
            SqlStatement::Insert { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(1))
            }
            SqlStatement::Update { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(0))
            }
            SqlStatement::Delete { table, .. } => {
                // Validate table exists
                let _schema = self.catalog.get_table(&table)?;
                Ok(QueryResult::with_affected(0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SqlParser;
    
    #[test]
    fn test_executor() -> Result<()> {
        let catalog = Arc::new(Catalog::new());
        let txn_manager = Arc::new(TransactionManager::new());
        let executor = Executor::new(catalog, txn_manager);
        
        let parser = SqlParser::new();
        let stmts = parser.parse("CREATE TABLE users (id INT, name VARCHAR(255))")?;
        
        let result = executor.execute(stmts[0].clone())?;
        assert_eq!(result.rows_affected, 0);
        
        Ok(())
    }
}
