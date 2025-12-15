// # Global Data Services (GDS)
//
// Region-aware routing, load balancing, service failover,
// and latency-based routing for global database deployments.

use crate::error::DbError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, DbError>;

/// Global service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalService {
    /// Service name
    pub name: String,
    /// Service regions
    pub regions: Vec<ServiceRegion>,
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    /// Failover policy
    pub failover_policy: FailoverPolicy,
    /// Service state
    pub state: ServiceState,
    /// Creation time
    pub created_at: u64,
}

/// Service region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegion {
    /// Region identifier
    pub region_id: String,
    /// Region name
    pub name: String,
    /// Geographic location
    pub location: GeoLocation,
    /// Databases in this region
    pub databases: Vec<DatabaseInstance>,
    /// Region role
    pub role: RegionRole,
    /// Health status
    pub health: HealthStatus,
    /// Average latency to other regions
    pub latencies: HashMap<String, u64>,
}

/// Geographic location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub country: String,
    pub city: String,
}

/// Database instance in a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInstance {
    /// Instance ID
    pub id: String,
    /// Host address
    pub host: String,
    /// Port
    pub port: u16,
    /// Instance role
    pub role: InstanceRole,
    /// Connection pool size
    pub pool_size: usize,
    /// Active connections
    pub active_connections: usize,
    /// Instance health
    pub health: HealthStatus,
    /// Last health check
    pub last_health_check: u64,
}

/// Region role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegionRole {
    /// Primary region for writes
    Primary,
    /// Standby region for failover
    Standby,
    /// Read-only region
    ReadOnly,
    /// Disaster recovery region
    DisasterRecovery,
}

/// Instance role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstanceRole {
    /// Primary (read-write)
    Primary,
    /// Replica (read-only)
    Replica,
    /// Standby (for failover)
    Standby,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Load balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round-robin across instances
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Least latency
    LeastLatency,
    /// Weighted distribution
    Weighted(HashMap<String, u32>),
    /// Locality-aware (prefer closest region)
    LocalityAware,
    /// Custom strategy
    Custom(String),
}

/// Failover policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverPolicy {
    /// Automatic failover enabled
    pub auto_failover: bool,
    /// Failover timeout (ms)
    pub timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Failover priority order
    pub priority_order: Vec<String>,
}

/// Service state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceState {
    Active,
    Degraded,
    FailingOver,
    Offline,
}

/// Connection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    /// Request ID
    pub id: String,
    /// Client location (if known)
    pub client_location: Option<GeoLocation>,
    /// Request type
    pub request_type: RequestType,
    /// Priority
    pub priority: u32,
    /// Timestamp
    pub timestamp: u64,
}

/// Type of database request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestType {
    Read,
    Write,
    Transaction,
}

/// Connection routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected region
    pub region_id: String,
    /// Selected instance
    pub instance_id: String,
    /// Decision reason
    pub reason: String,
    /// Estimated latency
    pub estimated_latency_ms: u64,
}

/// Global Data Services manager
pub struct GlobalDataServices {
    /// Registered services
    services: Arc<RwLock<HashMap<String, GlobalService>>>,
    /// Routing statistics
    stats: Arc<RwLock<GdsStats>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, Vec<ActiveConnection>>>>,
    /// Region latency matrix
    _latency_matrix: Arc<RwLock<BTreeMap<(String, String), u64>>>,
}

/// Active connection tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveConnection {
    connection_id: String,
    instance_id: String,
    established_at: u64,
    last_activity: u64,
}

/// GDS statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GdsStats {
    pub total_requests: u64,
    pub read_requests: u64,
    pub write_requests: u64,
    pub failovers: u64,
    pub avg_routing_time_ms: f64,
    pub requests_by_region: HashMap<String, u64>,
    pub requests_by_strategy: HashMap<String, u64>,
}

