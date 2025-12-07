use rusty_db::{Config, Result};
use rusty_db::network::Server;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let config = Config::default();
    
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║          RustyDB - Enterprise Database System           ║");
    println!("║         Rust-based Oracle DB Competitor v0.1.0          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Starting RustyDB server...");
    println!("Data directory: {}", config.data_dir);
    println!("Port: {}", config.port);
    println!("Page size: {} bytes", config.page_size);
    println!("Buffer pool size: {} pages", config.buffer_pool_size);
    println!();
    
    let server = Server::new();
    let addr = format!("127.0.0.1:{}", config.port);
    
    println!("Server listening on {}", addr);
    println!("Ready to accept connections!");
    println!();
    
    server.run(&addr).await?;
    
    Ok(())
}
