/// PL/SQL AST Node Definitions
///
/// This module defines all Abstract Syntax Tree nodes for PL/SQL-compatible
/// procedural language, including declarations, statements, expressions, and types.

use serde::{Deserialize, Serialize};

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