impl GlobalDataServices {
    /// Create a new GDS manager
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(GdsStats::default())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            _latency_matrix: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Register a global service
    pub fn register_service(&self, service: GlobalService) -> Result<()> {
        let mut services = self.services.write();

        if services.contains_key(&service.name) {
            return Err(DbError::Replication(format!(
                "Service {} already registered",
                service.name
            )));
        }

        services.insert(service.name.clone(), service);
        Ok(())
    }

    /// Route a connection request
    pub fn route_request(
        &self,
        service_name: &str,
        request: &ConnectionRequest,
    ) -> Result<RoutingDecision> {
        let start = Self::current_timestamp();

        let services = self.services.read();
        let service = services
            .get(service_name)
            .ok_or_else(|| DbError::Replication(format!("Service {} not found", service_name)))?;

        let decision = match request.request_type {
            RequestType::Write | RequestType::Transaction => {
                // Route to primary region
                self.route_to_primary(service, request)?
            }
            RequestType::Read => {
                // Apply load balancing strategy
                self.apply_load_balancing(service, request)?
            }
        };

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_requests += 1;

            match request.request_type {
                RequestType::Read => stats.read_requests += 1,
                RequestType::Write | RequestType::Transaction => stats.write_requests += 1,
            }

            *stats
                .requests_by_region
                .entry(decision.region_id.clone())
                .or_insert(0) += 1;

            let strategy_key = format!("{:?}", service.load_balancing);
            *stats.requests_by_strategy.entry(strategy_key).or_insert(0) += 1;

            let elapsed = Self::current_timestamp() - start;
            stats.avg_routing_time_ms =
                (stats.avg_routing_time_ms * (stats.total_requests - 1) as f64 + elapsed as f64)
                    / stats.total_requests as f64;
        }

