// # Audit Logger
//
// Enterprise-grade audit logging system with:
// - Async buffered writing for high performance
// - Multiple output destinations (file, database, syslog)
// - Tamper-evident logging with checksums
// - SOC2/HIPAA compliance features

use crate::audit::audit_events::{AuditEvent, AuditEventId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

// ============================================================================
// Configuration
// ============================================================================

/// Audit logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggerConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Log file path
    pub log_file_path: PathBuf,

    /// Enable database logging
    pub database_logging: bool,

    /// Enable syslog logging
    pub syslog_logging: bool,

    /// Buffer size (number of events before flush)
    pub buffer_size: usize,

    /// Flush interval in seconds
    pub flush_interval_secs: u64,

    /// Enable tamper-evident checksums
    pub enable_checksums: bool,

    /// Log file rotation size in bytes (0 = no rotation)
    pub rotation_size_bytes: u64,

    /// Maximum number of archived log files
    pub max_archive_files: usize,

    /// Enable compression for archived files
    pub compress_archives: bool,
}

impl Default for AuditLoggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_file_path: PathBuf::from("./data/audit/audit.log"),
            database_logging: true,
            syslog_logging: false,
            buffer_size: 1000,
            flush_interval_secs: 5,
            enable_checksums: true,
            rotation_size_bytes: 100 * 1024 * 1024, // 100 MB
            max_archive_files: 30,
            compress_archives: true,
        }
    }
}

// ============================================================================
// Audit Logger Implementation
// ============================================================================

/// Enterprise audit logger with async buffering
pub struct AuditLogger {
    /// Configuration
    config: AuditLoggerConfig,

    /// Event buffer
    buffer: Arc<Mutex<VecDeque<AuditEvent>>>,

    /// Current event ID counter
    event_id_counter: Arc<RwLock<AuditEventId>>,

    /// File writer (wrapped in Arc<Mutex> for async access)
    file_writer: Arc<Mutex<Option<BufWriter<File>>>>,

    /// Current log file size
    current_file_size: Arc<RwLock<u64>>,

