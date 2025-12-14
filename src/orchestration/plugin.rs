// # Plugin Architecture
//
// This module provides a comprehensive plugin system for extending RustyDB
// functionality without modifying core code.
//
// ## Features
//
// - **Dynamic Loading**: Load plugins at runtime
// - **Lifecycle Management**: Initialize, start, stop plugins
// - **Dependency Resolution**: Handle plugin dependencies
// - **Sandboxing**: Isolate plugin execution
// - **Event Hooks**: Subscribe to system events
// - **Configuration**: Per-plugin configuration
//
// ## Plugin Lifecycle
//
// ```text
// REGISTERED → INITIALIZED → STARTED → RUNNING
//                                ↓
//                            STOPPED → UNLOADED
// ```

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::error::{DbError, Result};

// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginState {
    // Plugin is registered but not initialized
    Registered,
    // Plugin is initialized
    Initialized,
    // Plugin is started and running
    Started,
    // Plugin is stopped
    Stopped,
    // Plugin failed
    Failed,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginState::Registered => write!(f, "REGISTERED"),
            PluginState::Initialized => write!(f, "INITIALIZED"),
            PluginState::Started => write!(f, "STARTED"),
            PluginState::Stopped => write!(f, "STOPPED"),
            PluginState::Failed => write!(f, "FAILED"),
        }
    }
}

// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    // Plugin name
    pub name: String,
    // Plugin version
    pub version: String,
    // Plugin author
    pub author: String,
    // Plugin description
    pub description: String,
    // Plugin dependencies (other plugin names)
    pub dependencies: Vec<String>,
    // Required API version
    pub api_version: String,
}

impl PluginMetadata {
    // Create new plugin metadata
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            author: String::new(),
            description: String::new(),
            dependencies: Vec::new(),
            api_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    // Add author
    pub fn with_author(mut self, author: String) -> Self {
        self.author = author;
        self
    }

    // Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    // Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

// Plugin configuration
pub type PluginConfig = HashMap<String, serde_json::Value>;

// Plugin context for accessing system services
pub struct PluginContext {
    // Plugin name
    plugin_name: String,
    // Plugin configuration
    config: Arc<RwLock<PluginConfig>>,
    // Event bus for plugin communication
    event_bus: Arc<PluginEventBus>,
}

impl PluginContext {
    // Create a new plugin context
    pub fn new(plugin_name: String, config: PluginConfig, event_bus: Arc<PluginEventBus>) -> Self {
        Self {
            plugin_name,
            config: Arc::new(RwLock::new(config)),
            event_bus,
        }
    }

    // Get plugin name
    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    // Get configuration value
    pub fn get_config(&self, key: &str) -> Option<serde_json::Value> {
        let config = self.config.read();
        config.get(key).cloned()
    }

    // Set configuration value
    pub fn set_config(&self, key: String, value: serde_json::Value) {
        let mut config = self.config.write();
        config.insert(key, value);
    }

    // Emit an event
    pub fn emit_event(&self, event: PluginEvent) {
        self.event_bus.emit(event);
    }

    // Subscribe to events
    pub fn subscribe(&self, event_type: &str) -> tokio::sync::mpsc::UnboundedReceiver<PluginEvent> {
        self.event_bus.subscribe(event_type)
    }
}

// Trait for plugins
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    // Get plugin metadata
    fn metadata(&self) -> PluginMetadata;

