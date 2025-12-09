// Stream Connectors
//
// Implements various connectors for streaming data including Kafka-compatible protocol,
// JDBC/Database sink, file sink (JSON, Parquet), HTTP webhook, and custom connectors.

use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Mutex;
use super::{Event, EventBatch, EventValue, StreamPosition};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration};

/// Connector type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectorType {
    Source,
    Sink,
    Bidirectional,
}

/// Connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Connector name
    pub name: String,

    /// Connector type
    pub connector_type: ConnectorType,

    /// Batch size
    pub batch_size: usize,

    /// Flush interval
    pub flush_interval: Duration,

    /// Max retries
    pub max_retries: u32,

    /// Connector-specific properties
    pub properties: HashMap<String, String>,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            connector_type: ConnectorType::Sink,
            batch_size: 1000,
            flush_interval: Duration::from_secs(5),
            max_retries: 3,
            properties: HashMap::new(),
        }
    }
}

/// Source connector trait
pub trait SourceConnector: Send + Sync {
    /// Start the connector
    fn start(&mut self) -> Result<()>;

    /// Stop the connector
    fn stop(&mut self) -> Result<()>;

    /// Poll for events
    fn poll(&mut self, timeout: Duration) -> Result<Vec<Event>>;

    /// Commit offsets
    fn commit(&mut self, position: StreamPosition) -> Result<()>;

    /// Get connector name
    fn name(&self) -> &str;
}

/// Sink connector trait
pub trait SinkConnector: Send + Sync {
    /// Start the connector
    fn start(&mut self) -> Result<()>;

    /// Stop the connector
    fn stop(&mut self) -> Result<()>;

    /// Write an event
    fn write(&mut self, event: Event) -> Result<()>;

    /// Write a batch of events
    fn write_batch(&mut self, events: Vec<Event>) -> Result<()>;

    /// Flush buffered events
    fn flush(&mut self) -> Result<()>;

    /// Get connector name
    fn name(&self) -> &str;
}

/// Kafka-compatible connector
pub struct KafkaConnector {
    config: ConnectorConfig,
    topic: String,
    bootstrap_servers: Vec<String>,
    consumer_group: Option<String>,
    buffer: Vec<Event>,
    committed_offset: u64,
    is_running: bool,
}

impl KafkaConnector {
    pub fn new(config: ConnectorConfig, topic: impl Into<String>) -> Self {
        let bootstrap_servers = config
            .properties
            .get("bootstrap.servers")
            .map(|s| s.split(',').map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["localhost:9092".to_string()]);

        let consumer_group = config.properties.get("group.id").cloned();

        Self {
            config,
            topic: topic.into(),
            bootstrap_servers,
            consumer_group,
            buffer: Vec::new(),
            committed_offset: 0,
            is_running: false,
        }
    }

    pub fn as_source(&self) -> KafkaSourceConnector {
        KafkaSourceConnector {
            connector: self.clone(),
        }
    }

    pub fn as_sink(&self) -> KafkaSinkConnector {
        KafkaSinkConnector {
            connector: self.clone(),
        }
    }

    fn serialize_event(&self, event: &Event) -> Result<Vec<u8>> {
        serde_json::to_vec(event).map_err(|e| {
            crate::error::DbError::Serialization(format!(
                "Failed to serialize event: {}",
                e
            ))
        })
    }

    fn deserialize_event(&self, data: &[u8]) -> Result<Event> {
        serde_json::from_slice(data).map_err(|e| {
            crate::error::DbError::Serialization(format!(
                "Failed to deserialize event: {}",
                e
            ))
        })
    }
}

impl Clone for KafkaConnector {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            topic: self.topic.clone(),
            bootstrap_servers: self.bootstrap_servers.clone(),
            consumer_group: self.consumer_group.clone(),
            buffer: Vec::new(),
            committed_offset: self.committed_offset,
            is_running: false,
        }
    }
}

/// Kafka source connector
pub struct KafkaSourceConnector {
    connector: KafkaConnector,
}

impl SourceConnector for KafkaSourceConnector {
    fn start(&mut self) -> Result<()> {
        self.connector.is_running = true;
        // In a real implementation, initialize Kafka consumer here
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.connector.is_running = false;
        Ok(())
    }

    fn poll(&mut self, _timeout: Duration) -> Result<Vec<Event>> {
        if !self.connector.is_running {
            return Ok(vec![]);
        }

        // In a real implementation, poll from Kafka
        // For now, return empty
        Ok(vec![])
    }

