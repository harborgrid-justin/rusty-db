// # Port Mapping Service
//
// Service registry and port mapping for distributed database services.
//
// ## Features
//
// - **Service Registry**: Map services to ports
// - **Port Discovery**: Find ports by service name
// - **Well-Known Ports**: Standard port definitions
// - **Dynamic Advertisement**: Announce service ports to cluster

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Port mapping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Service type
    pub service_type: String,

    /// Port number
    pub port: u16,

    /// Service description
    pub description: String,

    /// When this mapping was registered
    pub registered_at: SystemTime,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PortMapping {
    /// Create a new port mapping
    pub fn new(service_type: String, port: u16, description: String) -> Self {
        Self {
            service_type,
            port,
            description,
            registered_at: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the mapping
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Well-known service ports
pub struct WellKnownPorts;

impl WellKnownPorts {
    /// PostgreSQL default port
    pub const POSTGRESQL: u16 = 5432;

    /// MySQL default port
    pub const MYSQL: u16 = 3306;

    /// HTTP default port
    pub const HTTP: u16 = 80;

    /// HTTPS default port
    pub const HTTPS: u16 = 443;

    /// Redis default port
    pub const REDIS: u16 = 6379;

    /// MongoDB default port
    pub const MONGODB: u16 = 27017;

    /// RustyDB default database port
    pub const RUSTYDB_DATABASE: u16 = 5432;

    /// RustyDB cluster communication port
    pub const RUSTYDB_CLUSTER: u16 = 5433;

    /// RustyDB replication port
    pub const RUSTYDB_REPLICATION: u16 = 5434;

    /// RustyDB admin/monitoring port
    pub const RUSTYDB_ADMIN: u16 = 8080;

    /// RustyDB API gateway port
    pub const RUSTYDB_API: u16 = 8081;

    /// Get service name for a well-known port
    pub fn get_service_name(port: u16) -> Option<&'static str> {
        match port {
            Self::POSTGRESQL => Some("PostgreSQL/RustyDB"), // Both use 5432
            Self::MYSQL => Some("MySQL"),
            Self::HTTP => Some("HTTP"),
            Self::HTTPS => Some("HTTPS"),
            Self::REDIS => Some("Redis"),
            Self::MONGODB => Some("MongoDB"),
            // RUSTYDB_DATABASE omitted - same as POSTGRESQL (5432)
            Self::RUSTYDB_CLUSTER => Some("RustyDB Cluster"),
            Self::RUSTYDB_REPLICATION => Some("RustyDB Replication"),
            Self::RUSTYDB_ADMIN => Some("RustyDB Admin"),
            Self::RUSTYDB_API => Some("RustyDB API"),
            _ => None,
        }
    }

    /// Check if a port is well-known
    pub fn is_well_known(port: u16) -> bool {
        Self::get_service_name(port).is_some()
    }
}

/// Service registry for port mappings
pub struct ServiceRegistry {
    /// Map of service type to port mapping
    services: Arc<RwLock<HashMap<String, PortMapping>>>,

    /// Map of port to service type
    ports: Arc<RwLock<HashMap<u16, String>>>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            ports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a service
    pub async fn register(&self, mapping: PortMapping) -> Result<()> {
        let service_type = mapping.service_type.clone();
        let port = mapping.port;

        let mut services = self.services.write().await;
        let mut ports = self.ports.write().await;

        // Check if port is already in use by a different service
        if let Some(existing_service) = ports.get(&port) {
            if existing_service != &service_type {
                return Err(DbError::AlreadyExists(format!(
                    "Port {} already registered to service {}",
                    port, existing_service
                )));
            }
        }

        services.insert(service_type.clone(), mapping);
        ports.insert(port, service_type);

        Ok(())
    }

    /// Unregister a service by service type
    pub async fn unregister(&self, service_type: &str) -> Result<()> {
        let mut services = self.services.write().await;
        let mut ports = self.ports.write().await;

        if let Some(mapping) = services.remove(service_type) {
            ports.remove(&mapping.port);
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Service {} not registered",
                service_type
            )))
        }
    }

    /// Unregister by port
    pub async fn unregister_port(&self, port: u16) -> Result<()> {
        let mut services = self.services.write().await;
        let mut ports = self.ports.write().await;

        if let Some(service_type) = ports.remove(&port) {
            services.remove(&service_type);
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "No service registered on port {}",
                port
            )))
        }
    }

    /// Get service by service type
    pub async fn get_service(&self, service_type: &str) -> Option<PortMapping> {
        let services = self.services.read().await;
        services.get(service_type).cloned()
    }

    /// Get service by port
    pub async fn get_by_port(&self, port: u16) -> Option<PortMapping> {
        let ports = self.ports.read().await;
        let services = self.services.read().await;

        ports
            .get(&port)
            .and_then(|service_type| services.get(service_type))
            .cloned()
    }

    /// Get all registered services
    pub async fn get_all(&self) -> Vec<PortMapping> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    /// Get all registered ports
    pub async fn get_all_ports(&self) -> Vec<u16> {
        let ports = self.ports.read().await;
        ports.keys().copied().collect()
    }

    /// Check if a service is registered
    pub async fn is_registered(&self, service_type: &str) -> bool {
        let services = self.services.read().await;
        services.contains_key(service_type)
    }

    /// Check if a port is in use
    pub async fn is_port_used(&self, port: u16) -> bool {
        let ports = self.ports.read().await;
        ports.contains_key(&port)
    }

    /// Get service count
    pub async fn count(&self) -> usize {
        let services = self.services.read().await;
        services.len()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Port mapping service
pub struct PortMappingService {
    registry: ServiceRegistry,
}

impl PortMappingService {
    /// Create a new port mapping service
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
        }
    }

    /// Register a port for a service type
    pub fn register_port(
        &mut self,
        service_type: impl Into<String>,
        port: u16,
        description: impl Into<String>,
    ) {
        let service_type = service_type.into();
        let description = description.into();

        let mapping = PortMapping::new(service_type, port, description);

        // Spawn async task to register (fire and forget for now)
        let registry = self.registry.services.clone();
        let ports = self.registry.ports.clone();

        tokio::spawn(async move {
            let service_type = mapping.service_type.clone();
            let port = mapping.port;

            let mut services = registry.write().await;
            let mut ports_guard = ports.write().await;

            services.insert(service_type.clone(), mapping);
            ports_guard.insert(port, service_type);
        });
    }

    /// Unregister a port
    pub fn unregister_port(&mut self, port: u16) {
        let registry = self.registry.services.clone();
        let ports = self.registry.ports.clone();

        tokio::spawn(async move {
            let mut services = registry.write().await;
            let mut ports_guard = ports.write().await;

            if let Some(service_type) = ports_guard.remove(&port) {
                services.remove(&service_type);
            }
        });
    }

    /// Get port mapping
    pub fn get_mapping(&self, _port: u16) -> Option<PortMapping> {
        // Since this is synchronous, we need to block or return None
        // For now, return None and use async methods instead
        None
    }

    /// Get service registry reference
    pub fn registry(&self) -> &ServiceRegistry {
        &self.registry
    }

    /// Initialize default RustyDB service ports
    pub async fn initialize_defaults(&self) -> Result<()> {
        let defaults = vec![
            PortMapping::new(
                "Database".to_string(),
                WellKnownPorts::RUSTYDB_DATABASE,
                "Main database service".to_string(),
            ),
            PortMapping::new(
                "Cluster".to_string(),
                WellKnownPorts::RUSTYDB_CLUSTER,
                "Cluster communication".to_string(),
            ),
            PortMapping::new(
                "Replication".to_string(),
                WellKnownPorts::RUSTYDB_REPLICATION,
                "Replication service".to_string(),
            ),
            PortMapping::new(
                "Admin".to_string(),
                WellKnownPorts::RUSTYDB_ADMIN,
                "Admin and monitoring".to_string(),
            ),
            PortMapping::new(
                "API".to_string(),
                WellKnownPorts::RUSTYDB_API,
                "API gateway".to_string(),
            ),
        ];

        for mapping in defaults {
            self.registry.register(mapping).await?;
        }

        Ok(())
    }

    /// Get service port by service type
    pub async fn get_service_port(&self, service_type: &str) -> Option<u16> {
        self.registry
            .get_service(service_type)
            .await
            .map(|m| m.port)
    }

    /// List all registered services
    pub async fn list_services(&self) -> Vec<PortMapping> {
        self.registry.get_all().await
    }

    /// Export mappings as JSON
    pub async fn export_json(&self) -> Result<String> {
        let mappings = self.registry.get_all().await;
        serde_json::to_string_pretty(&mappings)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize mappings: {}", e)))
    }
}

