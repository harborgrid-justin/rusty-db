use crate::error::DbError;
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Column definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<String>,
}

// Supported data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Integer,
    BigInt,
    Float,
    Double,
    Varchar(usize),
    Text,
    Boolean,
    Date,
    Timestamp,
}

// Table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Option<String>,
}

impl Schema {
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        Self {
            name,
            columns,
            primary_key: None,
        }
    }

    pub fn with_primary_key(mut self, key: String) -> Self {
        self.primary_key = Some(key);
        self
    }

    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
}

// View definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    pub query: String,
}

// Catalog manages database metadata
#[derive(Clone)]
pub struct Catalog {
    schemas: Arc<RwLock<HashMap<String, Schema>>>,
    views: Arc<RwLock<HashMap<String, View>>>,
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            views: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_table(&self, schema: Schema) -> Result<()> {
        let mut schemas = self.schemas.write();

        if schemas.contains_key(&schema.name) {
            return Err(DbError::Catalog(format!(
                "Table {} already exists",
                schema.name
            )));
        }

        schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    pub fn get_table(&self, name: &str) -> Result<Schema> {
        let schemas = self.schemas.read();

        schemas
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::Catalog(format!("Table {} not found", name)))
    }

    pub fn drop_table(&self, name: &str) -> Result<()> {
        let mut schemas = self.schemas.write();

        schemas
            .remove(name)
            .ok_or_else(|| DbError::Catalog(format!("Table {} not found", name)))?;

        Ok(())
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.schemas.read().keys().cloned().collect()
    }

    pub fn create_view(&self, name: String, query: String) -> Result<()> {
        let mut views = self.views.write();

        if views.contains_key(&name) {
            return Err(DbError::Catalog(format!("View {} already exists", name)));
        }

        views.insert(name.clone(), View { name, query });
        Ok(())
    }

    pub fn get_view(&self, name: &str) -> Result<View> {
        let views = self.views.read();

        views
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::Catalog(format!("View {} not found", name)))
    }

    pub fn drop_view(&self, name: &str) -> Result<()> {
        let mut views = self.views.write();

        views
            .remove(name)
            .ok_or_else(|| DbError::Catalog(format!("View {} not found", name)))?;

        Ok(())
    }

    pub fn list_views(&self) -> Vec<String> {
        self.views.read().keys().cloned().collect()
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog() -> Result<()> {
        let catalog = Catalog::new();

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
                    name: "name".to_string(),
                    data_type: DataType::Varchar(255),
                    nullable: false,
                    default: None,
                },
            ],
        );

        catalog.create_table(schema)?;

        let loaded = catalog.get_table("users")?;
        assert_eq!(loaded.name, "users");
        assert_eq!(loaded.columns.len(), 2);

        Ok(())
    }
}
