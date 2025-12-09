// REST API Endpoints for String Functions
//
// HTTP endpoints for executing SQL Server string functions

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::error::DbError;
use crate::execution::string_functions::StringFunctionExecutor;
use crate::parser::string_functions::{StringExpr, StringFunction};

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct StringFunctionRequest {
    /// The string function to execute
    pub function: StringFunctionType,
    /// Context values for column references
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StringFunctionResponse {
    pub result: String,
    pub execution_time_ms: f64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StringFunctionType {
    Ascii { value: String },
    Char { code: i64 },
    CharIndex { substring: String, string: String, start_position: Option<i64> },
    Concat { values: Vec<String> },
    ConcatWs { separator: String, values: Vec<String> },
    DataLength { value: String },
    Difference { string1: String, string2: String },
    Format { value: String, format: String, culture: Option<String> },
    Left { string: String, length: i64 },
    Len { value: String },
    Lower { value: String },
    LTrim { value: String },
    NChar { code: i64 },
    PatIndex { pattern: String, string: String },
    QuoteName { string: String, quote_char: Option<String> },
    Replace { string: String, old_substring: String, new_substring: String },
    Replicate { string: String, count: i64 },
    Reverse { value: String },
    Right { string: String, length: i64 },
    RTrim { value: String },
    Soundex { value: String },
    Space { count: i64 },
    Str { number: f64, length: Option<i64>, decimals: Option<i64> },
    Stuff { string: String, start: i64, length: i64, new_string: String },
    Substring { string: String, start: i64, length: i64 },
    Translate { string: String, characters: String, translations: String },
    Trim { value: String, characters: Option<String> },
    Unicode { value: String },
    Upper { value: String },
}

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

impl StringFunctionType {
    fn to_ast(&self) -> StringFunction {
        match self {
            StringFunctionType::Ascii { value } => {
                StringFunction::Ascii(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Char { code } => {
                StringFunction::Char(Box::new(StringExpr::Integer(*code)))
            }
            StringFunctionType::CharIndex { substring, string, start_position } => {
                StringFunction::CharIndex {
                    substring: Box::new(StringExpr::Literal(substring.clone())),
                    string: Box::new(StringExpr::Literal(string.clone())),
                    start_position: start_position.map(|sp| Box::new(StringExpr::Integer(sp))),
                }
            }
            StringFunctionType::Concat { values } => {
                StringFunction::Concat(
                    values.iter().map(|v| StringExpr::Literal(v.clone())).collect()
                )
            }
            StringFunctionType::ConcatWs { separator, values } => {
                StringFunction::ConcatWs {
                    separator: Box::new(StringExpr::Literal(separator.clone())),
                    strings: values.iter().map(|v| StringExpr::Literal(v.clone())).collect(),
                }
            }
            StringFunctionType::DataLength { value } => {
                StringFunction::DataLength(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Difference { string1, string2 } => {
                StringFunction::Difference {
                    string1: Box::new(StringExpr::Literal(string1.clone())),
                    string2: Box::new(StringExpr::Literal(string2.clone())),
                }
            }
            StringFunctionType::Format { value, format, culture } => {
                StringFunction::Format {
                    value: Box::new(StringExpr::Literal(value.clone())),
                    format: Box::new(StringExpr::Literal(format.clone())),
                    culture: culture.as_ref().map(|c| Box::new(StringExpr::Literal(c.clone()))),
                }
            }
            StringFunctionType::Left { string, length } => {
                StringFunction::Left {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    length: Box::new(StringExpr::Integer(*length)),
                }
            }
            StringFunctionType::Len { value } => {
                StringFunction::Len(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Lower { value } => {
                StringFunction::Lower(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::LTrim { value } => {
                StringFunction::LTrim(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::NChar { code } => {
                StringFunction::NChar(Box::new(StringExpr::Integer(*code)))
            }
            StringFunctionType::PatIndex { pattern, string } => {
                StringFunction::PatIndex {
                    pattern: Box::new(StringExpr::Literal(pattern.clone())),
                    string: Box::new(StringExpr::Literal(string.clone())),
                }
            }
            StringFunctionType::QuoteName { string, quote_char } => {
                StringFunction::QuoteName {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    quote_char: quote_char.as_ref().map(|qc| Box::new(StringExpr::Literal(qc.clone()))),
                }
            }
            StringFunctionType::Replace { string, old_substring, new_substring } => {
                StringFunction::Replace {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    old_substring: Box::new(StringExpr::Literal(old_substring.clone())),
                    new_substring: Box::new(StringExpr::Literal(new_substring.clone())),
                }
            }
            StringFunctionType::Replicate { string, count } => {
                StringFunction::Replicate {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    count: Box::new(StringExpr::Integer(*count)),
                }
            }
            StringFunctionType::Reverse { value } => {
                StringFunction::Reverse(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Right { string, length } => {
                StringFunction::Right {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    length: Box::new(StringExpr::Integer(*length)),
                }
            }
            StringFunctionType::RTrim { value } => {
                StringFunction::RTrim(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Soundex { value } => {
                StringFunction::Soundex(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Space { count } => {
                StringFunction::Space(Box::new(StringExpr::Integer(*count)))
            }
            StringFunctionType::Str { number, length, decimals } => {
                StringFunction::Str {
                    number: Box::new(StringExpr::Float(*number)),
                    length: length.map(|l| Box::new(StringExpr::Integer(l))),
                    decimals: decimals.map(|d| Box::new(StringExpr::Integer(d))),
                }
            }
            StringFunctionType::Stuff { string, start, length, new_string } => {
                StringFunction::Stuff {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    start: Box::new(StringExpr::Integer(*start)),
                    length: Box::new(StringExpr::Integer(*length)),
                    new_string: Box::new(StringExpr::Literal(new_string.clone())),
                }
            }
            StringFunctionType::Substring { string, start, length } => {
                StringFunction::Substring {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    start: Box::new(StringExpr::Integer(*start)),
                    length: Box::new(StringExpr::Integer(*length)),
                }
            }
            StringFunctionType::Translate { string, characters, translations } => {
                StringFunction::Translate {
                    string: Box::new(StringExpr::Literal(string.clone())),
                    characters: Box::new(StringExpr::Literal(characters.clone())),
                    translations: Box::new(StringExpr::Literal(translations.clone())),
                }
            }
            StringFunctionType::Trim { value, characters } => {
                StringFunction::Trim {
                    string: Box::new(StringExpr::Literal(value.clone())),
                    characters: characters.as_ref().map(|c| Box::new(StringExpr::Literal(c.clone()))),
                }
            }
            StringFunctionType::Unicode { value } => {
                StringFunction::Unicode(Box::new(StringExpr::Literal(value.clone())))
            }
            StringFunctionType::Upper { value } => {
                StringFunction::Upper(Box::new(StringExpr::Literal(value.clone())))
            }
        }
    }
}

// ============================================================================
// API HANDLERS
// ============================================================================

/// Execute a string function
#[utoipa::path(
    post,
    path = "/api/v1/string-functions/execute",
    request_body = StringFunctionRequest,
    responses(
        (status = 200, description = "Function executed successfully", body = StringFunctionResponse),
        (status = 400, description = "Invalid input"),
        (status = 500, description = "Internal server error")
    ),
    tag = "String Functions"
)]
pub async fn execute_string_function(
    Json(request): Json<StringFunctionRequest>,
) -> Result<Json<StringFunctionResponse>, ApiError> {
    let start = std::time::Instant::now();

    // Convert request to AST
    let function = request.function.to_ast();

    // Execute function
    let mut executor = StringFunctionExecutor::new();
    let result = executor.execute(&function, &request.context)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(StringFunctionResponse {
        result,
        execution_time_ms,
    }))
}

/// Batch execute multiple string functions
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchStringFunctionRequest {
    pub functions: Vec<StringFunctionType>,
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchStringFunctionResponse {
    pub results: Vec<String>,
    pub execution_time_ms: f64,
}

#[utoipa::path(
    post,
    path = "/api/v1/string-functions/batch",
    request_body = BatchStringFunctionRequest,
    responses(
        (status = 200, description = "Functions executed successfully", body = BatchStringFunctionResponse),
        (status = 400, description = "Invalid input"),
        (status = 500, description = "Internal server error")
    ),
    tag = "String Functions"
)]
pub async fn batch_execute_string_functions(
    Json(request): Json<BatchStringFunctionRequest>,
) -> Result<Json<BatchStringFunctionResponse>, ApiError> {
    let start = std::time::Instant::now();

    let mut executor = StringFunctionExecutor::new();
    let mut results = Vec::new();

    for func_type in &request.functions {
        let function = func_type.to_ast();
        let result = executor.execute(&function, &request.context)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        results.push(result);
    }

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(BatchStringFunctionResponse {
        results,
        execution_time_ms,
    }))
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
