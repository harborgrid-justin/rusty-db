/// PL/SQL-style Language Parser for RustyDB Stored Procedures
///
/// This module provides a comprehensive parser for PL/SQL-compatible procedural language,
/// supporting Oracle-style syntax including control flow, exception handling, and variable declarations.

use crate::{Result, DbError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete PL/SQL block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlSqlBlock {
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
    pub exception_handlers: Vec<ExceptionHandler>,
}

/// Variable or constant declaration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub data_type: PlSqlType,
    pub is_constant: bool,
    pub initial_value: Option<Expression>,
    pub not_null: bool,
}

/// PL/SQL data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlSqlType {
    Integer,
    Number { precision: Option<u8>, scale: Option<u8> },
    Varchar2(usize),
    Char(usize),
    Date,
    Timestamp,
    Boolean,
    Clob,
    Blob,
    RowType { table: String },
    RecordType { fields: Vec<(String, PlSqlType)> },
    TableType { element_type: Box<PlSqlType> },
    RefCursor,
}

/// Statements that can appear in PL/SQL blocks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Statement {
    /// Assignment statement: variable := expression
    Assignment {
        target: String,
        value: Expression,
    },
    /// SQL SELECT INTO statement
    SelectInto {
        columns: Vec<String>,
        into_vars: Vec<String>,
        from: String,
        where_clause: Option<Expression>,
    },
    /// SQL INSERT statement
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Expression>,
    },
    /// SQL UPDATE statement
    Update {
        table: String,
        assignments: Vec<(String, Expression)>,
        where_clause: Option<Expression>,
    },
    /// SQL DELETE statement
    Delete {
        table: String,
        where_clause: Option<Expression>,
    },
    /// IF-THEN-ELSIF-ELSE control structure
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        elsif_blocks: Vec<(Expression, Vec<Statement>)>,
        else_block: Option<Vec<Statement>>,
    },
    /// Simple LOOP...END LOOP
    Loop {
        statements: Vec<Statement>,
    },
    /// WHILE loop
    While {
        condition: Expression,
        statements: Vec<Statement>,
    },
    /// FOR loop (numeric)
    ForNumeric {
        iterator: String,
        reverse: bool,
        start: Expression,
        end: Expression,
        statements: Vec<Statement>,
    },
    /// FOR loop (cursor)
    ForCursor {
        record: String,
        cursor: String,
        statements: Vec<Statement>,
    },
    /// EXIT statement (with optional WHEN condition)
    Exit {
        when: Option<Expression>,
    },
    /// CONTINUE statement (with optional WHEN condition)
    Continue {
        when: Option<Expression>,
    },
    /// RETURN statement
    Return {
        value: Option<Expression>,
    },
    /// RAISE exception
    Raise {
        exception: String,
    },
    /// COMMIT transaction
    Commit,
    /// ROLLBACK transaction
    Rollback {
        to_savepoint: Option<String>,
    },
    /// SAVEPOINT
    Savepoint {
        name: String,
    },
    /// Procedure or function call
    Call {
        name: String,
        arguments: Vec<Expression>,
    },
    /// NULL statement (no-op)
    Null,
    /// Open cursor
    OpenCursor {
        cursor: String,
        arguments: Vec<Expression>,
    },
    /// Fetch from cursor
    FetchCursor {
        cursor: String,
        into_vars: Vec<String>,
    },
    /// Close cursor
    CloseCursor {
        cursor: String,
    },
    /// CASE statement
    Case {
        selector: Option<Expression>,
        when_clauses: Vec<(Expression, Vec<Statement>)>,
        else_clause: Option<Vec<Statement>>,
    },
}

/// Expressions used in PL/SQL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Expression {
    /// Literal value
    Literal(LiteralValue),
    /// Variable reference
    Variable(String),
    /// Binary operation
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
    /// Function call
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },
    /// SQL aggregate function
    Aggregate {
        function: AggregateFunction,
        argument: Box<Expression>,
    },
    /// CASE expression
    CaseExpr {
        selector: Option<Box<Expression>>,
        when_clauses: Vec<(Expression, Expression)>,
        else_clause: Option<Box<Expression>>,
    },
    /// Subquery
    Subquery {
        query: String,
    },
    /// Record field access (e.g., employee.salary)
    FieldAccess {
        record: String,
        field: String,
    },
    /// Collection element access (e.g., array(i))
    CollectionAccess {
        collection: String,
        index: Box<Expression>,
    },
}

