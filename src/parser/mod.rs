use sqlparser::ast::{Statement, SetExpr, SelectItem, Expr, TableFactor, TableWithJoins};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use crate::Result;
use crate::error::DbError;
use crate::catalog::{Column, DataType};
use crate::security::injection_prevention::InjectionPreventionGuard;

pub mod expression;
pub mod string_functions;

pub use expression::*;
pub use string_functions::*;

// Parsed SQL statement
#[derive(Debug, Clone)]
pub enum SqlStatement {
    CreateTable {
        name: String,
        columns: Vec<Column>,
    },
    DropTable {
        name: String,
    },
    Select {
        table: String,
        columns: Vec<String>,
        filter: Option<String>,
        join: Option<JoinClause>,
        group_by: Vec<String>,
        having: Option<String>,
        order_by: Vec<OrderByClause>,
        limit: Option<usize>,
        offset: Option<usize>,
        distinct: bool,
    },
    SelectInto {
        target_table: String,
        source_table: String,
        columns: Vec<String>,
        filter: Option<String>,
    },
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<String>>,
    },
    InsertIntoSelect {
        table: String,
        columns: Vec<String>,
        source_query: String,
    },
    Update {
        table: String,
        assignments: Vec<(String, String)>,
        filter: Option<String>,
    },
    Delete {
        table: String,
        filter: Option<String>,
    },
    CreateIndex {
        name: String,
        table: String,
        columns: Vec<String>,
        unique: bool,
    },
    CreateView {
        name: String,
        query: String,
        or_replace: bool,
    },
    DropView {
        name: String,
    },
    DropIndex {
        name: String,
    },
    TruncateTable {
        name: String,
    },
    AlterTable {
        name: String,
        action: AlterAction,
    },
    CreateDatabase {
        name: String,
    },
    DropDatabase {
        name: String,
    },
    BackupDatabase {
        database: String,
        path: String,
    },
    CreateProcedure {
        name: String,
        parameters: Vec<(String, DataType)>,
        body: String,
    },
    ExecProcedure {
        name: String,
        arguments: Vec<String>,
    },
    Union {
        left: Box<SqlStatement>,
        right: Box<SqlStatement>,
        all: bool,
    },
    GrantPermission {
        permission: String,
        table: String,
        user: String,
    },
    RevokePermission {
        permission: String,
        table: String,
        user: String,
    },
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub column: String,
    pub ascending: bool,
}

#[derive(Debug, Clone)]
pub enum AlterAction {
    AddColumn(Column),
    DropColumn(String),
    AlterColumn {
        column_name: String,
        new_type: DataType,
    },
    ModifyColumn {
        column_name: String,
        new_type: DataType,
        nullable: Option<bool>,
    },
    AddConstraint(ConstraintType),
    DropConstraint(String),
    DropDefault(String),
}

#[derive(Debug, Clone)]
pub enum ConstraintType {
    PrimaryKey(Vec<String>),
    ForeignKey {
        columns: Vec<String>,
        ref_table: String,
        ref_columns: Vec<String>,
    },
    Unique(Vec<String>),
    Check(String),
    Default {
        column: String,
        value: String,
    },
}

// SQL parser wrapper with integrated injection prevention
pub struct SqlParser {
    dialect: GenericDialect,
    injection_guard: InjectionPreventionGuard,
}

impl SqlParser {
    pub fn new() -> Self {
        Self {
            dialect: GenericDialect {},
            injection_guard: InjectionPreventionGuard::new(),
        }
    }

    pub fn parse(&self, sql: &str) -> Result<Vec<SqlStatement>> {
        // LAYER 1-6: Multi-layer injection prevention
        // This validates and sanitizes the input through:
        // - Input sanitization (Unicode normalization, homograph detection)
        // - Dangerous pattern detection (SQL keywords, comments, tautologies)
        // - Syntax validation (quotes, parentheses, identifiers)
        // - Escape validation
        // - Whitelist validation
        let safe_sql = self.injection_guard.validate_and_sanitize(sql)?;

        // Parse the now-safe SQL
        let ast = Parser::parse_sql(&self.dialect, &safe_sql)
            .map_err(|e| DbError::SqlParse(e.to_string()))?;

        let mut statements = Vec::new();

        for stmt in ast {
            statements.push(self.convert_statement(stmt)?);
        }

        Ok(statements)
    }

