# Port Management System

Enterprise-grade port management for RustyDB distributed database.

## Overview

The port management system provides comprehensive port allocation, NAT traversal, firewall compatibility, and health monitoring for distributed database deployments.

## Architecture

```
ports/
├── mod.rs          - Main port manager and public API
├── allocator.rs    - Dynamic port allocation with multiple strategies
├── listener.rs     - Multi-protocol listener management (TCP/UDP)
├── nat.rs          - NAT traversal (STUN, UPnP, NAT-PMP)
├── firewall.rs     - Firewall-friendly port selection and tunneling
├── resolver.rs     - Address resolution with caching
├── mapping.rs      - Service-to-port mapping registry
└── health.rs       - Port health checking and monitoring
```

## Features

### 1. Dynamic Port Allocation (`allocator.rs`)

- **Sequential Allocation**: Allocate ports in order
- **Random Allocation**: Random port selection
- **Hash-Based Allocation**: Consistent hashing for predictable node assignment
- Port reservation and release
- Utilization monitoring

**Example:**
```rust
use rusty_db::network::ports::{PortAllocator, AllocationStrategy};

let mut allocator = PortAllocator::new(6000, 7000, AllocationStrategy::Random);
let port = allocator.allocate().unwrap();
println!("Allocated port: {}", port);
```

### 2. Multi-Port Listener (`listener.rs`)

- **IPv4/IPv6 Dual-Stack**: Support both IP versions simultaneously
- **TCP and UDP**: Multiple protocol support
- **Port Reuse**: SO_REUSEPORT and SO_REUSEADDR configuration
- **Unix Domain Sockets**: Local communication support

**Example:**
```rust
use rusty_db::network::ports::{ListenerManager, ListenerConfig};

let config = ListenerConfig::default();
let mut manager = ListenerManager::new(config);

let addrs = vec!["0.0.0.0:5432".parse().unwrap()];
manager.start_listeners(&addrs).await?;
```

### 3. NAT Traversal (`nat.rs`)

- **STUN Client**: Discover external IP address
- **UPnP Support**: Automatic port forwarding
- **NAT-PMP**: Lightweight port mapping protocol
- **Connectivity Testing**: Test reachability of remote endpoints

**Example:**
```rust
use rusty_db::network::ports::NatTraversal;

let mut nat = NatTraversal::new();
let external_ip = nat.get_external_ip().await?;
println!("External IP: {}", external_ip);

nat.setup_port_mapping(5432).await?;
```

### 4. Firewall Management (`firewall.rs`)

- **Port Probing**: Test if ports are accessible
- **Parallel Probing**: Check multiple ports simultaneously
- **Fallback Selection**: Automatically choose accessible ports
- **WebSocket Tunneling**: Tunnel database traffic over WebSocket
- **Well-Known Ports**: Use firewall-friendly ports (443, 80, etc.)

**Example:**
```rust
use rusty_db::network::ports::{FirewallManager, FirewallConfig};

let config = FirewallConfig::default();
let manager = FirewallManager::new(config);

let addrs = vec!["db.example.com:5432".parse().unwrap()];
let accessible = manager.find_accessible_port(&addrs).await?;
```

### 5. Address Resolution (`resolver.rs`)

- **DNS Resolution**: Convert hostnames to IP addresses
- **TTL-Based Caching**: Cache resolutions with configurable TTL
- **Load Balancing**: Round-robin, weighted random, priority-based selection
- **SRV Records**: Service discovery via DNS SRV (planned)

**Example:**
```rust
use rusty_db::network::ports::{AddressResolver, ResolverConfig};

let config = ResolverConfig::default();
let mut resolver = AddressResolver::new(config);

let endpoints = resolver.resolve("db.example.com:5432").await?;
for endpoint in endpoints {
    println!("Resolved: {}", endpoint.addr);
}
```

### 6. Port Mapping Service (`mapping.rs`)

- **Service Registry**: Map services to ports
- **Well-Known Ports**: Standard database port definitions
- **Metadata Support**: Attach arbitrary metadata to mappings
- **Conflict Detection**: Prevent duplicate port assignments
- **JSON Export**: Export mappings for documentation

**Example:**
```rust
use rusty_db::network::ports::PortMappingService;

let service = PortMappingService::new();
service.initialize_defaults().await?;

let port = service.get_service_port("Database").await;
println!("Database port: {}", port.unwrap());
```

### 7. Health Monitoring (`health.rs`)

- **Availability Checks**: Test if ports can be bound
- **Conflict Detection**: Identify port binding conflicts
- **Exhaustion Monitoring**: Track port pool utilization
- **Periodic Checks**: Automated health monitoring
- **History Tracking**: Maintain health check history

**Example:**
```rust
use rusty_db::network::ports::{PortHealthChecker, HealthCheckConfig};

let config = HealthCheckConfig::default();
let checker = PortHealthChecker::new(config);

let result = checker.check_port(5432).await?;
println!("Port 5432 status: {}", result.status);
```

## Complete Usage Example

