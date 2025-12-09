use sqlparser::ast::{Statement, SetExpr, SelectItem, Expr, TableFactor, TableWithJoins};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use crate::Result;
use crate::error::DbError;
use crate::catalog::{Column, DataType};
use crate::security::injection_prevention::InjectionPreventionGuard;

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
    },
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<String>>,
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
    },
    AlterTable {
        name: String,
        action: AlterAction,
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
    AddConstraint(String),
    DropConstraint(String),
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
            Statement::Drop { names, .. } => {
                Ok(SqlStatement::DropTable {
                    name: names[0].to_string(),
                })
            }
            Statement::Query(query) => {
                if let SetExpr::Select(select) = *query.body {
                    let table = self.extract_table_name(&select.from)?;
                    let columns = self.extract_columns(&select.projection)?;

                    Ok(SqlStatement::Select {
                        table,
                        columns,
                        filter: None,
                        join: None,
                        group_by: Vec::new(),
                        having: None,
                        order_by: Vec::new(),
                        limit: None,
                    })
                } else {
                    Err(DbError::SqlParse("Unsupported query type".to_string()))
                }
            }
            Statement::Insert { table_name, columns, .. } => {
                let table = table_name.to_string();
                let cols: Vec<String> = columns.iter().map(|c| c.to_string()).collect();

                // TODO: Parse source values from the INSERT statement
                // For now, return empty values - this will be enhanced in future versions
                Ok(SqlStatement::Insert {
                    table,
                    columns: cols,
                    values: vec![],
                })
            }
            Statement::Delete { from, .. } => {
                Ok(SqlStatement::Delete {
                    table: from[0].relation.to_string(),
                    filter: None,
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
            SqlStatement::Select { table, columns, .. } => {
                assert_eq!(table, "users");
                assert_eq!(columns.len(), 2);
            }
            _ => panic!("Expected Select"),
        }

        Ok(())
    }
}