/// Literal values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Date(String),
    Timestamp(String),
}

/// Binary operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    // Logical
    And,
    Or,
    // String
    Concat,
    Like,
    // Set operations
    In,
    NotIn,
}

/// Unary operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
}

/// Aggregate functions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
}

/// Exception handler
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExceptionHandler {
    pub exception_type: ExceptionType,
    pub statements: Vec<Statement>,
}

/// Exception types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExceptionType {
    /// NO_DATA_FOUND
    NoDataFound,
    /// TOO_MANY_ROWS
    TooManyRows,
    /// ZERO_DIVIDE
    ZeroDivide,
    /// VALUE_ERROR
    ValueError,
    /// INVALID_CURSOR
    InvalidCursor,
    /// DUP_VAL_ON_INDEX
    DupValOnIndex,
    /// User-defined exception
    UserDefined(String),
    /// OTHERS (catch-all)
    Others,
}

/// PL/SQL Parser
pub struct PlSqlParser {
    tokens: Vec<Token>,
    current: usize,
}

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Keywords
    Begin,
    End,
    Declare,
    If,
    Then,
    Elsif,
    Else,
    Loop,
    While,
    For,
    Exit,
    Continue,
    When,
    Return,
    Raise,
    Exception,
    Is,
    As,
    In,
    Out,
    InOut,
    Constant,
    NotNull,
    Default,
    Commit,
    Rollback,
    Savepoint,
    Open,
    Fetch,
    Close,
    Into,
    From,
    Where,
    Select,
    Insert,
    Update,
    Delete,
    Values,
    Set,
    Case,
    Null,
    Reverse,
    To,
    Cursor,

    // Operators
    Assign,           // :=
    Equal,            // =
    NotEqual,         // !=, <>
    LessThan,         // <
    LessThanOrEqual,  // <=
    GreaterThan,      // >
    GreaterThanOrEqual, // >=
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Concat,           // ||

    // Logical
    And,
    Or,
    Not,
    Like,

    // Delimiters
    LeftParen,
    RightParen,
    Semicolon,
    Comma,
    Dot,

    // Literals
    IntegerLit(i64),
    FloatLit(f64),
    StringLit(String),
    BooleanLit(bool),

    // Identifiers
    Identifier(String),

    // End of input
    Eof,
}