    fn convert_statement(&self, stmt: Statement) -> Result<SqlStatement> {
        match stmt {
            Statement::CreateTable { name, columns, .. } => {
                let table_name = name.to_string();
                let mut cols = Vec::new();

                for col in columns {
                    let data_type = match col.data_type {
                        sqlparser::ast::DataType::Int(_) => DataType::Integer,
                        sqlparser::ast::DataType::BigInt(_) => DataType::BigInt,
                        sqlparser::ast::DataType::Float(_) => DataType::Float,
                        sqlparser::ast::DataType::Double => DataType::Double,
                        sqlparser::ast::DataType::Varchar(len) => {
                            let size = len.map(|l| match l {
                                sqlparser::ast::CharacterLength::IntegerLength { length, .. } => length as usize,
                                _ => 255,
                            }).unwrap_or(255);
                            DataType::Varchar(size)
                        }
                        sqlparser::ast::DataType::Text => DataType::Text,
                        sqlparser::ast::DataType::Boolean => DataType::Boolean,
                        sqlparser::ast::DataType::Date => DataType::Date,
                        sqlparser::ast::DataType::Timestamp(_, _) => DataType::Timestamp,
                        _ => DataType::Text,
                    };

                    cols.push(Column {
                        name: col.name.to_string(),
                        data_type,
                        nullable: col.options.iter().any(|opt| {
                            matches!(opt.option, sqlparser::ast::ColumnOption::Null)
                        }),
                        default: None,
                    });
                }

                Ok(SqlStatement::CreateTable {
                    name: table_name,
                    columns: cols,
                })
            }
            Statement::Drop { names, object_type, .. } => {
                use sqlparser::ast::ObjectType;
                match object_type {
                    ObjectType::Table => {
                        Ok(SqlStatement::DropTable {
                            name: names[0].to_string(),
                        })
                    }
                    ObjectType::View => {
                        Ok(SqlStatement::DropView {
                            name: names[0].to_string(),
                        })
                    }
                    ObjectType::Index => {
                        Ok(SqlStatement::DropIndex {
                            name: names[0].to_string(),
                        })
                    }
                    _ => Err(DbError::SqlParse(format!("Unsupported DROP object type: {:?}", object_type))),
                }
            }
            Statement::Query(query) => {
                if let SetExpr::Select(select) = *query.body {
                    let table = self.extract_table_name(&select.from)?;
                    let columns = self.extract_columns(&select.projection)?;
                    let distinct = select.distinct.is_some();

                    Ok(SqlStatement::Select {
                        table,
                        columns,
                        filter: None,
                        join: None,
                        group_by: Vec::new(),
                        having: None,
                        order_by: Vec::new(),
                        limit: None,
                        offset: None,
                        distinct,
                    })
                } else {
                    Err(DbError::SqlParse("Unsupported query type".to_string()))
                }
            }
            Statement::Insert { table_name, columns, source, .. } => {
                let table = table_name.to_string();
                let cols: Vec<String> = columns.iter().map(|c| c.to_string()).collect();

                // Parse source values from the INSERT statement
                let mut values: Vec<Vec<String>> = Vec::new();

                if let Some(src) = source {
                    if let SetExpr::Values(vals) = *src.body {
                        for row in vals.rows {
                            let mut row_values = Vec::new();
                            for expr in row {
                                let value = self.extract_literal_value(&expr);
                                row_values.push(value);
                            }
                            values.push(row_values);
                        }
                    }
                }

                Ok(SqlStatement::Insert {
                    table,
                    columns: cols,
                    values,
                })
            }
            Statement::Delete { from, .. } => {
                Ok(SqlStatement::Delete {
                    table: from[0].relation.to_string(),
                    filter: None,
                })
            }
            Statement::Truncate { table_name, .. } => {
                Ok(SqlStatement::TruncateTable {
                    name: table_name.to_string(),
                })
            }
            Statement::CreateIndex { name, table_name, columns, unique, .. } => {
                let index_name = name.as_ref()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("idx_{}", table_name.to_string()));
                let table = table_name.to_string();
                let cols: Vec<String> = columns.iter()
                    .map(|col| col.expr.to_string())
                    .collect();

                Ok(SqlStatement::CreateIndex {
                    name: index_name,
                    table,
                    columns: cols,
                    unique,
                })
            }
            Statement::CreateView { name, query, .. } => {
                Ok(SqlStatement::CreateView {
                    name: name.to_string(),
                    query: query.to_string(),
                    or_replace: false,
                })
            }
            _ => Err(DbError::SqlParse("Unsupported statement type".to_string())),
        }
    }

    fn extract_table_name(&self, from: &[TableWithJoins]) -> Result<String> {
        if from.is_empty() {
            return Err(DbError::SqlParse("No table specified".to_string()));
        }

        match &from[0].relation {
            TableFactor::Table { name, .. } => Ok(name.to_string()),
            _ => Err(DbError::SqlParse("Unsupported table factor".to_string())),
        }
    }

    fn extract_columns(&self, projection: &[SelectItem]) -> Result<Vec<String>> {
        let mut columns = Vec::new();

        for item in projection {
            match item {
                SelectItem::UnnamedExpr(Expr::Identifier(ident)) => {
                    columns.push(ident.to_string());
                }
                SelectItem::Wildcard(_) => {
                    columns.push("*".to_string());
                }
                _ => {}
            }
        }

        Ok(columns)
    }

    /// Extract a literal value from an expression
    fn extract_literal_value(&self, expr: &Expr) -> String {
        match expr {
            Expr::Value(val) => match val {
                sqlparser::ast::Value::Number(n, _) => n.clone(),
                sqlparser::ast::Value::SingleQuotedString(s) => s.clone(),
                sqlparser::ast::Value::DoubleQuotedString(s) => s.clone(),
                sqlparser::ast::Value::Boolean(b) => b.to_string(),
                sqlparser::ast::Value::Null => "NULL".to_string(),
                _ => val.to_string(),
            },
            Expr::Identifier(ident) => ident.to_string(),
            Expr::UnaryOp { op, expr } => {
                format!("{}{}", op, self.extract_literal_value(expr))
            },
            _ => expr.to_string(),
        }
    }
}

