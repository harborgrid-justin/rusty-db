// Protocol Extensions Module
//
// Extensibility framework for custom protocol features and messages

use std::collections::HashMap;
use std::sync::Arc;

use super::message_types::MessageType;

// ============================================================================
// Extension Registry
// ============================================================================

pub type ExtensionId = u32;

pub trait ProtocolExtension: Send + Sync {
    fn id(&self) -> ExtensionId;
    fn name(&self) -> &str;
    fn version(&self) -> &str {
        "1.0.0"
    }
}

pub struct ExtensionRegistry {
    extensions: HashMap<ExtensionId, Arc<dyn ProtocolExtension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    pub fn register(&mut self, extension: Arc<dyn ProtocolExtension>) -> Result<(), String> {
        let id = extension.id();
        if self.extensions.contains_key(&id) {
            return Err(format!("Extension with ID {} already registered", id));
        }
        self.extensions.insert(id, extension);
        Ok(())
    }

    pub fn get(&self, id: ExtensionId) -> Option<&Arc<dyn ProtocolExtension>> {
        self.extensions.get(&id)
    }

    pub fn unregister(&mut self, id: ExtensionId) -> Option<Arc<dyn ProtocolExtension>> {
        self.extensions.remove(&id)
    }

    pub fn list(&self) -> Vec<ExtensionId> {
        self.extensions.keys().copied().collect()
    }

    pub fn count(&self) -> usize {
        self.extensions.len()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Feature Flags
// ============================================================================

#[derive(Debug, Clone)]
pub struct FeatureFlags {
    pub flags: HashMap<String, bool>,
}

impl FeatureFlags {
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
    }

    pub fn enable(&mut self, feature: String) {
        self.flags.insert(feature, true);
    }

    pub fn disable(&mut self, feature: String) {
        self.flags.insert(feature, false);
    }

    pub fn is_enabled(&self, feature: &str) -> bool {
        self.flags.get(feature).copied().unwrap_or(false)
    }

    pub fn set(&mut self, feature: String, enabled: bool) {
        self.flags.insert(feature, enabled);
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Custom Message Registry
// ============================================================================

pub struct CustomMessageRegistry {
    handlers: HashMap<MessageType, String>,
}

impl CustomMessageRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler(&mut self, message_type: MessageType, handler_name: String) {
        self.handlers.insert(message_type, handler_name);
    }

    pub fn get_handler(&self, message_type: &MessageType) -> Option<&String> {
        self.handlers.get(message_type)
    }

    pub fn unregister_handler(&mut self, message_type: MessageType) -> Option<String> {
        self.handlers.remove(&message_type)
    }
}

impl Default for CustomMessageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Extension Negotiation
// ============================================================================

pub struct ExtensionNegotiator;

impl ExtensionNegotiator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExtensionNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Protocol Manager
// ============================================================================

pub struct ProtocolManager {
    registry: ExtensionRegistry,
    features: FeatureFlags,
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            registry: ExtensionRegistry::new(),
            features: FeatureFlags::new(),
        }
    }

    pub fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.registry
    }

    pub fn features(&self) -> &FeatureFlags {
        &self.features
    }

    pub fn features_mut(&mut self) -> &mut FeatureFlags {
        &mut self.features
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolHealthStatus {
    pub healthy: bool,
    pub message: String,
}
