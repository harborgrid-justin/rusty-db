//! # RustyDB CLI
//!
//! Interactive SQL client for RustyDB.
//! Connects to a RustyDB server and allows executing SQL queries.

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, stdin, AsyncBufReadExt, BufReader};
use rusty_db::network::protocol::{Request, Response};
use rusty_db::Result;
use rusty_db::error::DbError;
use rusty_db::execution::QueryResult;
use rusty_db::VERSION;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║          RustyDB CLI - Interactive SQL Client           ║");
    println!("║                    Version {}                        ║", VERSION);
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    
    let addr = "127.0.0.1:5432";
    println!("Connecting to RustyDB server at {}...", addr);
    
    let mut stream = TcpStream::connect(addr).await
        .map_err(|e| DbError::Network(format!("Failed to connect: {}", e)))?;
    
    println!("Connected successfully!");
    println!("Type SQL commands or 'exit' to quit.");
    println!();
    
    let mut reader = BufReader::new(stdin());
    let mut input = String::new();
    
    loop {
        print!("rustydb> ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        input.clear();
        reader.read_line(&mut input).await
            .map_err(|e| DbError::Io(e))?;
        
        let cmd = input.trim();
        
        if cmd.is_empty() {
            continue;
        }
        
        if cmd.eq_ignore_ascii_case("exit") || cmd.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }
        
        // Send query
        let request = Request::Query { sql: cmd.to_string() };
        let request_bytes = bincode::serialize(&request)
            .map_err(|e| DbError::Serialization(e.to_string()))?;
        
        stream.write_all(&request_bytes).await
            .map_err(|e| DbError::Network(e.to_string()))?;
        
        // Read response
        let mut buffer = vec![0u8; 8192];
        let n = stream.read(&mut buffer).await
            .map_err(|e| DbError::Network(e.to_string()))?;
        
        if n == 0 {
            println!("Connection closed by server");
            break;
        }
        
        let response: Response = bincode::deserialize(&buffer[..n])
            .map_err(|e| DbError::Serialization(e.to_string()))?;
        
        match response {
            Response::QueryResult(result) => {
                print_result(&result);
            }
            Response::Ok => {
                println!("OK");
            }
            Response::Error(msg) => {
                println!("ERROR: {}", msg);
            }
            Response::TransactionId(id) => {
                println!("Transaction started: {}", id);
            }
            Response::Pong => {
                println!("PONG");
            }
        }
        
        println!();
    }
    
    Ok(())
}

fn print_result(result: &QueryResult) {
    if !result.columns.is_empty() {
        // Print column headers
        for col in &result.columns {
            print!("{:20}", col);
        }
        println!();
        
        // Print separator
        for _ in &result.columns {
            print!("{}", "-".repeat(20));
        }
        println!();
        
        // Print rows
        for row in &result.rows {
            for value in row {
                print!("{:20}", value);
            }
            println!();
        }
    }
    
    println!("{} row(s) affected", result.rows_affected);
}


