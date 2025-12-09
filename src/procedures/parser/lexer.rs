/// PL/SQL Lexical Analyzer
///
/// This module provides tokenization functionality for PL/SQL source code.

use crate::{Result, DbError};

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
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

/// Tokenize PL/SQL source code
pub fn tokenize(source: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
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
            tokens.push(Token::StringLit(string_val));
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
                    DbError::SqlParse(format!("Invalid float: {}", num_str)))?;
                tokens.push(Token::FloatLit(val));
            } else {
                let val: i64 = num_str.parse().map_err(|_|
                    DbError::SqlParse(format!("Invalid integer: {}", num_str)))?;
                tokens.push(Token::IntegerLit(val));
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
            tokens.push(token);
            continue;
        }

        // Operators and delimiters
        match chars[i] {
            '(' => {
                tokens.push(Token::LeftParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                i += 1;
            }
            ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            ':' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Assign);
                    i += 2;
                } else {
                    return Err(DbError::SqlParse("Unexpected character: :".to_string()));
                }
            }
            '=' => {
                tokens.push(Token::Equal);
                i += 1;
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::LessThanOrEqual);
                    i += 2;
                } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::NotEqual);
                    i += 2;
                } else {
                    tokens.push(Token::LessThan);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::GreaterThanOrEqual);
                    i += 2;
                } else {
                    tokens.push(Token::GreaterThan);
                    i += 1;
                }
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::NotEqual);
                    i += 2;
                } else {
                    return Err(DbError::SqlParse("Unexpected character: !".to_string()));
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(Token::Concat);
                    i += 2;
                } else {
                    return Err(DbError::SqlParse("Unexpected character: |".to_string()));
                }
            }
            _ => {
                return Err(DbError::SqlParse(format!("Unexpected character: {}", chars[i])));
            }
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}