    fn commit(&mut self, position: StreamPosition) -> Result<()> {
        self.connector.committed_offset = position.offset;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.connector.config.name
    }
}

/// Kafka sink connector
pub struct KafkaSinkConnector {
    connector: KafkaConnector,
}

impl SinkConnector for KafkaSinkConnector {
    fn start(&mut self) -> Result<()> {
        self.connector.is_running = true;
        // In a real implementation, initialize Kafka producer here
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.flush()?;
        self.connector.is_running = false;
        Ok(())
    }

    fn write(&mut self, event: Event) -> Result<()> {
        self.connector.buffer.push(event);

        if self.connector.buffer.len() >= self.connector.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn write_batch(&mut self, events: Vec<Event>) -> Result<()> {
        for event in events {
            self.write(event)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.connector.buffer.is_empty() {
            return Ok(());
        }

        // In a real implementation, send to Kafka
        // For now, just clear the buffer
        self.connector.buffer.clear();

        Ok(())
    }

    fn name(&self) -> &str {
        &self.connector.config.name
    }
}

/// JDBC/Database sink connector
pub struct JdbcSinkConnector {
    config: ConnectorConfig,
    connection_url: String,
    table_name: String,
    buffer: Vec<Event>,
    batch_insert: bool,
}

impl JdbcSinkConnector {
    pub fn new(
        config: ConnectorConfig,
        connection_url: impl Into<String>,
        table_name: impl Into<String>,
    ) -> Self {
        let batch_insert = config
            .properties
            .get("batch.insert")
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        Self {
            config,
            connection_url: connection_url.into(),
            table_name: table_name.into(),
            buffer: Vec::new(),
            batch_insert,
        }
    }

    fn event_to_sql(&self, event: &Event) -> String {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        columns.push("event_id".to_string());
        values.push(format!("'{}'", event.id));

        columns.push("event_type".to_string());
        values.push(format!("'{}'", event.event_type));

        for (key, value) in &event.payload {
            columns.push(key.clone());
            values.push(self.value_to_sql(value));
        }

        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table_name,
            columns.join(", "),
            values.join(", ")
        )
    }

    fn value_to_sql(&self, value: &EventValue) -> String {
        match value {
            EventValue::Null => "NULL".to_string(),
            EventValue::Bool(b) => b.to_string(),
            EventValue::Int64(i) => i.to_string(),
            EventValue::Float64(f) => f.to_string(),
            EventValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            EventValue::Bytes(b) => format!("'{}'", hex::encode(b)),
            EventValue::Timestamp(_) => "NOW()".to_string(),
            _ => "NULL".to_string(),
        }
    }
}

impl SinkConnector for JdbcSinkConnector {
    fn start(&mut self) -> Result<()> {
        // In a real implementation, establish database connection
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.flush()?;
        // In a real implementation, close database connection
        Ok(())
    }

    fn write(&mut self, event: Event) -> Result<()> {
        self.buffer.push(event);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn write_batch(&mut self, events: Vec<Event>) -> Result<()> {
        self.buffer.extend(events);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // In a real implementation, execute SQL
        for event in &self.buffer {
            let _sql = self.event_to_sql(event);
            // Execute SQL
        }

        self.buffer.clear();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

/// File sink connector
pub struct FileSinkConnector {
    config: ConnectorConfig,
    file_path: PathBuf,
    format: FileFormat,
    writer: Option<Arc<Mutex<BufWriter<File>>>>,
    buffer: Vec<Event>,
    rotation_size: u64,
    current_size: u64,
}

/// File format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileFormat {
    Json,
    JsonLines,
    Csv,
    Parquet,
}

impl FileSinkConnector {
    pub fn new(config: ConnectorConfig, file_path: PathBuf, format: FileFormat) -> Self {
        let rotation_size = config
            .properties
            .get("rotation.size.bytes")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100 * 1024 * 1024); // 100MB default

        Self {
            config,
            file_path,
            format,
            writer: None,
            buffer: Vec::new(),
            rotation_size,
            current_size: 0,
        }
    }

    fn open_file(&mut self) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|e| crate::error::DbError::Io(format!("Failed to open file: {}", e)))?;

        self.writer = Some(Arc::new(Mutex::new(BufWriter::new(file))));
        Ok(())
    }