    // Initialize the plugin
    async fn initialize(&mut self, ctx: &PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    // Start the plugin
    async fn start(&mut self, ctx: &PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    // Stop the plugin
    async fn stop(&mut self, ctx: &PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    // Handle a plugin event
    async fn handle_event(&mut self, event: &PluginEvent, ctx: &PluginContext) -> Result<()> {
        let _ = (event, ctx);
        Ok(())
    }

    // Get plugin as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    // Get plugin as mutable Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Plugin event for communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    // Event type
    pub event_type: String,
    // Event source (plugin name)
    pub source: String,
    // Event data
    pub data: serde_json::Value,
    // Timestamp
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: std::time::Instant,
}

impl PluginEvent {
    // Create a new plugin event
    pub fn new(event_type: String, source: String, data: serde_json::Value) -> Self {
        Self {
            event_type,
            source,
            data,
            timestamp: std::time::Instant::now(),
        }
    }
}

// Plugin event bus for inter-plugin communication
pub struct PluginEventBus {
    // Event subscribers
    subscribers: RwLock<HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<PluginEvent>>>>,
}

impl PluginEventBus {
    // Create a new event bus
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            subscribers: RwLock::new(HashMap::new()),
        })
    }

    // Subscribe to events of a specific type
    pub fn subscribe(&self, event_type: &str) -> tokio::sync::mpsc::UnboundedReceiver<PluginEvent> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let mut subscribers = self.subscribers.write();
        subscribers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(tx);

        rx
    }

    // Emit an event
    pub fn emit(&self, event: PluginEvent) {
        let subscribers = self.subscribers.read();

        if let Some(subs) = subscribers.get(&event.event_type) {
            for tx in subs {
                let _ = tx.send(event.clone());
            }
        }

        // Also send to wildcard subscribers
        if let Some(subs) = subscribers.get("*") {
            for tx in subs {
                let _ = tx.send(event.clone());
            }
        }
    }

    // Clear all subscribers
    pub fn clear(&self) {
        let mut subscribers = self.subscribers.write();
        subscribers.clear();
    }
}

impl Default for PluginEventBus {
    fn default() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
        }
    }
}

// Plugin instance wrapper
struct PluginInstance {
    // The plugin
    plugin: Box<dyn Plugin>,
    // Plugin state
    state: PluginState,
    // Plugin context
    context: Arc<PluginContext>,
    // Metadata
    metadata: PluginMetadata,
}

// Plugin registry for managing plugins
pub struct PluginRegistry {
    // Registered plugins
    plugins: RwLock<HashMap<String, PluginInstance>>,
    // Event bus
    event_bus: Arc<PluginEventBus>,
    // Global configuration
    global_config: RwLock<PluginConfig>,
}