        Ok(decision)
    }

    /// Route to primary region
    fn route_to_primary(
        &self,
        service: &GlobalService,
        _request: &ConnectionRequest,
    ) -> Result<RoutingDecision> {
        // Find primary region
        for region in &service.regions {
            if region.role == RegionRole::Primary && region.health == HealthStatus::Healthy {
                // Find primary instance in region
                for db in &region.databases {
                    if db.role == InstanceRole::Primary && db.health == HealthStatus::Healthy {
                        return Ok(RoutingDecision {
                            region_id: region.region_id.clone(),
                            instance_id: db.id.clone(),
                            reason: "Primary region and instance".to_string(),
                            estimated_latency_ms: 0,
                        });
                    }
                }
            }
        }

        Err(DbError::Replication(
            "No healthy primary region found".to_string(),
        ))
    }

    /// Apply load balancing strategy
    fn apply_load_balancing(
        &self,
        service: &GlobalService,
        request: &ConnectionRequest,
    ) -> Result<RoutingDecision> {
        match &service.load_balancing {
            LoadBalancingStrategy::RoundRobin => self.round_robin_routing(service),
            LoadBalancingStrategy::LeastConnections => self.least_connections_routing(service),
            LoadBalancingStrategy::LeastLatency => self.least_latency_routing(service, request),
            LoadBalancingStrategy::LocalityAware => self.locality_aware_routing(service, request),
            LoadBalancingStrategy::Weighted(weights) => self.weighted_routing(service, weights),
            LoadBalancingStrategy::Custom(_) => {
                // Would call custom routing logic
                self.round_robin_routing(service)
            }
        }
    }

    /// Round-robin load balancing
    fn round_robin_routing(&self, service: &GlobalService) -> Result<RoutingDecision> {
        // Simple round-robin across all healthy instances
        for region in &service.regions {
            if region.health != HealthStatus::Healthy {
                continue;
            }

            for db in &region.databases {
                if db.health == HealthStatus::Healthy {
                    return Ok(RoutingDecision {
                        region_id: region.region_id.clone(),
                        instance_id: db.id.clone(),
                        reason: "Round-robin selection".to_string(),
                        estimated_latency_ms: 10,
                    });
                }
            }
        }

        Err(DbError::Replication(
            "No healthy instances found".to_string(),
        ))
    }

    /// Least connections load balancing
    fn least_connections_routing(&self, service: &GlobalService) -> Result<RoutingDecision> {
        let connections = self.connections.read();

        let mut min_connections = usize::MAX;
        let mut selected_region = None;
        let mut selected_instance = None;

        for region in &service.regions {
            if region.health != HealthStatus::Healthy {
                continue;
            }

            for db in &region.databases {
                if db.health != HealthStatus::Healthy {
                    continue;
                }

                let conn_count = connections.get(&db.id).map(|c| c.len()).unwrap_or(0);

                if conn_count < min_connections {
                    min_connections = conn_count;
                    selected_region = Some(region.clone());
                    selected_instance = Some(db.clone());
                }
            }
        }

        if let (Some(region), Some(instance)) = (selected_region, selected_instance) {
            Ok(RoutingDecision {
                region_id: region.region_id.clone(),
                instance_id: instance.id.clone(),
                reason: format!("Least connections ({})", min_connections),
                estimated_latency_ms: 10,
            })
        } else {
            Err(DbError::Replication(
                "No healthy instances found".to_string(),
            ))
        }
    }

    /// Least latency load balancing
    fn least_latency_routing(
        &self,
        service: &GlobalService,
        request: &ConnectionRequest,
    ) -> Result<RoutingDecision> {
        if let Some(ref client_loc) = request.client_location {
            // Calculate distance to each region
            let mut min_latency = u64::MAX;
            let mut selected_region = None;
            let mut selected_instance = None;

            for region in &service.regions {
                if region.health != HealthStatus::Healthy {
                    continue;
                }

                let latency = self.calculate_latency(client_loc, &region.location);

                if latency < min_latency {
                    // Find a healthy instance in this region
                    for db in &region.databases {
                        if db.health == HealthStatus::Healthy {
                            min_latency = latency;
                            selected_region = Some(region.clone());
                            selected_instance = Some(db.clone());
                            break;
                        }
                    }
                }
            }

            if let (Some(region), Some(instance)) = (selected_region, selected_instance) {
                Ok(RoutingDecision {
                    region_id: region.region_id.clone(),
                    instance_id: instance.id.clone(),
                    reason: "Lowest latency".to_string(),
                    estimated_latency_ms: min_latency,
                })
            } else {
                Err(DbError::Replication(
                    "No healthy instances found".to_string(),
                ))
            }
        } else {
            // Fall back to round-robin if no client location
            self.round_robin_routing(service)
        }
    }

    /// Locality-aware routing (prefer same region as client)
    fn locality_aware_routing(
        &self,
        service: &GlobalService,
        request: &ConnectionRequest,
    ) -> Result<RoutingDecision> {
        if let Some(ref client_loc) = request.client_location {
            // Find closest region
            let mut closest_region = None;
            let mut min_distance = f64::MAX;

            for region in &service.regions {
                if region.health != HealthStatus::Healthy {
                    continue;
                }

                let distance = self.calculate_distance(client_loc, &region.location);

                if distance < min_distance {
                    min_distance = distance;
                    closest_region = Some(region);
                }
            }

            if let Some(region) = closest_region {
                // Find a healthy instance in this region
                for db in &region.databases {
                    if db.health == HealthStatus::Healthy {
                        return Ok(RoutingDecision {
                            region_id: region.region_id.clone(),
                            instance_id: db.id.clone(),
                            reason: "Closest region".to_string(),
                            estimated_latency_ms: (min_distance * 10.0) as u64,
                        });
                    }
                }
            }
        }

        // Fall back to round-robin
        self.round_robin_routing(service)
    }

    /// Weighted load balancing
    fn weighted_routing(
        &self,
        service: &GlobalService,
        _weights: &HashMap<String, u32>,
    ) -> Result<RoutingDecision> {
        // Simple implementation - would use weights in production
        self.round_robin_routing(service)
    }

    /// Calculate latency between two locations
    fn calculate_latency(&self, loc1: &GeoLocation, loc2: &GeoLocation) -> u64 {
        // Simplified: use distance as proxy for latency
        let distance = self.calculate_distance(loc1, loc2);
        // Assume 1km = 0.01ms latency (rough approximation)
        (distance * 0.01) as u64
    }

    /// Calculate distance between two geographic locations (Haversine formula)
    fn calculate_distance(&self, loc1: &GeoLocation, loc2: &GeoLocation) -> f64 {
        let r = 6371.0; // Earth radius in km

        let lat1 = loc1.latitude.to_radians();
        let lat2 = loc2.latitude.to_radians();
        let delta_lat = (loc2.latitude - loc1.latitude).to_radians();
        let delta_lon = (loc2.longitude - loc1.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        r * c
    }

    /// Perform failover for a region
    pub async fn failover(&self, service_name: &str, failed_region: &str) -> Result<String> {
        let mut services = self.services.write();

        let service = services
            .get_mut(service_name)
            .ok_or_else(|| DbError::Replication(format!("Service {} not found", service_name)))?;

        service.state = ServiceState::FailingOver;

        // Find standby region
        let mut target_region = None;

        for priority_region in &service.failover_policy.priority_order {
            for region in &service.regions {
                if &region.region_id == priority_region
                    && region.role == RegionRole::Standby
                    && region.health == HealthStatus::Healthy
                {
                    target_region = Some(region.clone());
                    break;
                }
            }
            if target_region.is_some() {
                break;
            }
        }

        if let Some(target) = target_region {
            // Promote standby to primary
            for region in &mut service.regions {
                if region.region_id == failed_region {
                    region.role = RegionRole::Standby;
                    region.health = HealthStatus::Unhealthy;
                } else if region.region_id == target.region_id {
                    region.role = RegionRole::Primary;
                }
            }

            service.state = ServiceState::Active;

            // Update statistics
            let mut stats = self.stats.write();
            stats.failovers += 1;

            Ok(target.region_id.clone())
        } else {
            service.state = ServiceState::Degraded;
            Err(DbError::Replication(
                "No healthy standby region available".to_string(),
            ))
        }
    }

    /// Update region health
    pub fn update_region_health(
        &self,
        service_name: &str,
        region_id: &str,
        health: HealthStatus,
    ) -> Result<()> {
        let mut services = self.services.write();

        let service = services
            .get_mut(service_name)
            .ok_or_else(|| DbError::Replication(format!("Service {} not found", service_name)))?;

        for region in &mut service.regions {
            if region.region_id == region_id {
                region.health = health;
                return Ok(());
            }
        }

        Err(DbError::Replication(format!(
            "Region {} not found",
            region_id
        )))
    }

    /// Update instance health
    pub fn update_instance_health(
        &self,
        service_name: &str,
        instance_id: &str,
        health: HealthStatus,
    ) -> Result<()> {
        let mut services = self.services.write();

        let service = services
            .get_mut(service_name)
            .ok_or_else(|| DbError::Replication(format!("Service {} not found", service_name)))?;

        for region in &mut service.regions {
            for db in &mut region.databases {
                if db.id == instance_id {
                    db.health = health;
                    db.last_health_check = Self::current_timestamp();
                    return Ok(());
                }
            }
        }

        Err(DbError::Replication(format!(
            "Instance {} not found",
            instance_id
        )))
    }

    /// Get statistics
    pub fn get_stats(&self) -> GdsStats {
        self.stats.read().clone()
    }

    /// Get service
    pub fn get_service(&self, name: &str) -> Option<GlobalService> {
        self.services.read().get(name).cloned()
    }

    /// Current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl Default for GlobalDataServices {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_service() {
        let gds = GlobalDataServices::new();

        let service = GlobalService {
            name: "test-service".to_string(),
            regions: vec![],
            load_balancing: LoadBalancingStrategy::RoundRobin,
            failover_policy: FailoverPolicy {
                auto_failover: true,
                timeout_ms: 5000,
                max_retries: 3,
                priority_order: vec![],
            },
            state: ServiceState::Active,
            created_at: 0,
        };

        gds.register_service(service).unwrap();

        let retrieved = gds.get_service("test-service");
        assert!(retrieved.is_some());
    }
}
