// P2P Communication Protocol Demo
//
// This example demonstrates the core P2P communication features:
// - TCP transport with connection pooling
// - Message encoding/decoding
// - Protocol handshake
// - Connection health monitoring

use rusty_db::error::Result;
use rusty_db::networking::protocol::{HandshakeRequest, Message, MessageCodec};
use rusty_db::networking::transport::{
    ConnectionPool, PoolConfig, TcpConfig, TcpTransport, TransportType,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== RustyDB P2P Communication Demo ===\n");

    // 1. Create TCP transport configuration
    println!("1. Creating TCP transport configuration...");
    let mut tcp_config = TcpConfig::default();
    tcp_config.bind_addr = "127.0.0.1:19000".parse().unwrap();
    tcp_config.nodelay = true;
    println!("   ✓ TCP configured on {}", tcp_config.bind_addr);

    // 2. Create transport
    println!("\n2. Initializing TCP transport...");
    let mut transport = TcpTransport::new(tcp_config);
    transport.bind().await?;
    println!("   ✓ TCP transport listening");

    // 3. Create connection pool
    println!("\n3. Creating connection pool...");
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        idle_timeout: std::time::Duration::from_secs(60),
        health_check_interval: std::time::Duration::from_secs(10),
        acquisition_timeout: std::time::Duration::from_secs(5),
    };
    let pool = Arc::new(ConnectionPool::new(pool_config));
    println!("   ✓ Connection pool created (max: 5 connections per peer)");

    // 4. Start health check background task
    println!("\n4. Starting health check task...");
    let _health_task = pool.clone().start_health_check_task();
    println!("   ✓ Health check task started (interval: 10s)");

    // 5. Add a mock connection to the pool
    println!("\n5. Adding connection to pool...");
    let node_id = "test-node-1".to_string();
    let conn = pool
        .add_connection(node_id.clone(), TransportType::Tcp)
        .await?;
    println!("   ✓ Connection added for peer: {}", node_id);
    println!(
        "   ✓ Total connections: {}",
        pool.connection_count(&node_id).await
    );

    // 6. Create message codec
    println!("\n6. Creating message codec...");
    let codec = MessageCodec::new();
    println!("   ✓ Message codec initialized");

    // 7. Create and encode a handshake request
    println!("\n7. Creating handshake request...");
    let handshake = HandshakeRequest::new(
        "rusty-db-node-1".to_string(),
        "production-cluster".to_string(),
    );
    println!("   ✓ Handshake request created");
    println!("      - Protocol version: {}", handshake.protocol_version);
    println!("      - Node ID: {}", handshake.node_id);
    println!("      - Cluster: {}", handshake.cluster_name);

    // 8. Encode the handshake message
    println!("\n8. Encoding handshake message...");
    let message = Message::HandshakeRequest(handshake);
    let encoded = codec.encode(1, &message)?;
    println!("   ✓ Message encoded ({} bytes)", encoded.len());

    // 9. Decode the message
    println!("\n9. Decoding message...");
    let (message_id, decoded) = codec.decode(encoded)?;
    println!("   ✓ Message decoded");
    println!("      - Message ID: {}", message_id);
    println!("      - Message type: {}", decoded.message_type());

    // 10. Create ping/pong messages
    println!("\n10. Creating and encoding Ping message...");
    let ping = Message::Ping {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    let ping_encoded = codec.encode(2, &ping)?;
    println!("   ✓ Ping message encoded ({} bytes)", ping_encoded.len());

    let (ping_id, ping_decoded) = codec.decode(ping_encoded)?;
    println!("   ✓ Ping message decoded (ID: {})", ping_id);

    // 11. Display connection metrics
    println!("\n11. Connection metrics:");
    println!("   - Uptime: {:?}", conn.uptime());
    println!("   - Bytes sent: {}", conn.bytes_sent());
    println!("   - Bytes received: {}", conn.bytes_received());
    println!("   - Messages sent: {}", conn.messages_sent());
    println!("   - Messages received: {}", conn.messages_received());
    println!("   - Is healthy: {}", conn.is_healthy().await);

    // 12. Get pool statistics
    println!("\n12. Pool statistics:");
    let stats = pool.get_statistics().await;
    for (peer, stat) in stats {
        println!("   Peer: {}", peer);
        println!("      - Total connections: {}", stat.total_connections);
        println!("      - Healthy connections: {}", stat.healthy_connections);
        println!("      - Total bytes sent: {}", stat.total_bytes_sent);
        println!(
            "      - Total bytes received: {}",
            stat.total_bytes_received
        );
    }

    println!("\n=== Demo Complete ===");
    println!("\nSuccessfully demonstrated:");
    println!("  ✓ TCP transport initialization");
    println!("  ✓ Connection pool management");
    println!("  ✓ Message encoding/decoding");
    println!("  ✓ Protocol handshake");
    println!("  ✓ Connection metrics tracking");
    println!("  ✓ Health monitoring");

    Ok(())
}