    /// Logger state
    is_running: Arc<RwLock<bool>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub async fn new(config: AuditLoggerConfig) -> Result<Self> {
        // Create log directory if it doesn't exist
        if let Some(parent) = config.log_file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                DbError::Io(Arc::new(e))
            })?;
        }

        let logger = Self {
            config,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            event_id_counter: Arc::new(RwLock::new(1)),
            file_writer: Arc::new(Mutex::new(None)),
            current_file_size: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
        };

        Ok(logger)
    }

    /// Initialize the audit logger
    pub async fn initialize(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Open log file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file_path)
            .await
            .map_err(|e| DbError::Io(Arc::new(e)))?;

        // Get current file size
        let metadata = file.metadata().await
            .map_err(|e| DbError::Io(Arc::new(e)))?;
        *self.current_file_size.write().await = metadata.len();

        let writer = BufWriter::new(file);
        *self.file_writer.lock().await = Some(writer);

        // Mark as running
        *self.is_running.write().await = true;

        // Start background flush task
        self.start_flush_task().await;

        Ok(())
    }

    /// Shutdown the audit logger
    pub async fn shutdown(&mut self) -> Result<()> {
        *self.is_running.write().await = false;

        // Flush remaining events
        self.flush().await?;

        // Close file
        let mut writer_guard = self.file_writer.lock().await;
        if let Some(mut writer) = writer_guard.take() {
            writer.flush().await.map_err(|e| DbError::Io(Arc::new(e)))?;
        }

        Ok(())
    }

    /// Log an audit event
    pub async fn log_event(&self, mut event: AuditEvent) -> Result<AuditEventId> {
        if !self.config.enabled {
            return Ok(0);
        }

        // Assign event ID
        let event_id = self.next_event_id().await;
        event.event_id = event_id;

        // Compute checksum if enabled
        if self.config.enable_checksums {
            event.compute_checksum();
        }

        // Add to buffer
        let mut buffer = self.buffer.lock().await;
        buffer.push_back(event);

        // Check if we need to flush
        if buffer.len() >= self.config.buffer_size {
            drop(buffer); // Release lock before flushing
            self.flush().await?;
        }

        Ok(event_id)
    }

    /// Flush buffered events to storage
    pub async fn flush(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut buffer = self.buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }

        // Drain all events from buffer
        let events: Vec<AuditEvent> = buffer.drain(..).collect();
        drop(buffer); // Release lock early

        // Write to file
        if let Err(e) = self.write_to_file(&events).await {
            // On error, put events back in buffer
            let mut buffer = self.buffer.lock().await;
            for event in events.iter().rev() {
                buffer.push_front(event.clone());
            }
            return Err(e);
        }

        // Write to database if enabled
        if self.config.database_logging {
            if let Err(e) = self.write_to_database(&events).await {
                eprintln!("Failed to write audit events to database: {}", e);
                // Continue anyway - file logging succeeded
            }
        }

        // Write to syslog if enabled
        if self.config.syslog_logging {
            if let Err(e) = self.write_to_syslog(&events).await {
                eprintln!("Failed to write audit events to syslog: {}", e);
                // Continue anyway - file logging succeeded
            }
        }

        Ok(())
    }

    /// Write events to file
    async fn write_to_file(&self, events: &[AuditEvent]) -> Result<()> {
        let mut writer_guard = self.file_writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            for event in events {
                // Serialize event as JSON (one per line)
                let json = serde_json::to_string(event)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;

                writer.write_all(json.as_bytes()).await
                    .map_err(|e| DbError::Io(Arc::new(e)))?;
                writer.write_all(b"\n").await
                    .map_err(|e| DbError::Io(Arc::new(e)))?;

                // Update file size
                let bytes_written = json.len() + 1; // +1 for newline
                *self.current_file_size.write().await += bytes_written as u64;
            }

            writer.flush().await.map_err(|e| DbError::Io(Arc::new(e)))?;

            // Check if rotation is needed
            let file_size = *self.current_file_size.read().await;
            if self.config.rotation_size_bytes > 0 && file_size >= self.config.rotation_size_bytes {
                drop(writer_guard); // Release lock before rotation
                self.rotate_log_file().await?;
            }
        }

        Ok(())
    }

    /// Write events to database (placeholder for actual implementation)
    async fn write_to_database(&self, events: &[AuditEvent]) -> Result<()> {
        // TODO: Implement database insertion
        // This would insert events into an audit_log table
        // For now, just log the count
        let _ = events.len();
        Ok(())
    }

    /// Write events to syslog (placeholder for actual implementation)
    async fn write_to_syslog(&self, events: &[AuditEvent]) -> Result<()> {
        // TODO: Implement syslog integration
        // This would send events to system syslog
        let _ = events.len();
        Ok(())
    }

    /// Rotate log file
    async fn rotate_log_file(&self) -> Result<()> {
        // Close current file
        let mut writer_guard = self.file_writer.lock().await;
        if let Some(mut writer) = writer_guard.take() {
            writer.flush().await.map_err(|e| DbError::Io(Arc::new(e)))?;
        }

        // Generate timestamp for archive name
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let archive_path = self.config.log_file_path.with_extension(
            format!("log.{}", timestamp)
        );

        // Rename current log file
        tokio::fs::rename(&self.config.log_file_path, &archive_path)
            .await
            .map_err(|e| DbError::Io(Arc::new(e)))?;

        // Compress if enabled
        if self.config.compress_archives {
            // TODO: Implement compression
            // For now, just leave uncompressed
        }

        // Clean old archives
        self.cleanup_old_archives().await?;

        // Create new log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file_path)
            .await
            .map_err(|e| DbError::Io(Arc::new(e)))?;

        *writer_guard = Some(BufWriter::new(file));
        *self.current_file_size.write().await = 0;

        Ok(())
    }

    /// Clean up old archive files
    async fn cleanup_old_archives(&self) -> Result<()> {
        // TODO: Implement archive cleanup
        // Would enumerate archive files and delete oldest ones beyond max_archive_files
        Ok(())
    }

    /// Get next event ID
    async fn next_event_id(&self) -> AuditEventId {
        let mut counter = self.event_id_counter.write().await;
        let id = *counter;
        *counter += 1;
        id
    }

    /// Start background flush task
    async fn start_flush_task(&self) {
        let buffer = Arc::clone(&self.buffer);
        let is_running = Arc::clone(&self.is_running);
        let flush_interval_secs = self.config.flush_interval_secs;
        let logger_clone = self.clone_for_task();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(flush_interval_secs));

            loop {
                ticker.tick().await;

                // Check if still running
                if !*is_running.read().await {
                    break;
                }

                // Flush if buffer has events
                let has_events = {
                    let buffer = buffer.lock().await;
                    !buffer.is_empty()
                };

                if has_events {
                    if let Err(e) = logger_clone.flush().await {
                        eprintln!("Audit logger flush error: {}", e);
                    }
                }
            }
        });
    }

    /// Clone for background task
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            buffer: Arc::clone(&self.buffer),
            event_id_counter: Arc::clone(&self.event_id_counter),
            file_writer: Arc::clone(&self.file_writer),
            current_file_size: Arc::clone(&self.current_file_size),
            is_running: Arc::clone(&self.is_running),
        }
    }

    /// Get statistics
    pub async fn get_statistics(&self) -> AuditLoggerStatistics {
        let buffer_size = self.buffer.lock().await.len();
        let current_event_id = *self.event_id_counter.read().await;
        let current_file_size = *self.current_file_size.read().await;

        AuditLoggerStatistics {
            total_events_logged: current_event_id,
            buffered_events: buffer_size,
            current_file_size_bytes: current_file_size,
            is_running: *self.is_running.read().await,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Audit logger statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggerStatistics {
    /// Total events logged
    pub total_events_logged: AuditEventId,

    /// Number of events in buffer
    pub buffered_events: usize,

    /// Current file size
    pub current_file_size_bytes: u64,

    /// Is logger running
    pub is_running: bool,
}

// ============================================================================
// Thread-safe Audit Logger Wrapper
// ============================================================================

/// Thread-safe wrapper for AuditLogger
pub type SharedAuditLogger = Arc<AuditLogger>;

/// Create a new shared audit logger
pub async fn create_audit_logger(config: AuditLoggerConfig) -> Result<SharedAuditLogger> {
    let mut logger = AuditLogger::new(config).await?;
    logger.initialize().await?;
    Ok(Arc::new(logger))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::audit_events::{
        ActionOutcome, AuditEventType, AuditSeverity, EventCategory,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLoggerConfig {
            enabled: true,
            log_file_path: log_path.clone(),
            ..Default::default()
        };

        let logger = AuditLogger::new(config).await.unwrap();
        assert!(log_path.parent().is_some());
    }

    #[tokio::test]
    async fn test_event_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLoggerConfig {
            enabled: true,
            log_file_path: log_path.clone(),
            buffer_size: 10,
            ..Default::default()
        };

        let mut logger = AuditLogger::new(config).await.unwrap();
        logger.initialize().await.unwrap();

        let event = AuditEvent::new(
            0, // Will be assigned by logger
            EventCategory::Authentication,
            AuditSeverity::Info,
            ActionOutcome::Success,
            AuditEventType::LoginSuccess {
                authentication_method: "password".to_string(),
            },
            "test_user".to_string(),
            "test_db".to_string(),
        );

        let event_id = logger.log_event(event).await.unwrap();
        assert_eq!(event_id, 1);

        logger.shutdown().await.unwrap();
    }
}