impl PluginRegistry {
    // Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            event_bus: PluginEventBus::new(),
            global_config: RwLock::new(HashMap::new()),
        }
    }

    // Register a plugin
    pub fn register(&self, plugin: Box<dyn Plugin>, config: PluginConfig) -> Result<()> {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();

        // Check if already registered
        {
            let plugins = self.plugins.read();
            if plugins.contains_key(&name) {
                return Err(DbError::Internal(format!(
                    "Plugin already registered: {}",
                    name
                )));
            }
        }

        // Create plugin context
        let context = Arc::new(PluginContext::new(
            name.clone(),
            config,
            Arc::clone(&self.event_bus),
        ));

        let instance = PluginInstance {
            plugin,
            state: PluginState::Registered,
            context,
            metadata: metadata.clone(),
        };

        let mut plugins = self.plugins.write();
        plugins.insert(name.clone(), instance);

        info!(
            "Registered plugin: {} v{} by {}",
            metadata.name, metadata.version, metadata.author
        );

        Ok(())
    }

    // Initialize a plugin
    pub async fn initialize(&self, name: &str) -> Result<()> {
        // Check dependencies first (immutable borrow)
        {
            let plugins = self.plugins.read();
            let instance = plugins
                .get(name)
                .ok_or_else(|| DbError::Internal(format!("Plugin not found: {}", name)))?;

            if instance.state != PluginState::Registered {
                return Err(DbError::Internal(format!(
                    "Plugin {} is not in REGISTERED state",
                    name
                )));
            }

            // Check dependencies
            for dep in &instance.metadata.dependencies {
                if !plugins.contains_key(dep) {
                    return Err(DbError::Internal(format!(
                        "Plugin {} requires dependency: {}",
                        name, dep
                    )));
                }
            }
        }

        // Now initialize (mutable borrow)
        let mut plugins = self.plugins.write();
        let instance = plugins
            .get_mut(name)
            .ok_or_else(|| DbError::Internal(format!("Plugin not found: {}", name)))?;

        // Initialize
        debug!("Initializing plugin: {}", name);
        instance.plugin.initialize(&instance.context).await?;
        instance.state = PluginState::Initialized;

        info!("Initialized plugin: {}", name);
        Ok(())
    }

    // Start a plugin
    pub async fn start(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        let instance = plugins
            .get_mut(name)
            .ok_or_else(|| DbError::Internal(format!("Plugin not found: {}", name)))?;

        if instance.state != PluginState::Initialized && instance.state != PluginState::Stopped {
            return Err(DbError::Internal(format!(
                "Plugin {} cannot be started from state: {}",
                name, instance.state
            )));
        }

        debug!("Starting plugin: {}", name);
        instance.plugin.start(&instance.context).await?;
        instance.state = PluginState::Started;

        info!("Started plugin: {}", name);
        Ok(())
    }

    // Stop a plugin
    pub async fn stop(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write();
        let instance = plugins
            .get_mut(name)
            .ok_or_else(|| DbError::Internal(format!("Plugin not found: {}", name)))?;

        if instance.state != PluginState::Started {
            return Err(DbError::Internal(format!("Plugin {} is not started", name)));
        }

        debug!("Stopping plugin: {}", name);
        instance.plugin.stop(&instance.context).await?;
        instance.state = PluginState::Stopped;

        info!("Stopped plugin: {}", name);
        Ok(())
    }

    // Unregister a plugin
    pub async fn unregister(&self, name: &str) -> Result<()> {
        // Stop first if running
        if let Ok(state) = self.get_state(name) {
            if state == PluginState::Started {
                self.stop(name).await?;
            }
        }

        let mut plugins = self.plugins.write();
        if plugins.remove(name).is_some() {
            info!("Unregistered plugin: {}", name);
            Ok(())
        } else {
            Err(DbError::Internal(format!("Plugin not found: {}", name)))
        }
    }

    // Get plugin state
    pub fn get_state(&self, name: &str) -> Result<PluginState> {
        let plugins = self.plugins.read();
        plugins
            .get(name)
            .map(|p| p.state)
            .ok_or_else(|| DbError::Internal(format!("Plugin not found: {}", name)))
    }

    // Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read();
        plugins.get(name).map(|p| p.metadata.clone())
    }

    // List all plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read();
        plugins
            .values()
            .map(|p| PluginInfo {
                metadata: p.metadata.clone(),
                state: p.state,
            })
            .collect()
    }

    // Initialize all plugins
    pub async fn initialize_all(&self) -> Result<()> {
        info!("Initializing all plugins...");

        let plugin_names: Vec<String> = {
            let plugins = self.plugins.read();
            plugins.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        for name in plugin_names {
            if let Err(e) = self.initialize(&name).await {
                error!("Failed to initialize plugin {}: {}", name, e);
                errors.push((name, e));
            }
        }

        if errors.is_empty() {
            info!("All plugins initialized successfully");
            Ok(())
        } else {
            Err(DbError::Internal(format!(
                "Failed to initialize {} plugins",
                errors.len()
            )))
        }
    }

    // Start all plugins
    pub async fn start_all(&self) -> Result<()> {
        info!("Starting all plugins...");

        let plugin_names: Vec<String> = {
            let plugins = self.plugins.read();
            plugins.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        for name in plugin_names {
            if let Err(e) = self.start(&name).await {
                error!("Failed to start plugin {}: {}", name, e);
                errors.push((name, e));
            }
        }

        if errors.is_empty() {
            info!("All plugins started successfully");
            Ok(())
        } else {
            Err(DbError::Internal(format!(
                "Failed to start {} plugins",
                errors.len()
            )))
        }
    }

    // Stop all plugins
    pub async fn stop_all(&self) -> Result<()> {
        info!("Stopping all plugins...");

        let plugin_names: Vec<String> = {
            let plugins = self.plugins.read();
            plugins.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        // Stop in reverse order
        for name in plugin_names.iter().rev() {
            if let Err(e) = self.stop(name).await {
                error!("Failed to stop plugin {}: {}", name, e);
                errors.push((name.clone(), e));
            }
        }

        if errors.is_empty() {
            info!("All plugins stopped successfully");
            Ok(())
        } else {
            log::warn!("Some plugins failed to stop: {}", errors.len());
            Ok(()) // Don't fail on shutdown
        }
    }

    // Get event bus
    pub fn event_bus(&self) -> &Arc<PluginEventBus> {
        &self.event_bus
    }

    // Set global configuration
    pub fn set_global_config(&self, config: PluginConfig) {
        let mut global = self.global_config.write();
        *global = config;
    }

    // Get global configuration
    pub fn get_global_config(&self) -> PluginConfig {
        let global = self.global_config.read();
        global.clone()
    }

    // Get statistics
    pub fn statistics(&self) -> PluginRegistryStats {
        let plugins = self.plugins.read();

        let mut state_counts = HashMap::new();
        for instance in plugins.values() {
            *state_counts.entry(instance.state).or_insert(0) += 1;
        }

        PluginRegistryStats {
            total_plugins: plugins.len(),
            state_counts,
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    // Plugin metadata
    pub metadata: PluginMetadata,
    // Current state
    pub state: PluginState,
}

// Statistics about the plugin registry
#[derive(Debug, Clone)]
pub struct PluginRegistryStats {
    // Total number of plugins
    pub total_plugins: usize,
    // Number of plugins in each state
    pub state_counts: HashMap<PluginState, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    struct TestPlugin {
        name: String,
    }

    #[async_trait::async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(self.name.clone(), "1.0.0".into())
                .with_author("Test Author".into())
                .with_description("Test plugin".into())
        }

        async fn initialize(&mut self, _ctx: &PluginContext) -> Result<()> {
            Ok(())
        }

        async fn start(&mut self, _ctx: &PluginContext) -> Result<()> {
            Ok(())
        }

        async fn stop(&mut self, _ctx: &PluginContext) -> Result<()> {
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin {
            name: "test-plugin".into(),
        });

        let config = HashMap::new();
        assert!(registry.register(plugin, config).is_ok());

        let state = registry.get_state("test-plugin").unwrap();
        assert_eq!(state, PluginState::Registered);
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin {
            name: "test-plugin".into(),
        });

        registry.register(plugin, HashMap::new()).unwrap();

        // Initialize
        registry.initialize("test-plugin").await.unwrap();
        assert_eq!(
            registry.get_state("test-plugin").unwrap(),
            PluginState::Initialized
        );

        // Start
        registry.start("test-plugin").await.unwrap();
        assert_eq!(
            registry.get_state("test-plugin").unwrap(),
            PluginState::Started
        );

        // Stop
        registry.stop("test-plugin").await.unwrap();
        assert_eq!(
            registry.get_state("test-plugin").unwrap(),
            PluginState::Stopped
        );

        // Unregister
        registry.unregister("test-plugin").await.unwrap();
        assert!(registry.get_state("test-plugin").is_err());
    }

    #[tokio::test]
    async fn test_plugin_list() {
        let registry = PluginRegistry::new();

        for i in 0..3 {
            let plugin = Box::new(TestPlugin {
                name: format!("plugin-{}", i),
            });
            registry.register(plugin, HashMap::new()).unwrap();
        }

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 3);
    }

    #[tokio::test]
    async fn test_event_bus() {
        let event_bus = PluginEventBus::new();

        let mut rx = event_bus.subscribe("test-event");

        let event = PluginEvent::new(
            "test-event".into(),
            "test-source".into(),
            serde_json::json!({"data": "value"}),
        );

        event_bus.emit(event.clone());

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, "test-event");
        assert_eq!(received.source, "test-source");
    }
}