```rust
use rusty_db::network::ports::{PortManager, PortConfig, ServiceType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create port manager with configuration
    let config = PortConfig {
        base_port: 5432,
        port_range_start: 6000,
        port_range_end: 7000,
        enable_ipv6: true,
        enable_nat_traversal: true,
        enable_firewall_friendly: true,
        bind_addresses: vec!["0.0.0.0:5432".to_string()],
        ..Default::default()
    };

    let mut manager = PortManager::new(config);

    // Allocate ports for different services
    let db_port = manager.allocate_port(ServiceType::Database).await?;
    let cluster_port = manager.allocate_port(ServiceType::Cluster).await?;
    let api_port = manager.allocate_port(ServiceType::Api).await?;

    println!("Database port: {}", db_port);
    println!("Cluster port: {}", cluster_port);
    println!("API port: {}", api_port);

    // Start listeners
    manager.start_listener(db_port).await?;
    manager.start_listener(cluster_port).await?;

    // Get external IP (if NAT traversal enabled)
    if let Ok(external_ip) = manager.get_external_ip().await {
        println!("External IP: {}", external_ip);
    }

    // Check health of all ports
    let health_status = manager.check_health().await?;
    for (port, status) in health_status {
        println!("Port {} health: {:?}", port, status);
    }

    // Resolve remote database address
    let endpoints = manager.resolve_address("db.example.com:5432").await?;
    for endpoint in endpoints {
        println!("Resolved endpoint: {}", endpoint.addr);
    }

    // Cleanup
    manager.shutdown().await?;

    Ok(())
}
```

## Configuration

### PortConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_port` | `u16` | `5432` | Base port for database services |
| `port_range_start` | `u16` | `6000` | Start of dynamic allocation range |
| `port_range_end` | `u16` | `7000` | End of dynamic allocation range |
| `enable_ipv6` | `bool` | `true` | Enable IPv6 dual-stack |
| `enable_unix_sockets` | `bool` | `true` | Enable Unix domain sockets |
| `enable_nat_traversal` | `bool` | `false` | Enable NAT traversal |
| `enable_firewall_friendly` | `bool` | `true` | Enable firewall features |
| `bind_addresses` | `Vec<String>` | `["0.0.0.0:5432"]` | Addresses to bind |
| `reuse_port` | `bool` | `true` | Enable SO_REUSEPORT |
| `reuse_addr` | `bool` | `true` | Enable SO_REUSEADDR |
| `health_check_interval` | `u64` | `60` | Health check interval (seconds) |

## Service Types

- `Database`: Main database service (default: 5432)
- `Cluster`: Cluster communication (default: 5433)
- `Replication`: Replication service (default: 5434)
- `Admin`: Admin/monitoring (default: 8080)
- `Api`: API gateway (default: 8081)
- `Custom`: User-defined services

## Well-Known Ports

| Port | Service |
|------|---------|
| 5432 | PostgreSQL / RustyDB Database |
| 5433 | RustyDB Cluster |
| 5434 | RustyDB Replication |
| 8080 | RustyDB Admin |
| 8081 | RustyDB API |
| 443  | HTTPS (firewall-friendly fallback) |
| 80   | HTTP (firewall-friendly fallback) |

## Error Handling

All operations return `Result<T, DbError>` for consistent error handling:

```rust
use rusty_db::error::{DbError, Result};

match manager.allocate_port(ServiceType::Database).await {
    Ok(port) => println!("Allocated: {}", port),
    Err(DbError::ResourceExhausted(msg)) => eprintln!("No ports available: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

Run the test suite:

```bash
# Run all port management tests
cargo test --lib ports::

# Run specific module tests
cargo test --lib ports::allocator::
cargo test --lib ports::health::
cargo test --lib ports::nat::
```

## Performance Considerations

1. **Port Allocation**: O(n) worst case for sequential/random, O(1) average for hash-based
2. **Caching**: DNS resolution cached with configurable TTL
3. **Parallel Probing**: Firewall port checks run in parallel for better performance
4. **Lock-Free**: Uses async RwLock for minimal contention

## Security

- **Port Range Restriction**: Configurable port ranges prevent unauthorized access
- **Permission Checks**: Detects and reports permission denied errors
- **Validation**: All inputs validated before use
- **No Privileged Ports**: Default range avoids ports < 1024

## Platform Support

- **Linux**: Full support including SO_REUSEPORT, io_uring (if enabled)
- **macOS**: Full support including SO_REUSEPORT
- **Windows**: TCP/UDP support, limited SO_REUSEPORT
- **Unix**: Unix domain socket support on Unix-like platforms

## Future Enhancements

- [ ] Full UPnP IGD implementation
- [ ] DNS SRV record resolution
- [ ] ICE (Interactive Connectivity Establishment)
- [ ] TURN relay support
- [ ] Automatic port migration on conflicts
- [ ] Port usage analytics

## Dependencies

- `tokio`: Async runtime
- `socket2`: Advanced socket configuration
- `rand`: Random port allocation
- `serde`: Configuration serialization

## License

MIT OR Apache-2.0

## Contributing

See main project CONTRIBUTING.md for guidelines.
