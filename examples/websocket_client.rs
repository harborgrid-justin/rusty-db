// # WebSocket Client Example
//
// This example demonstrates how to connect to RustyDB using WebSocket
// for real-time query execution and streaming results.

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

// WebSocket message structure
#[derive(Debug, Serialize, Deserialize)]
struct WebSocketMessage {
    #[serde(rename = "type")]
    message_type: String,
    payload: serde_json::Value,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

// Query response structure
#[derive(Debug, Deserialize)]
struct QueryResponse {
    status: String,
    #[serde(default)]
    rows: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    rows_affected: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RustyDB WebSocket Client Example ===\n");

    // Example 1: Basic connection and authentication
    println!("Example 1: Basic Connection");
    println!("----------------------------");
    basic_connection_example().await?;

    // Example 2: Execute queries with streaming results
    println!("\nExample 2: Query Execution");
    println!("----------------------------");
    query_execution_example().await?;

    // Example 3: Heartbeat (ping/pong)
    println!("\nExample 3: Heartbeat");
    println!("----------------------------");
    heartbeat_example().await?;

    // Example 4: Error handling
    println!("\nExample 4: Error Handling");
    println!("----------------------------");
    error_handling_example().await?;

    println!("\n=== All examples completed ===");

    Ok(())
}

// Example 1: Basic connection and authentication
async fn basic_connection_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to RustyDB WebSocket endpoint
    let url = "ws://localhost:8080/api/v1/stream";
    println!("Connecting to: {}", url);

    let (ws_stream, _) = connect_async(url).await?;
    println!("✓ Connected successfully");

    let (mut write, mut read) = ws_stream.split();

    // Send authentication message
    let auth_message = WebSocketMessage {
        message_type: "auth".to_string(),
        payload: json!({
            "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: Some(uuid::Uuid::new_v4().to_string()),
    };

    let auth_json = serde_json::to_string(&auth_message)?;
    write.send(Message::Text(auth_json)).await?;
    println!("✓ Sent authentication message");

    // Wait for authentication response
    if let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let response: WebSocketMessage = serde_json::from_str(&text)?;
                match response.message_type.as_str() {
                    "auth_success" => println!("✓ Authentication successful"),
                    "auth_error" => {
                        println!("✗ Authentication failed: {:?}", response.payload);
                        return Err("Authentication failed".into());
                    }
                    _ => println!("  Received: {}", response.message_type),
                }
            }
            _ => println!("  Received non-text message"),
        }
    }

    // Close connection
    write.close().await?;
    println!("✓ Connection closed\n");

    Ok(())
}

// Example 2: Execute queries with streaming results
async fn query_execution_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8080/api/v1/stream";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✓ Connected to RustyDB");

    let (mut write, mut read) = ws_stream.split();

    // Execute a query
    let query_message = WebSocketMessage {
        message_type: "query".to_string(),
        payload: json!({
            "sql": "SELECT name, age FROM users WHERE age > 18 ORDER BY name LIMIT 5",
            "params": [],
            "streaming": true
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: Some("query-001".to_string()),
    };

    let query_json = serde_json::to_string(&query_message)?;
    write.send(Message::Text(query_json)).await?;
    println!("✓ Sent query: SELECT name, age FROM users...");

    // Receive query results
    if let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let response: WebSocketMessage = serde_json::from_str(&text)?;

                if response.message_type == "data" {
                    let query_response: QueryResponse = serde_json::from_value(response.payload)?;

                    println!("✓ Query executed successfully");
                    println!("  Status: {}", query_response.status);
                    println!("  Columns: {:?}", query_response.columns);
                    println!("  Rows returned: {}", query_response.rows.len());

                    // Print results
                    if !query_response.rows.is_empty() {
                        println!("\n  Results:");
                        for (i, row) in query_response.rows.iter().enumerate() {
                            println!("    Row {}: {:?}", i + 1, row);
                        }
                    }
                } else if response.message_type == "error" {
                    let query_response: QueryResponse = serde_json::from_value(response.payload)?;
                    println!("✗ Query error: {}", query_response.message.unwrap_or_default());
                }
            }
            _ => println!("  Received non-text message"),
        }
    }

    // Close connection
    write.close().await?;
    println!("\n✓ Connection closed\n");

    Ok(())
}