    fn write_event_to_file(&mut self, event: &Event) -> Result<()> {
        if self.writer.is_none() {
            self.open_file()?;
        }

        {
            let writer = self.writer.as_ref().unwrap();
            let mut writer = writer.lock().unwrap();

            match self.format {
                FileFormat::Json => {
                    let json = serde_json::to_string_pretty(event).map_err(|e| {
                        crate::error::DbError::Serialization(format!("Failed to serialize: {}", e))
                    })?;
                    writeln!(writer, "{}", json).map_err(|e| {
                        crate::error::DbError::Io(format!("Failed to write: {}", e))
                    })?;
                    self.current_size += json.len() as u64;
                }

                FileFormat::JsonLines => {
                    let json = serde_json::to_string(event).map_err(|e| {
                        crate::error::DbError::Serialization(format!("Failed to serialize: {}", e))
                    })?;
                    writeln!(writer, "{}", json).map_err(|e| {
                        crate::error::DbError::Io(format!("Failed to write: {}", e))
                    })?;
                    self.current_size += json.len() as u64;
                }

                FileFormat::Csv => {
                    // Simplified CSV output
                    let csv_line = format!(
                        "{},{},{}\n",
                        event.id,
                        event.event_type,
                        serde_json::to_string(&event.payload).unwrap_or_default()
                    );
                    write!(writer, "{}", csv_line).map_err(|e| {
                        crate::error::DbError::Io(format!("Failed to write: {}", e))
                    })?;
                    self.current_size += csv_line.len() as u64;
                }

                FileFormat::Parquet => {
                    // Parquet writing would require apache-parquet crate
                    // Simplified for now
                    return Err(crate::error::DbError::NotImplemented(
                        "Parquet format not yet implemented".to_string()
                    ));
                }
            }
        }

        // Check for rotation
        if self.current_size >= self.rotation_size {
            self.rotate_file()?;
        }

        Ok(())
    }

    fn rotate_file(&mut self) -> Result<()> {
        // Flush current writer
        if let Some(writer) = &self.writer {
            let mut writer = writer.lock().unwrap();
            writer.flush().map_err(|e| {
                crate::error::DbError::Io(format!("Failed to flush: {}", e))
            })?;
        }

        // Rename current file with timestamp
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_path = self.file_path.with_extension(format!("{}.old", timestamp));
        std::fs::rename(&self.file_path, &new_path).map_err(|e| {
            crate::error::DbError::Io(format!("Failed to rename file: {}", e))
        })?;

        // Reset writer
        self.writer = None;
        self.current_size = 0;

        Ok(())
    }
}

impl SinkConnector for FileSinkConnector {
    fn start(&mut self) -> Result<()> {
        self.open_file()
    }

    fn stop(&mut self) -> Result<()> {
        self.flush()?;

        if let Some(writer) = &self.writer {
            let mut writer = writer.lock().unwrap();
            writer.flush().map_err(|e| {
                crate::error::DbError::Io(format!("Failed to flush: {}", e))
            })?;
        }

        Ok(())
    }