impl Default for PortMappingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_well_known_ports() {
        assert_eq!(WellKnownPorts::POSTGRESQL, 5432);
        assert_eq!(WellKnownPorts::MYSQL, 3306);
        assert_eq!(WellKnownPorts::RUSTYDB_DATABASE, 5432);

        assert_eq!(WellKnownPorts::get_service_name(5432), Some("PostgreSQL"));
        assert!(WellKnownPorts::is_well_known(5432));
        assert!(!WellKnownPorts::is_well_known(9999));
    }

    #[test]
    fn test_port_mapping() {
        let mapping = PortMapping::new(
            "TestService".to_string(),
            8080,
            "Test service description".to_string(),
        );

        assert_eq!(mapping.service_type, "TestService");
        assert_eq!(mapping.port, 8080);
        assert_eq!(mapping.description, "Test service description");
    }

    #[test]
    fn test_port_mapping_with_metadata() {
        let mapping = PortMapping::new("TestService".to_string(), 8080, "Test service".to_string())
            .with_metadata("version".to_string(), "1.0".to_string())
            .with_metadata("protocol".to_string(), "TCP".to_string());

        assert_eq!(mapping.get_metadata("version"), Some(&"1.0".to_string()));
        assert_eq!(mapping.get_metadata("protocol"), Some(&"TCP".to_string()));
        assert_eq!(mapping.get_metadata("nonexistent"), None);
    }

    #[tokio::test]
    async fn test_service_registry() {
        let registry = ServiceRegistry::new();

        let mapping = PortMapping::new("Database".to_string(), 5432, "Main database".to_string());

        registry.register(mapping).await.unwrap();

        assert!(registry.is_registered("Database").await);
        assert!(registry.is_port_used(5432).await);
        assert_eq!(registry.count().await, 1);

        let retrieved = registry.get_service("Database").await.unwrap();
        assert_eq!(retrieved.port, 5432);
    }

    #[tokio::test]
    async fn test_registry_port_conflict() {
        let registry = ServiceRegistry::new();

        let mapping1 = PortMapping::new("Service1".to_string(), 8080, "First service".to_string());

        registry.register(mapping1).await.unwrap();

        let mapping2 = PortMapping::new("Service2".to_string(), 8080, "Second service".to_string());

        let result = registry.register(mapping2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ServiceRegistry::new();

        let mapping = PortMapping::new("TestService".to_string(), 9000, "Test".to_string());

        registry.register(mapping).await.unwrap();
        assert_eq!(registry.count().await, 1);

        registry.unregister("TestService").await.unwrap();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_port_mapping_service() {
        let service = PortMappingService::new();

        service.initialize_defaults().await.unwrap();

        let port = service.get_service_port("Database").await;
        assert_eq!(port, Some(5432));

        let services = service.list_services().await;
        assert!(services.len() >= 5);
    }

    #[tokio::test]
    async fn test_export_json() {
        let service = PortMappingService::new();

        service.initialize_defaults().await.unwrap();

        let json = service.export_json().await.unwrap();
        assert!(json.contains("Database"));
        assert!(json.contains("5432"));
    }
}