// Example 3: Heartbeat (ping/pong)
async fn heartbeat_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8080/api/v1/stream";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✓ Connected to RustyDB");

    let (mut write, mut read) = ws_stream.split();

    // Send ping message
    let ping_message = WebSocketMessage {
        message_type: "ping".to_string(),
        payload: json!({}),
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: None,
    };

    let ping_json = serde_json::to_string(&ping_message)?;
    write.send(Message::Text(ping_json)).await?;
    println!("✓ Sent ping");

    // Wait for pong response with timeout
    match tokio::time::timeout(Duration::from_secs(5), read.next()).await {
        Ok(Some(msg)) => {
            match msg? {
                Message::Text(text) => {
                    let response: WebSocketMessage = serde_json::from_str(&text)?;
                    if response.message_type == "pong" {
                        println!("✓ Received pong");
                        println!("  Server timestamp: {}", response.timestamp);
                    } else {
                        println!("  Received: {}", response.message_type);
                    }
                }
                Message::Pong(_) => println!("✓ Received WebSocket pong"),
                _ => println!("  Received other message type"),
            }
        }
        Ok(None) => println!("✗ Connection closed"),
        Err(_) => println!("✗ Timeout waiting for pong"),
    }

    // Close connection
    write.close().await?;
    println!("\n✓ Connection closed\n");

    Ok(())
}

// Example 4: Error handling
async fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8080/api/v1/stream";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✓ Connected to RustyDB");

    let (mut write, mut read) = ws_stream.split();

    // Execute an invalid query
    let invalid_query = WebSocketMessage {
        message_type: "query".to_string(),
        payload: json!({
            "sql": "SELECT * FROM nonexistent_table",
            "params": []
        }),
        timestamp: chrono::Utc::now().to_rfc3339(),
        id: Some("query-error-001".to_string()),
    };

    let query_json = serde_json::to_string(&invalid_query)?;
    write.send(Message::Text(query_json)).await?;
    println!("✓ Sent invalid query (expecting error)");

    // Receive error response
    if let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let response: WebSocketMessage = serde_json::from_str(&text)?;

                if response.message_type == "error" {
                    println!("✓ Received expected error response");
                    println!("  Error details: {:?}", response.payload);

                    // Parse error details
                    if let Some(message) = response.payload.get("message") {
                        println!("  Error message: {}", message);
                    }
                    if let Some(code) = response.payload.get("code") {
                        println!("  Error code: {}", code);
                    }
                } else {
                    println!("  Unexpected response type: {}", response.message_type);
                }
            }
            _ => println!("  Received non-text message"),
        }
    }

    // Close connection
    write.close().await?;
    println!("\n✓ Connection closed\n");

    Ok(())
}

// Advanced Example: Reconnection logic with exponential backoff
#[allow(dead_code)]
async fn advanced_reconnection_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8080/api/v1/stream";
    let max_retries = 5;
    let mut retry_count = 0;
    let mut retry_delay = Duration::from_secs(1);

    loop {
        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                println!("✓ Connected successfully");
                let (mut write, mut read) = ws_stream.split();

                // Handle messages
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            println!("Received: {}", text);
                        }
                        Ok(Message::Close(_)) => {
                            println!("Server closed connection");
                            break;
                        }
                        Err(e) => {
                            println!("Error receiving message: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }

                // Connection closed, attempt reconnection
                retry_count = 0;
                retry_delay = Duration::from_secs(1);
            }
            Err(e) => {
                retry_count += 1;
                if retry_count > max_retries {
                    println!("✗ Max retries reached, giving up");
                    return Err(e.into());
                }

                println!(
                    "✗ Connection failed (attempt {}/{}): {}",
                    retry_count, max_retries, e
                );
                println!("  Retrying in {:?}...", retry_delay);

                tokio::time::sleep(retry_delay).await;

                // Exponential backoff
                retry_delay = std::cmp::min(
                    retry_delay * 2,
                    Duration::from_secs(60)
                );
            }
        }
    }
}

// Advanced Example: Multiple concurrent subscriptions
#[allow(dead_code)]
async fn multiple_subscriptions_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8080/api/v1/stream";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✓ Connected to RustyDB");

    let (mut write, mut read) = ws_stream.split();

    // Send multiple queries
    let queries = vec![
        ("query-1", "SELECT * FROM users LIMIT 5"),
        ("query-2", "SELECT * FROM products LIMIT 5"),
        ("query-3", "SELECT * FROM orders LIMIT 5"),
    ];

    for (id, sql) in &queries {
        let query_message = WebSocketMessage {
            message_type: "query".to_string(),
            payload: json!({
                "sql": sql,
                "streaming": true
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            id: Some(id.to_string()),
        };

        let query_json = serde_json::to_string(&query_message)?;
        write.send(Message::Text(query_json)).await?;
        println!("✓ Sent query: {}", id);
    }

    // Receive all responses
    let mut responses_received = 0;
    while responses_received < queries.len() {
        if let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    let response: WebSocketMessage = serde_json::from_str(&text)?;
                    println!("✓ Received response for: {:?}", response.id);
                    responses_received += 1;
                }
                _ => {}
            }
        }
    }

    write.close().await?;
    println!("✓ All queries completed\n");

    Ok(())
}

// Helper: Format duration for display
#[allow(dead_code)]
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{}.{:03}s", secs, millis)
}