impl Default for SqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_table() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "CREATE TABLE users (id INT, name VARCHAR(255))";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::CreateTable { name, columns } => {
                assert_eq!(name, "users");
                assert_eq!(columns.len(), 2);
            }
            _ => panic!("Expected CreateTable"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_select() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "SELECT id, name FROM users";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::Select { table, columns, distinct, .. } => {
                assert_eq!(table, "users");
                assert_eq!(columns.len(), 2);
                assert!(!distinct);
            }
            _ => panic!("Expected Select"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_select_distinct() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "SELECT DISTINCT id FROM users";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::Select { table, columns, distinct, .. } => {
                assert_eq!(table, "users");
                assert_eq!(columns.len(), 1);
                assert!(distinct);
            }
            _ => panic!("Expected Select with DISTINCT"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_drop_index() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "DROP INDEX idx_users_email";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::DropIndex { name } => {
                assert_eq!(name, "idx_users_email");
            }
            _ => panic!("Expected DropIndex"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_drop_view() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "DROP VIEW active_users";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::DropView { name } => {
                assert_eq!(name, "active_users");
            }
            _ => panic!("Expected DropView"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_truncate_table() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "TRUNCATE TABLE users";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::TruncateTable { name } => {
                assert_eq!(name, "users");
            }
            _ => panic!("Expected TruncateTable"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_create_index() -> Result<()> {
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
            _ => panic!("Expected CreateIndex"),
        }

        Ok(())
    }

    #[test]
    fn test_parse_create_view() -> Result<()> {
        let parser = SqlParser::new();
        let sql = "CREATE VIEW active_users AS SELECT * FROM users WHERE active = true";
        let stmts = parser.parse(sql)?;

        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            SqlStatement::CreateView { name, query } => {
                assert_eq!(name, "active_users");
                assert!(query.contains("SELECT"));
            }
            _ => panic!("Expected CreateView"),
        }

        Ok(())
    }
}