    fn write(&mut self, event: Event) -> Result<()> {
        self.buffer.push(event);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn write_batch(&mut self, events: Vec<Event>) -> Result<()> {
        self.buffer.extend(events);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let events = self.buffer.clone();
        for event in &events {
            self.write_event_to_file(event)?;
        }

        self.buffer.clear();

        if let Some(writer) = &self.writer {
            let mut writer = writer.lock().unwrap();
            writer.flush().map_err(|e| {
                crate::error::DbError::Io(format!("Failed to flush: {}", e))
            })?;
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

/// HTTP webhook connector
pub struct HttpWebhookConnector {
    config: ConnectorConfig,
    endpoint_url: String,
    headers: HashMap<String, String>,
    buffer: Vec<Event>,
    timeout: Duration,
}

impl HttpWebhookConnector {
    pub fn new(config: ConnectorConfig, endpoint_url: impl Into<String>) -> Self {
        let timeout = config
            .properties
            .get("timeout.ms")
            .and_then(|v| v.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30));

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // Extract custom headers from properties
        for (key, value) in &config.properties {
            if key.starts_with("header.") {
                let header_name = key.strip_prefix("header.").unwrap();
                headers.insert(header_name.to_string(), value.clone());
            }
        }

        Self {
            config,
            endpoint_url: endpoint_url.into(),
            headers,
            buffer: Vec::new(),
            timeout,
        }
    }

    fn send_http_request(&self, events: &[Event]) -> Result<()> {
        let payload = serde_json::to_string(events).map_err(|e| {
            crate::error::DbError::Serialization(format!("Failed to serialize: {}", e))
        })?;

        // In a real implementation, use an HTTP client library
        // For now, just simulate the request
        println!("Sending HTTP POST to {}", self.endpoint_url);
        println!("Payload: {}", payload);

        Ok(())
    }
}

impl SinkConnector for HttpWebhookConnector {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.flush()
    }

    fn write(&mut self, event: Event) -> Result<()> {
        self.buffer.push(event);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn write_batch(&mut self, events: Vec<Event>) -> Result<()> {
        self.buffer.extend(events);

        if self.buffer.len() >= self.config.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.send_http_request(&self.buffer)?;
        self.buffer.clear();

        Ok(())
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

/// Custom connector trait for user-defined connectors
pub trait CustomConnector: Send + Sync {
    fn initialize(&mut self, config: &ConnectorConfig) -> Result<()>;
    fn process(&mut self, event: Event) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

/// Connector manager
pub struct ConnectorManager {
    sources: RwLock<HashMap<String, Box<dyn SourceConnector>>>,
    sinks: RwLock<HashMap<String, Box<dyn SinkConnector>>>,
}

impl ConnectorManager {
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            sinks: RwLock::new(HashMap::new()),
        }
    }

    /// Register a source connector
    pub fn register_source(&self, connector: Box<dyn SourceConnector>) -> Result<()> {
        let name = connector.name().to_string();
        let mut sources = self.sources.write().unwrap();
        sources.insert(name, connector);
        Ok(())
    }

    /// Register a sink connector
    pub fn register_sink(&self, connector: Box<dyn SinkConnector>) -> Result<()> {
        let name = connector.name().to_string();
        let mut sinks = self.sinks.write().unwrap();
        sinks.insert(name, connector);
        Ok(())
    }

    /// Get a source connector
    pub fn get_source(&self, name: &str) -> Option<()> {
        let sources = self.sources.read().unwrap();
        sources.get(name)?;
        Some(())
    }

    /// Get a sink connector
    pub fn get_sink(&self, name: &str) -> Option<()> {
        let sinks = self.sinks.read().unwrap();
        sinks.get(name)?;
        Some(())
    }

    /// Start all connectors
    pub fn start_all(&self) -> Result<()> {
        let mut sources = self.sources.write().unwrap();
        for connector in sources.values_mut() {
            connector.start()?;
        }
        drop(sources);

        let mut sinks = self.sinks.write().unwrap();
        for connector in sinks.values_mut() {
            connector.start()?;
        }

        Ok(())
    }

    /// Stop all connectors
    pub fn stop_all(&self) -> Result<()> {
        let mut sources = self.sources.write().unwrap();
        for connector in sources.values_mut() {
            connector.stop()?;
        }
        drop(sources);

        let mut sinks = self.sinks.write().unwrap();
        for connector in sinks.values_mut() {
            connector.stop()?;
        }

        Ok(())
    }
}

impl Default for ConnectorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_kafka_connector() {
        let config = ConnectorConfig::default();
        let connector = KafkaConnector::new(config, "test_topic");

        assert_eq!(connector.topic, "test_topic");
        assert!(!connector.bootstrap_servers.is_empty());
    }

    #[test]
    fn test_jdbc_sink_connector() {
        let config = ConnectorConfig::default();
        let mut connector = JdbcSinkConnector::new(
            config,
            "jdbc:postgresql://localhost/test",
            "events",
        );

        connector.start().unwrap();

        let event = Event::new("test.event").with_payload("value", 42i64);
        connector.write(event).unwrap();

        connector.stop().unwrap();
    }

    #[test]
    fn test_file_sink_connector() {
        let config = ConnectorConfig::default();
        let temp_file = env::temp_dir().join("test_events.jsonl");

        let mut connector = FileSinkConnector::new(
            config,
            temp_file.clone(),
            FileFormat::JsonLines,
        );

        connector.start().unwrap();

        let event = Event::new("test.event").with_payload("value", 42i64);
        connector.write(event).unwrap();
        connector.flush().unwrap();

        connector.stop().unwrap();

        // Clean up
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_http_webhook_connector() {
        let config = ConnectorConfig::default();
        let mut connector = HttpWebhookConnector::new(config, "http://localhost:8080/webhook");

        connector.start().unwrap();

        let event = Event::new("test.event").with_payload("value", 42i64);
        connector.write(event).unwrap();

        connector.stop().unwrap();
    }

    #[test]
    fn test_connector_manager() {
        let manager = ConnectorManager::new();

        let config = ConnectorConfig::default();
        let connector = KafkaConnector::new(config.clone(), "test_topic");

        manager
            .register_source(Box::new(connector.as_source()))
            .unwrap();
        manager
            .register_sink(Box::new(connector.as_sink()))
            .unwrap();

        assert!(manager.get_source("default").is_some());
        assert!(manager.get_sink("default").is_some());
    }
}


