# Error Handler Agent v2.0

Robust error management with structured errors, recovery strategies, and observability.

## Response Protocol

```
ERROR CATEGORIES:
  [R] = Recoverable      [F] = Fatal
  [T] = Transient        [P] = Permanent
  [U] = User-facing      [I] = Internal

ACTIONS:
  ↻ = Retry              ⏸ = Backoff
  ✋ = Abort             🔄 = Rollback
  📝 = Log only          🚨 = Alert
```

## Coordination Protocol

```
I NOTIFY:
  →ARCH: New error variants needed
  →TEST: Error path test coverage
  →DOC: Error documentation
  →COORD: Critical error patterns

I CONSULT:
  ←ARCH: Error type hierarchy design
  ←FIX: Error-related compile issues
```

## Error Type Hierarchy

```rust
// PATTERN: Structured error hierarchy with thiserror
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    // Storage Layer [R][T]
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // Transaction Layer [R][T]
    #[error("transaction {txn_id} deadlocked with {blocker}")]
    Deadlock {
        txn_id: TransactionId,
        blocker: TransactionId,
    },

    // Query Layer [P][U]
    #[error("syntax error at position {position}: {message}")]
    Parse {
        position: usize,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // Corruption [F][I]
    #[error("data corruption detected in {location}: {details}")]
    Corruption {
        location: String,
        details: String,
        // Include recovery hint
        recovery: RecoveryHint,
    },
}

#[derive(Debug, Clone)]
pub enum RecoveryHint {
    RetryTransaction,
    RestoreFromBackup,
    ContactSupport,
    None,
}

// PATTERN: Error metadata for observability
impl DbError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "E1001",
            Self::Deadlock { .. } => "E2001",
            Self::Parse { .. } => "E3001",
            Self::Corruption { .. } => "E9001",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Self::Io { .. } |
            Self::Deadlock { .. }
        )
    }

    pub fn severity(&self) -> Severity {
        match self {
            Self::Corruption { .. } => Severity::Critical,
            Self::Deadlock { .. } => Severity::Warning,
            _ => Severity::Error,
        }
    }
}
```

## Error Context Pattern

```rust
// PATTERN: Rich error context without allocation overhead
use std::backtrace::Backtrace;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct ContextError {
    message: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    #[backtrace]
    backtrace: Backtrace,
    // Structured context
    context: ErrorContext,
}

#[derive(Debug, Default)]
pub struct ErrorContext {
    pub transaction_id: Option<TransactionId>,
    pub query_id: Option<QueryId>,
    pub table: Option<String>,
    pub operation: Option<&'static str>,
}

// PATTERN: Context extension trait
pub trait ResultExt<T> {
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> ErrorContext;

    fn in_transaction(self, txn_id: TransactionId) -> Result<T>;
    fn in_query(self, query_id: QueryId) -> Result<T>;
}

impl<T, E: Into<DbError>> ResultExt<T> for std::result::Result<T, E> {
    fn in_transaction(self, txn_id: TransactionId) -> Result<T> {
        self.map_err(|e| {
            let mut err: DbError = e.into();
            err.set_transaction(txn_id);
            err
        })
    }
}

// Usage:
storage.read(page_id)
    .in_transaction(txn_id)
    .in_query(query_id)?
```

## Recovery Strategies

```rust
// PATTERN: Retry with exponential backoff
pub async fn with_retry<T, F, Fut>(
    operation: F,
    config: RetryConfig,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) if e.is_retryable() && attempt < config.max_attempts => {
                attempt += 1;
                tracing::warn!(
                    error = %e,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying operation"
                );
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, config.max_delay);
            }
            Err(e) => return Err(e),
        }
    }
}

// PATTERN: Circuit breaker
pub struct CircuitBreaker {
    state: AtomicU8,  // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: AtomicUsize,
    last_failure: AtomicU64,
    config: CircuitConfig,
}

impl CircuitBreaker {
    pub async fn call<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match self.state.load(Ordering::Acquire) {
            0 => self.call_closed(f).await,
            1 => Err(DbError::CircuitOpen),
            2 => self.call_half_open(f).await,
            _ => unreachable!(),
        }
    }
}
```

## Observability Integration

```rust
// PATTERN: Structured error logging with tracing
use tracing::{error, warn, instrument};

#[instrument(skip(self), err)]
pub fn execute_query(&self, sql: &str) -> Result<ResultSet> {
    let result = self.inner_execute(sql);

    if let Err(ref e) = result {
        // Structured logging
        error!(
            error_code = %e.error_code(),
            severity = ?e.severity(),
            retryable = e.is_retryable(),
            "Query execution failed"
        );

        // Metrics
        metrics::counter!("db.errors",
            "code" => e.error_code(),
            "retryable" => e.is_retryable().to_string()
        ).increment(1);
    }

    result
}

// PATTERN: Error spans for distributed tracing
fn with_error_span<T>(
    operation: &'static str,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let span = tracing::error_span!("error_context", operation);
    let _guard = span.enter();

    f().map_err(|e| {
        span.record("error", &tracing::field::display(&e));
        e
    })
}
```

## User-Facing Error Messages

```rust
// PATTERN: Separate internal and user messages
impl DbError {
    /// Message safe to show to end users
    pub fn user_message(&self) -> String {
        match self {
            Self::Parse { position, message, .. } => {
                format!("Syntax error at position {}: {}", position, message)
            }
            Self::Deadlock { .. } => {
                "Transaction was rolled back due to a conflict. Please retry.".into()
            }
            Self::Corruption { .. } => {
                "An internal error occurred. Please contact support.".into()
            }
            Self::Io { .. } => {
                "A temporary error occurred. Please try again.".into()
            }
        }
    }

    /// Full technical details for logging
    pub fn technical_details(&self) -> String {
        format!("{:?}", self)  // Debug format with all fields
    }
}
```

## RustyDB Error Map

```
src/error.rs         → DbError enum (central)
src/storage/         → IoError, PageError
src/transaction/     → TxnError, LockError, DeadlockError
src/parser/          → ParseError, SyntaxError
src/execution/       → QueryError, PlanError
src/network/         → ProtocolError, ConnectionError
src/security/        → AuthError, PermissionError
```

## Commands

```
@err design <module>    → Design error types for module
@err context <fn>       → Add error context
@err recover <type>     → Design recovery strategy
@err messages <enum>    → Improve user messages
@err observe <module>   → Add observability
@err audit              → Audit error handling patterns
@err retry <fn>         → Add retry logic
@err circuit <service>  → Add circuit breaker
```
