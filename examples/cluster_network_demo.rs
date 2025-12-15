// Cluster Network Demo
// This example demonstrates the usage of RustyDB's cluster networking capabilities

use rusty_db::network::cluster_network::NodeCapacity;
use rusty_db::network::{
    ClusterMessage, ClusterNetworkManager, MembershipEvent, MessagePriority, NodeCapacity,
    NodeInfo, NodeState, RoutingStrategy,
};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== RustyDB Cluster Network Demo ===\n");

    // 1. Create cluster manager
    println!("1. Creating cluster network manager...");
    let local_addr = "127.0.0.1:7000".parse()?;
    let manager = ClusterNetworkManager::new(local_addr).await?;

    // 2. Start cluster services
    println!("2. Starting cluster services...");
    manager.start().await?;

    // 3. Subscribe to cluster events
    println!("3. Subscribing to cluster events...");
    let mut events = manager.subscribe_to_events();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                MembershipEvent::NodeJoined(node_id) => {
                    println!("   [EVENT] Node {} joined at", node_id);
                }
                MembershipEvent::NodeFailed(node_id) => {
                    println!("   [EVENT] Node {} failed", node_id);
                }
                MembershipEvent::NodeLeft(node_id) => {
                    println!("   [EVENT] Node {} left gracefully", node_id);
                }
                MembershipEvent::TopologyChanged => {
                    println!("   [EVENT] Cluster topology changed");
                }
                _ => {}
            }
        }
    });

    // 4. Join existing cluster (if seed nodes provided)
    // Uncomment to join an existing cluster:
    // println!("4. Joining existing cluster...");
    // let seed_nodes = vec![
    //     "10.0.0.1:7000".parse()?,
    //     "10.0.0.2:7000".parse()?,
    // ];
    // manager.join_cluster(seed_nodes).await?;

    // For demo, we'll simulate adding nodes
    println!("4. Simulating cluster nodes...");
    let node1 = NodeInfo {
        id: rusty_db::network::NodeId::new(0),
        addr: "127.0.0.1:7001".parse()?,
        state: NodeState::Alive,
        incarnation: 1,
        metadata: vec![("role".to_string(), "worker".to_string())]
            .into_iter()
            .collect(),
        last_seen: std::time::Instant::now(),
        datacenter: "dc1".to_string(),
        rack: "rack1".to_string(),
        capacity: NodeCapacity {
            cpu: 0.0,
            memory: 0,
            cpu_cores: 16,
            memory_gb: 64,
            max_connections: 2000,
            current_connections: 150,
            query_latency_ms: 5.2,
            disk_io_utilization: 0.3,
            connections: 0,
        },
        address: (),
    };

    let node2 = NodeInfo {
        id: rusty_db::network::NodeId::new(0),
        addr: "127.0.0.1:7002".parse()?,
        state: NodeState::Alive,
        incarnation: 1,
        metadata: vec![("role".to_string(), "worker".to_string())]
            .into_iter()
            .collect(),
        last_seen: std::time::Instant::now(),
        datacenter: "dc1".to_string(),
        rack: "rack2".to_string(),
        capacity: NodeCapacity {
            cpu: 0.0,
            memory: 0,
            connections: 0,
            cpu_cores: 16,
            memory_gb: 64,
            max_connections: 2000,
            current_connections: 80,
            query_latency_ms: 3.8,
            disk_io_utilization: 0.2,
        },
        address: (),
    };

    manager.add_node(node1.clone()).await?;
    manager.add_node(node2.clone()).await?;

    sleep(Duration::from_millis(100)).await;

    // 5. Display cluster members
    println!("\n5. Current cluster members:");
    for member in manager.get_cluster_members() {
        println!(
            "   - Node {} at {} ({})",
            member.id,
            member.addr,
            match member.state {
                NodeState::Alive => "ALIVE",
                NodeState::Suspect => "SUSPECT",
                NodeState::Dead => "DEAD",
                NodeState::Left => "LEFT",
                NodeState::Joining => "JOINING",
            }
        );
        println!(
            "     Datacenter: {}, Rack: {}",
            member.datacenter, member.rack
        );
        println!(
            "     Capacity: {} cores, {}GB RAM, {}/{} connections",
            member.capacity.cpu_cores,
            member.capacity.memory_gb,
            member.capacity.current_connections,
            member.capacity.max_connections,
        );
        println!(
            "     Latency: {:.2}ms, IO Util: {:.1}%",
            member.capacity.query_latency_ms,
            member.capacity.disk_io_utilization * 100.0,
        );
    }

    // 6. Query routing demo
    println!("\n6. Query routing demonstration:");
    let queries = vec![
        ("SELECT * FROM users WHERE id = 1", MessagePriority::Normal),
        ("SELECT COUNT(*) FROM orders", MessagePriority::High),
        (
            "UPDATE inventory SET quantity = 100",
            MessagePriority::Critical,
        ),
        (
            "SELECT * FROM logs WHERE date > NOW() - INTERVAL 1 DAY",
            MessagePriority::Low,
        ),
    ];

    for (sql, priority) in queries {
        match manager.route_query(sql, priority).await {
            Ok(target_node) => {
                println!("   Query: '{}...'", &sql[..30.min(sql.len())]);
                println!("   Priority: {:?}, Routed to: {}", priority, target_node);
            }
            Err(e) => {
                println!("   Query routing failed: {}", e);
            }
        }
    }

    // 7. Load balancing strategies
    println!("\n7. Testing different load balancing strategies:");
    let strategies = vec![
        RoutingStrategy::RoundRobin,
        RoutingStrategy::LeastConnections,
        RoutingStrategy::Adaptive,
    ];

    for strategy in strategies {
        manager.set_routing_strategy(strategy).await;
        println!("   Strategy: {:?}", strategy);

        for _ in 0..3 {
            if let Ok(node) = manager
                .route_query("SELECT 1", MessagePriority::Normal)
                .await
            {
                print!("   → {} ", node);
            }
        }
        println!();
    }

    // 8. Health check
    println!("\n8. Cluster health check:");
    let health = manager.health_check().await;
    println!(
        "   Status: {}",
        if health.healthy {
            "✓ HEALTHY"
        } else {
            "✗ UNHEALTHY"
        }
    );
    println!("   Timestamp: {}", health.timestamp);
    println!("   Metrics:");
    for (key, value) in health.metrics {
        println!("     {}: {:.2}", key, value);
    }

    // 9. Quorum check
    println!("\n9. Quorum status:");
    println!(
        "   Has quorum: {}",
        if manager.has_quorum() {
            "✓ YES"
        } else {
            "✗ NO"
        }
    );
    println!("   Alive nodes: {}", manager.get_alive_members().len());

    // 10. Metrics collection
    println!("\n10. Comprehensive metrics:");
    let all_metrics = manager.get_all_metrics();
    for (category, metrics) in all_metrics {
        println!("   {}:", category.to_uppercase());
        for (name, value) in metrics {
            println!("     {}: {}", name, value);
        }
    }

    // 11. Message broadcasting demo
    println!("\n11. Broadcasting heartbeat to all nodes:");
    let heartbeat = ClusterMessage::HeartBeat {
        node: rusty_db::network::NodeId::new(0),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let results = manager.broadcast_message(heartbeat).await;
    println!("   Broadcast sent to {} nodes", results.len());
    let successful = results.iter().filter(|r| r.is_ok()).count();
    println!("   Successful: {}/{}", successful, results.len());

    // 12. Simulate rolling restart
    println!("\n12. Rolling restart simulation:");
    println!("   (Skipped in demo - would restart all nodes sequentially)");
    // Uncomment to actually perform rolling restart:
    // manager.rolling_restart().await?;

    // Keep running for a bit to observe events
    println!("\n=== Demo running for 10 seconds to observe events ===");
    println!("(Press Ctrl+C to exit earlier)\n");

    sleep(Duration::from_secs(10)).await;

    // Cleanup
    println!("\n=== Shutting down cluster manager ===");
    manager.shutdown().await;

    println!("Demo completed successfully!");

    Ok(())
}