impl PlSqlParser {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current: 0,
        }
    }

    /// Parse a PL/SQL block from source text
    pub fn parse(&mut self, source: &str) -> Result<PlSqlBlock> {
        self.tokenize(source)?;
        self.parse_block()
    }

    /// Tokenize the source code
    fn tokenize(&mut self, source: &str) -> Result<()> {
        let mut chars: Vec<char> = source.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Skip whitespace
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            // Skip comments
            if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                // Single-line comment
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                // Multi-line comment
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            // String literals
            if chars[i] == '\'' {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                let string_val: String = chars[start..i].iter().collect();
                self.tokens.push(Token::StringLit(string_val));
                i += 1;
                continue;
            }

            // Numbers
            if chars[i].is_ascii_digit() {
                let start = i;
                let mut is_float = false;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    if chars[i] == '.' {
                        is_float = true;
                    }
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                if is_float {
                    let val: f64 = num_str.parse().map_err(|_|
                        DbError::SqlParse(format!("Invalid float: {}", num_str)))?);
                    self.tokens.push(Token::FloatLit(val));
                } else {
                    let val: i64 = num_str.parse().map_err(|_|
                        DbError::SqlParse(format!("Invalid integer: {}", num_str)))?);
                    self.tokens.push(Token::IntegerLit(val));
                }
                continue;
            }

            // Identifiers and keywords
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                let ident_upper = ident.to_uppercase();

                let token = match ident_upper.as_str() {
                    "BEGIN" => Token::Begin,
                    "END" => Token::End,
                    "DECLARE" => Token::Declare,
                    "IF" => Token::If,
                    "THEN" => Token::Then,
                    "ELSIF" => Token::Elsif,
                    "ELSE" => Token::Else,
                    "LOOP" => Token::Loop,
                    "WHILE" => Token::While,
                    "FOR" => Token::For,
                    "EXIT" => Token::Exit,
                    "CONTINUE" => Token::Continue,
                    "WHEN" => Token::When,
                    "RETURN" => Token::Return,
                    "RAISE" => Token::Raise,
                    "EXCEPTION" => Token::Exception,
                    "IS" => Token::Is,
                    "AS" => Token::As,
                    "IN" => Token::In,
                    "OUT" => Token::Out,
                    "INOUT" => Token::InOut,
                    "CONSTANT" => Token::Constant,
                    "NULL" => Token::Null,
                    "DEFAULT" => Token::Default,
                    "COMMIT" => Token::Commit,
                    "ROLLBACK" => Token::Rollback,
                    "SAVEPOINT" => Token::Savepoint,
                    "OPEN" => Token::Open,
                    "FETCH" => Token::Fetch,
                    "CLOSE" => Token::Close,
                    "INTO" => Token::Into,
                    "FROM" => Token::From,
                    "WHERE" => Token::Where,
                    "SELECT" => Token::Select,
                    "INSERT" => Token::Insert,
                    "UPDATE" => Token::Update,
                    "DELETE" => Token::Delete,
                    "VALUES" => Token::Values,
                    "SET" => Token::Set,
                    "CASE" => Token::Case,
                    "AND" => Token::And,
                    "OR" => Token::Or,
                    "NOT" => Token::Not,
                    "LIKE" => Token::Like,
                    "TRUE" => Token::BooleanLit(true),
                    "FALSE" => Token::BooleanLit(false),
                    "REVERSE" => Token::Reverse,
                    "TO" => Token::To,
                    "CURSOR" => Token::Cursor,
                    _ => Token::Identifier(ident),
                };
                self.tokens.push(token);
                continue;
            }

            // Operators and delimiters
            match chars[i] {
                '(' => {
                    self.tokens.push(Token::LeftParen);
                    i += 1;
                }
                ')' => {
                    self.tokens.push(Token::RightParen);
                    i += 1;
                }
                ';' => {
                    self.tokens.push(Token::Semicolon);
                    i += 1;
                }
                ',' => {
                    self.tokens.push(Token::Comma);
                    i += 1;
                }
                '.' => {
                    self.tokens.push(Token::Dot);
                    i += 1;
                }
                '+' => {
                    self.tokens.push(Token::Plus);
                    i += 1;
                }
                '-' => {
                    self.tokens.push(Token::Minus);
                    i += 1;
                }
                '*' => {
                    self.tokens.push(Token::Star);
                    i += 1;
                }
                '/' => {
                    self.tokens.push(Token::Slash);
                    i += 1;
                }
                '%' => {
                    self.tokens.push(Token::Percent);
                    i += 1;
                }
                ':' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(Token::Assign);
                        i += 2;
                    } else {
                        return Err(DbError::SqlParse("Unexpected character: :".to_string()));
                    }
                }
                '=' => {
                    self.tokens.push(Token::Equal);
                    i += 1;
                }
                '<' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(Token::LessThanOrEqual);
                        i += 2;
                    } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                        self.tokens.push(Token::NotEqual);
                        i += 2;
                    } else {
                        self.tokens.push(Token::LessThan);
                        i += 1;
                    }
                }
                '>' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(Token::GreaterThanOrEqual);
                        i += 2;
                    } else {
                        self.tokens.push(Token::GreaterThan);
                        i += 1;
                    }
                }
                '!' => {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        self.tokens.push(Token::NotEqual);
                        i += 2;
                    } else {
                        return Err(DbError::SqlParse("Unexpected character: !".to_string()));
                    }
                }
                '|' => {
                    if i + 1 < chars.len() && chars[i + 1] == '|' {
                        self.tokens.push(Token::Concat);
                        i += 2;
                    } else {
                        return Err(DbError::SqlParse("Unexpected character: |".to_string()));
                    }
                }
                _ => {
                    return Err(DbError::SqlParse(format!("Unexpected character: {}", chars[i]))));
                }
            }
        }

        self.tokens.push(Token::Eof);
        Ok(())
    }

    /// Parse a complete block (DECLARE...BEGIN...EXCEPTION...END)
    fn parse_block(&mut self) -> Result<PlSqlBlock> {
        let mut declarations = Vec::new();
        let mut statements = Vec::new();
        let mut exception_handlers = Vec::new();

        // Optional DECLARE section
        if self.match_token(&Token::Declare) {
            while !self.check(&Token::Begin) && !self.check(&Token::Eof) {
                declarations.push(self.parse_declaration()?);
            }
        }

        // BEGIN section
        self.consume(&Token::Begin, "Expected BEGIN")?;

        while !self.check(&Token::Exception) && !self.check(&Token::End) && !self.check(&Token::Eof) {
            statements.push(self.parse_statement()?);
        }

        // Optional EXCEPTION section
        if self.match_token(&Token::Exception) {
            while !self.check(&Token::End) && !self.check(&Token::Eof) {
                exception_handlers.push(self.parse_exception_handler()?);
            }
        }

        self.consume(&Token::End, "Expected END")?;
        self.consume(&Token::Semicolon, "Expected semicolon after END")?;

        Ok(PlSqlBlock {
            declarations,
            statements,
            exception_handlers,
        })
    }

    /// Parse a variable declaration
    fn parse_declaration(&mut self) -> Result<Declaration> {
        let name = self.consume_identifier("Expected variable name")?;

        let is_constant = self.match_token(&Token::Constant);

        let data_type = self.parse_type()?;

        let not_null = if self.match_token(&Token::Not) {
            self.consume(&Token::Null, "Expected NULL after NOT")?;
            true
        } else {
            false
        };

        let initial_value = if self.match_token(&Token::Assign) || self.match_token(&Token::Default) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon after declaration")?;

        Ok(Declaration {
            name,
            data_type,
            is_constant,
            initial_value,
            not_null,
        })
    }

    /// Parse a data type
    fn parse_type(&mut self) -> Result<PlSqlType> {
        let type_name = self.consume_identifier("Expected type name")?;

        match type_name.to_uppercase().as_str() {
            "INTEGER" | "INT" => Ok(PlSqlType::Integer),
            "NUMBER" | "NUMERIC" => {
                if self.match_token(&Token::LeftParen) {
                    let precision = self.consume_integer("Expected precision")?;
                    let scale = if self.match_token(&Token::Comma) {
                        Some(self.consume_integer("Expected scale")?)
                    } else {
                        None
                    };
                    self.consume(&Token::RightParen, "Expected ')'")?;
                    Ok(PlSqlType::Number {
                        precision: Some(precision as u8),
                        scale: scale.map(|s| s as u8),
                    })
                } else {
                    Ok(PlSqlType::Number {
                        precision: None,
                        scale: None,
                    })
                }
            }
            "VARCHAR2" | "VARCHAR" => {
                self.consume(&Token::LeftParen, "Expected '(' after VARCHAR2")?;
                let size = self.consume_integer("Expected size")?;
                self.consume(&Token::RightParen, "Expected ')'")?;
                Ok(PlSqlType::Varchar2(size as usize))
            }
            "CHAR" => {
                self.consume(&Token::LeftParen, "Expected '(' after CHAR")?;
                let size = self.consume_integer("Expected size")?;
                self.consume(&Token::RightParen, "Expected ')'")?;
                Ok(PlSqlType::Char(size as usize))
            }
            "DATE" => Ok(PlSqlType::Date),
            "TIMESTAMP" => Ok(PlSqlType::Timestamp),
            "BOOLEAN" => Ok(PlSqlType::Boolean),
            "CLOB" => Ok(PlSqlType::Clob),
            "BLOB" => Ok(PlSqlType::Blob),
            _ => Err(DbError::SqlParse(format!("Unknown type: {}", type_name))),
        }
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Statement> {
        if self.check(&Token::If) {
            self.parse_if_statement()
        } else if self.check(&Token::Loop) {
            self.parse_loop_statement()
        } else if self.check(&Token::While) {
            self.parse_while_statement()
        } else if self.check(&Token::For) {
            self.parse_for_statement()
        } else if self.check(&Token::Exit) {
            self.parse_exit_statement()
        } else if self.check(&Token::Continue) {
            self.parse_continue_statement()
        } else if self.check(&Token::Return) {
            self.parse_return_statement()
        } else if self.check(&Token::Raise) {
            self.parse_raise_statement()
        } else if self.check(&Token::Commit) {
            self.advance());
            self.consume(&Token::Semicolon, "Expected semicolon")?;
            Ok(Statement::Commit)
        } else if self.check(&Token::Rollback) {
            self.parse_rollback_statement()
        } else if self.check(&Token::Savepoint) {
            self.parse_savepoint_statement()
        } else if self.check(&Token::Select) {
            self.parse_select_into_statement()
        } else if self.check(&Token::Insert) {
            self.parse_insert_statement()
        } else if self.check(&Token::Update) {
            self.parse_update_statement()
        } else if self.check(&Token::Delete) {
            self.parse_delete_statement()
        } else if self.check(&Token::Open) {
            self.parse_open_cursor_statement()
        } else if self.check(&Token::Fetch) {
            self.parse_fetch_cursor_statement()
        } else if self.check(&Token::Close) {
            self.parse_close_cursor_statement()
        } else if self.check(&Token::Case) {
            self.parse_case_statement()
        } else if self.check(&Token::Null) {
            self.advance();
            self.consume(&Token::Semicolon, "Expected semicolon")?;
            Ok(Statement::Null)
        } else if let Token::Identifier(_) = self.peek() {
            self.parse_assignment_or_call()
        } else {
            Err(DbError::SqlParse(format!("Unexpected token: {:?}", self.peek())))
        }
    }

    /// Parse IF statement
    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::If, "Expected IF")?);
        let condition = self.parse_expression()?;
        self.consume(&Token::Then, "Expected THEN")?;

        let mut then_block = Vec::new();
        while !self.check(&Token::Elsif) && !self.check(&Token::Else) && !self.check(&Token::End) {
            then_block.push(self.parse_statement()?);
        }

        let mut elsif_blocks = Vec::new();
        while self.match_token(&Token::Elsif) {
            let elsif_cond = self.parse_expression()?;
            self.consume(&Token::Then, "Expected THEN")?;
            let mut elsif_stmts = Vec::new();
            while !self.check(&Token::Elsif) && !self.check(&Token::Else) && !self.check(&Token::End) {
                elsif_stmts.push(self.parse_statement()?);
            }
            elsif_blocks.push((elsif_cond, elsif_stmts));
        }

        let else_block = if self.match_token(&Token::Else) {
            let mut else_stmts = Vec::new();
            while !self.check(&Token::End) {
                else_stmts.push(self.parse_statement()?);
            }
            Some(else_stmts)
        } else {
            None
        };

        self.consume(&Token::End, "Expected END")?;
        self.consume(&Token::If, "Expected IF")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::If {
            condition,
            then_block,
            elsif_blocks,
            else_block,
        })
    }

    /// Parse LOOP statement
    fn parse_loop_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Loop, "Expected LOOP")?;

        let mut statements = Vec::new();
        while !self.check(&Token::End) {
            statements.push(self.parse_statement()?);
        }

        self.consume(&Token::End, "Expected END")?;
        self.consume(&Token::Loop, "Expected LOOP")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Loop { statements })
    }

    /// Parse WHILE loop
    fn parse_while_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::While, "Expected WHILE")?;
        let condition = self.parse_expression()?;
        self.consume(&Token::Loop, "Expected LOOP")?;

        let mut statements = Vec::new();
        while !self.check(&Token::End) {
            statements.push(self.parse_statement()?);
        }

        self.consume(&Token::End, "Expected END")?;
        self.consume(&Token::Loop, "Expected LOOP")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::While {
            condition,
            statements,
        })
    }

    /// Parse FOR loop
    fn parse_for_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::For, "Expected FOR")?;
        let iterator = self.consume_identifier("Expected iterator name")?;
        self.consume(&Token::In, "Expected IN")?;

        // Check if it's a cursor loop or numeric loop
        if self.check(&Token::LeftParen) || self.check_identifier() {
            // Could be cursor loop
            let cursor = self.consume_identifier("Expected cursor name")?;
            self.consume(&Token::Loop, "Expected LOOP")?;

            let mut statements = Vec::new();
            while !self.check(&Token::End) {
                statements.push(self.parse_statement()?);
            }

            self.consume(&Token::End, "Expected END")?;
            self.consume(&Token::Loop, "Expected LOOP")?;
            self.consume(&Token::Semicolon, "Expected semicolon")?;

            Ok(Statement::ForCursor {
                record: iterator,
                cursor,
                statements,
            })
        } else {
            // Numeric loop
            let reverse = self.match_token(&Token::Reverse);
            let start = self.parse_expression()?;
            self.consume(&Token::Dot, "Expected ..")?;
            self.consume(&Token::Dot, "Expected ..")?;
            let end = self.parse_expression()?;
            self.consume(&Token::Loop, "Expected LOOP")?;

            let mut statements = Vec::new();
            while !self.check(&Token::End) {
                statements.push(self.parse_statement()?);
            }

            self.consume(&Token::End, "Expected END")?;
            self.consume(&Token::Loop, "Expected LOOP")?;
            self.consume(&Token::Semicolon, "Expected semicolon")?;

            Ok(Statement::ForNumeric {
                iterator,
                reverse,
                start,
                end,
                statements,
            })
        }
    }

    /// Parse EXIT statement
    fn parse_exit_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Exit, "Expected EXIT")?;

        let when = if self.match_token(&Token::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Exit { when })
    }

    /// Parse CONTINUE statement
    fn parse_continue_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Continue, "Expected CONTINUE")?;

        let when = if self.match_token(&Token::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Continue { when })
    }

    /// Parse RETURN statement
    fn parse_return_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Return, "Expected RETURN")?;

        let value = if !self.check(&Token::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Return { value })
    }

    /// Parse RAISE statement
    fn parse_raise_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Raise, "Expected RAISE")?;
        let exception = self.consume_identifier("Expected exception name")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Raise { exception })
    }

    /// Parse ROLLBACK statement
    fn parse_rollback_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Rollback, "Expected ROLLBACK")?;

        let to_savepoint = if self.match_token(&Token::To) {
            Some(self.consume_identifier("Expected savepoint name")?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Rollback { to_savepoint })
    }

    /// Parse SAVEPOINT statement
    fn parse_savepoint_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Savepoint, "Expected SAVEPOINT")?;
        let name = self.consume_identifier("Expected savepoint name")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Savepoint { name })
    }

    /// Parse SELECT INTO statement
    fn parse_select_into_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Select, "Expected SELECT")?;

        let mut columns = Vec::new();
        loop {
            columns.push(self.consume_identifier("Expected column name")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.consume(&Token::Into, "Expected INTO")?;

        let mut into_vars = Vec::new();
        loop {
            into_vars.push(self.consume_identifier("Expected variable name")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.consume(&Token::From, "Expected FROM")?;
        let from = self.consume_identifier("Expected table name")?;

        let where_clause = if self.match_token(&Token::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::SelectInto {
            columns,
            into_vars,
            from,
            where_clause,
        })
    }

    /// Parse INSERT statement
    fn parse_insert_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Insert, "Expected INSERT")?;
        self.consume(&Token::Into, "Expected INTO")?;
        let table = self.consume_identifier("Expected table name")?;

        self.consume(&Token::LeftParen, "Expected '('")?;
        let mut columns = Vec::new();
        loop {
            columns.push(self.consume_identifier("Expected column name")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.consume(&Token::RightParen, "Expected ')'")?;

        self.consume(&Token::Values, "Expected VALUES")?;
        self.consume(&Token::LeftParen, "Expected '('")?;
        let mut values = Vec::new();
        loop {
            values.push(self.parse_expression()?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.consume(&Token::RightParen, "Expected ')'")?;

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Insert {
            table,
            columns,
            values,
        })
    }

    /// Parse UPDATE statement
    fn parse_update_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Update, "Expected UPDATE")?;
        let table = self.consume_identifier("Expected table name")?;

        self.consume(&Token::Set, "Expected SET")?;

        let mut assignments = Vec::new();
        loop {
            let column = self.consume_identifier("Expected column name")?;
            self.consume(&Token::Equal, "Expected '='")?;
            let value = self.parse_expression()?;
            assignments.push((column, value));

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        let where_clause = if self.match_token(&Token::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Update {
            table,
            assignments,
            where_clause,
        })
    }

    /// Parse DELETE statement
    fn parse_delete_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Delete, "Expected DELETE")?;
        self.consume(&Token::From, "Expected FROM")?;
        let table = self.consume_identifier("Expected table name")?;

        let where_clause = if self.match_token(&Token::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Delete {
            table,
            where_clause,
        })
    }

    /// Parse OPEN cursor statement
    fn parse_open_cursor_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Open, "Expected OPEN")?;
        let cursor = self.consume_identifier("Expected cursor name")?;

        let mut arguments = Vec::new();
        if self.match_token(&Token::LeftParen) {
            if !self.check(&Token::RightParen) {
                loop {
                    arguments.push(self.parse_expression()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }
            self.consume(&Token::RightParen, "Expected ')'")?;
        }

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::OpenCursor { cursor, arguments })
    }

    /// Parse FETCH cursor statement
    fn parse_fetch_cursor_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Fetch, "Expected FETCH")?;
        let cursor = self.consume_identifier("Expected cursor name")?;
        self.consume(&Token::Into, "Expected INTO")?;

        let mut into_vars = Vec::new();
        loop {
            into_vars.push(self.consume_identifier("Expected variable name")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::FetchCursor { cursor, into_vars })
    }

    /// Parse CLOSE cursor statement
    fn parse_close_cursor_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Close, "Expected CLOSE")?;
        let cursor = self.consume_identifier("Expected cursor name")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::CloseCursor { cursor })
    }

    /// Parse CASE statement
    fn parse_case_statement(&mut self) -> Result<Statement> {
        self.consume(&Token::Case, "Expected CASE")?;

        let selector = if !self.check(&Token::When) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let mut when_clauses = Vec::new();
        while self.match_token(&Token::When) {
            let condition = self.parse_expression()?;
            self.consume(&Token::Then, "Expected THEN")?;

            let mut statements = Vec::new();
            while !self.check(&Token::When) && !self.check(&Token::Else) && !self.check(&Token::End) {
                statements.push(self.parse_statement()?);
            }

            when_clauses.push((condition, statements));
        }

        let else_clause = if self.match_token(&Token::Else) {
            let mut statements = Vec::new();
            while !self.check(&Token::End) {
                statements.push(self.parse_statement()?);
            }
            Some(statements)
        } else {
            None
        };

        self.consume(&Token::End, "Expected END")?;
        self.consume(&Token::Case, "Expected CASE")?;
        self.consume(&Token::Semicolon, "Expected semicolon")?;

        Ok(Statement::Case {
            selector,
            when_clauses,
            else_clause,
        })
    }

    /// Parse assignment or procedure call
    fn parse_assignment_or_call(&mut self) -> Result<Statement> {
        let name = self.consume_identifier("Expected identifier")?;

        if self.match_token(&Token::Assign) {
            // Assignment
            let value = self.parse_expression()?;
            self.consume(&Token::Semicolon, "Expected semicolon")?;
            Ok(Statement::Assignment {
                target: name,
                value,
            })
        } else if self.match_token(&Token::LeftParen) {
            // Procedure call
            let mut arguments = Vec::new();
            if !self.check(&Token::RightParen) {
                loop {
                    arguments.push(self.parse_expression()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }
            self.consume(&Token::RightParen, "Expected ')'")?;
            self.consume(&Token::Semicolon, "Expected semicolon")?;

            Ok(Statement::Call { name, arguments })
        } else {
            Err(DbError::SqlParse(format!("Expected := or ( after {}", name)))
        }
    }

    /// Parse exception handler
    fn parse_exception_handler(&mut self) -> Result<ExceptionHandler> {
        self.consume(&Token::When, "Expected WHEN")?);

        let exception_name = self.consume_identifier("Expected exception name")?;
        let exception_type = match exception_name.to_uppercase().as_str() {
            "NO_DATA_FOUND" => ExceptionType::NoDataFound,
            "TOO_MANY_ROWS" => ExceptionType::TooManyRows,
            "ZERO_DIVIDE" => ExceptionType::ZeroDivide,
            "VALUE_ERROR" => ExceptionType::ValueError,
            "INVALID_CURSOR" => ExceptionType::InvalidCursor,
            "DUP_VAL_ON_INDEX" => ExceptionType::DupValOnIndex,
            "OTHERS" => ExceptionType::Others,
            _ => ExceptionType::UserDefined(exception_name),
        };

        self.consume(&Token::Then, "Expected THEN")?;

        let mut statements = Vec::new();
        while !self.check(&Token::When) && !self.check(&Token::End) {
            statements.push(self.parse_statement()?);
        }

        Ok(ExceptionHandler {
            exception_type,
            statements,
        })
    }

    /// Parse expression
    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.match_token(&Token::Or) {
            let right = self.parse_and_expression()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_comparison_expression()?;

        while self.match_token(&Token::And) {
            let right = self.parse_comparison_expression()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_additive_expression()?;

        if self.check(&Token::Equal) || self.check(&Token::NotEqual) ||
           self.check(&Token::LessThan) || self.check(&Token::LessThanOrEqual) ||
           self.check(&Token::GreaterThan) || self.check(&Token::GreaterThanOrEqual) ||
           self.check(&Token::Like) {

            let op = match self.advance() {
                Token::Equal => BinaryOperator::Equal,
                Token::NotEqual => BinaryOperator::NotEqual,
                Token::LessThan => BinaryOperator::LessThan,
                Token::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
                Token::GreaterThan => BinaryOperator::GreaterThan,
                Token::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
                Token::Like => BinaryOperator::Like,
                _ => unreachable!(),
            };

            let right = self.parse_additive_expression()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_multiplicative_expression()?;

        while self.check(&Token::Plus) || self.check(&Token::Minus) || self.check(&Token::Concat) {
            let op = match self.advance() {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                Token::Concat => BinaryOperator::Concat,
                _ => unreachable!(),
            };

            let right = self.parse_multiplicative_expression()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_unary_expression()?;

        while self.check(&Token::Star) || self.check(&Token::Slash) || self.check(&Token::Percent) {
            let op = match self.advance() {
                Token::Star => BinaryOperator::Multiply,
                Token::Slash => BinaryOperator::Divide,
                Token::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };

            let right = self.parse_unary_expression()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression> {
        if self.match_token(&Token::Not) {
            let operand = self.parse_unary_expression()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
            })
        } else if self.match_token(&Token::Minus) {
            let operand = self.parse_unary_expression()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Minus,
                operand: Box::new(operand),
            })
        } else if self.match_token(&Token::Plus) {
            let operand = self.parse_unary_expression()?;
            Ok(Expression::UnaryOp {
                op: UnaryOperator::Plus,
                operand: Box::new(operand),
            })
        } else {
            self.parse_primary_expression()
        }
    }

    fn parse_primary_expression(&mut self) -> Result<Expression> {
        match self.peek() {
            Token::IntegerLit(val) => {
                let v = *val;
                self.advance();
                Ok(Expression::Literal(LiteralValue::Integer(v)))
            }
            Token::FloatLit(val) => {
                let v = *val;
                self.advance();
                Ok(Expression::Literal(LiteralValue::Float(v)))
            }
            Token::StringLit(val) => {
                let v = val.clone();
                self.advance();
                Ok(Expression::Literal(LiteralValue::String(v)))
            }
            Token::BooleanLit(val) => {
                let v = *val;
                self.advance();
                Ok(Expression::Literal(LiteralValue::Boolean(v)))
            }
            Token::Null => {
                self.advance();
                Ok(Expression::Literal(LiteralValue::Null))
            }
            Token::Identifier(_) => {
                let name = self.consume_identifier("Expected identifier")?;

                if self.match_token(&Token::LeftParen) {
                    // Function call
                    let mut arguments = Vec::new();
                    if !self.check(&Token::RightParen) {
                        loop {
                            arguments.push(self.parse_expression()?);
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(&Token::RightParen, "Expected ')'")?;

                    Ok(Expression::FunctionCall { name, arguments })
                } else if self.match_token(&Token::Dot) {
                    // Field access
                    let field = self.consume_identifier("Expected field name")?;
                    Ok(Expression::FieldAccess {
                        record: name,
                        field,
                    })
                } else {
                    // Variable reference
                    Ok(Expression::Variable(name))
                }
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(&Token::RightParen, "Expected ')'")?;
                Ok(expr)
            }
            _ => Err(DbError::SqlParse(format!("Unexpected token in expression: {:?}", self.peek()))),
        }
    }

    // Helper methods
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1);
        }
        &self.tokens[self.current - 1]
    }

    fn check(&self, token: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.peek(), Token::Identifier(_))
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token: &Token, message: &str) -> Result<()> {
        if self.check(token) {
            self.advance();
            Ok(())
        } else {
            Err(DbError::SqlParse(format!("{}, got {:?}", message, self.peek())))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String> {
        if let Token::Identifier(name) = self.peek() {
            let result = name.clone());
            self.advance();
            Ok(result)
        } else {
            Err(DbError::SqlParse(format!("{}, got {:?}", message, self.peek())))
        }
    }

    fn consume_integer(&mut self, message: &str) -> Result<i64> {
        if let Token::IntegerLit(val) = self.peek() {
            let result = *val);
            self.advance();
            Ok(result)
        } else {
            Err(DbError::SqlParse(format!("{}, got {:?}", message, self.peek())))
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }
}

impl Default for PlSqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*);

    #[test]
    fn test_parse_simple_block() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            BEGIN
                NULL;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(block.statements[0], Statement::Null));

        Ok(())
    }

    #[test]
    fn test_parse_declarations() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            DECLARE
                x INTEGER;
                y VARCHAR2(100) := 'hello';
            BEGIN
                NULL;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.declarations.len(), 2);

        Ok(())
    }

    #[test]
    fn test_parse_if_statement() -> Result<()> {
        let mut parser = PlSqlParser::new();
        let source = r#"
            BEGIN
                IF x > 10 THEN
                    y := 1;
                ELSIF x > 5 THEN
                    y := 2;
                ELSE
                    y := 3;
                END IF;
            END;
        "#;

        let block = parser.parse(source)?;
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(block.statements[0], Statement::If { .. }));

        Ok(())
    }
}


