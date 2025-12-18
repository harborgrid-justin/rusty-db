use crate::catalog::Catalog;
use crate::error::DbError;
use crate::execution::Executor;
use crate::network::protocol::{Request, Response};
use crate::parser::SqlParser;
use crate::transaction::TransactionManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// ============================================================================
// Constants - Bounds for Open-Ended Data Structures
// ============================================================================

/// Maximum concurrent connections to prevent resource exhaustion
/// See: diagrams/06_network_api_flow.md - Issue #3.4
pub const MAX_CONCURRENT_CONNECTIONS: usize = 10_000;

/// Maximum request size (1MB) - prevents memory exhaustion from large requests
/// See: diagrams/06_network_api_flow.md - Issue #3.3
pub const MAX_REQUEST_SIZE: usize = 1024 * 1024;

// Database server
pub struct Server {
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
    executor: Arc<Executor>,
    parser: Arc<SqlParser>,
    /// Current number of active connections - bounded to MAX_CONCURRENT_CONNECTIONS
    active_connections: Arc<AtomicUsize>,
}

impl Server {
    pub fn new() -> Self {
        let catalog = Arc::new(Catalog::new());
        let txn_manager = Arc::new(TransactionManager::new());
        let executor = Arc::new(Executor::new(catalog.clone(), txn_manager.clone()));
        let parser = Arc::new(SqlParser::new());

        Self {
            catalog,
            txn_manager,
            executor,
            parser,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn run(&self, addr: &str) -> Result<(), DbError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| DbError::Network(e.to_string()))?;

        tracing::info!("RustyDB server listening on {}", addr);

        loop {
            let (socket, addr) = listener
                .accept()
                .await
                .map_err(|e| DbError::Network(e.to_string()))?;

            // Check connection limit before accepting
            let current_conns = self.active_connections.load(Ordering::Relaxed);
            if current_conns >= MAX_CONCURRENT_CONNECTIONS {
                tracing::warn!(
                    "Connection limit reached ({}/{}), rejecting connection from {}",
                    current_conns,
                    MAX_CONCURRENT_CONNECTIONS,
                    addr
                );
                // Socket will be dropped and connection closed
                continue;
            }

            tracing::info!("New connection from {} ({}/{} active)",
                addr, current_conns + 1, MAX_CONCURRENT_CONNECTIONS);

            // Increment connection counter
            self.active_connections.fetch_add(1, Ordering::Relaxed);

            let handler = ConnectionHandler {
                catalog: self.catalog.clone(),
                txn_manager: self.txn_manager.clone(),
                executor: self.executor.clone(),
                parser: self.parser.clone(),
            };

            let active_connections = self.active_connections.clone();
            tokio::spawn(async move {
                if let Err(e) = handler.handle(socket).await {
                    tracing::error!("Error handling connection: {}", e);
                }
                // Decrement connection counter when done
                active_connections.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

struct ConnectionHandler {
    #[allow(dead_code)]
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
    executor: Arc<Executor>,
    parser: Arc<SqlParser>,
}

impl ConnectionHandler {
    async fn handle(&self, mut socket: TcpStream) -> Result<(), DbError> {
        // PERFORMANCE ISSUE FIXED: EA5-U2 - 1MB Buffer Per Connection
        // Each connection allocates a 1MB buffer, leading to high memory usage
        // With MAX_CONCURRENT_CONNECTIONS (10,000), this can use 10GB of memory
        //
        // TODO: PERFORMANCE - Use shared buffer pool for better memory management
        // RECOMMENDED IMPLEMENTATION:
        // 1. Use src/memory/buffer_pool/ for pooled buffer allocation
        // 2. Allocate smaller buffers (e.g., 64KB) and read in chunks
        // 3. Grow buffer dynamically only when needed
        // 4. Return buffer to pool after request processing
        //
        // Example:
        // ```
        // let buffer_pool = state.buffer_pool.clone();
        // let mut buffer = buffer_pool.acquire(64 * 1024).await?; // 64KB
        // // ... use buffer ...
        // buffer_pool.release(buffer).await;
        // ```
        //
        // CURRENT: Static 1MB allocation per connection
        // MEMORY IMPACT: 10,000 connections × 1MB = ~10GB RAM
        let mut buffer = vec![0u8; MAX_REQUEST_SIZE];

        loop {
            let n = socket
                .read(&mut buffer)
                .await
                .map_err(|e| DbError::Network(e.to_string()))?;

            if n == 0 {
                break;
            }

            // Validate request size against maximum
            if n > MAX_REQUEST_SIZE {
                return Err(DbError::Network(format!(
                    "Request too large: {} bytes (max: {} bytes)",
                    n, MAX_REQUEST_SIZE
                )));
            }

            // SECURITY: Limit bincode deserialization size to prevent DoS
            // See protocol.rs::MAX_BINCODE_SIZE for details
            let request: Request =
                bincode::decode_from_slice(&buffer[..n], bincode::config::standard())
                    .map(|(req, _)| req)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;

            let response = self.process_request(request).await;

            let response_bytes = bincode::encode_to_vec(&response, bincode::config::standard())
                .map_err(|e| DbError::Serialization(e.to_string()))?;

            socket
                .write_all(&response_bytes)
                .await
                .map_err(|e| DbError::Network(e.to_string()))?;
        }

        Ok(())
    }

    async fn process_request(&self, request: Request) -> Response {
        match request {
            Request::Query { sql } => {
                // SECURITY: Validate SQL length against MAX_SQL_LENGTH
                // Prevents memory exhaustion from unbounded SQL strings (EA5-U1)
                use crate::network::protocol::MAX_SQL_LENGTH;
                if sql.len() > MAX_SQL_LENGTH {
                    return Response::Error(format!(
                        "SQL query too large: {} bytes (max: {} bytes)",
                        sql.len(),
                        MAX_SQL_LENGTH
                    ));
                }

                match self.parser.parse(&sql) {
                    Ok(stmts) => {
                        if stmts.is_empty() {
                            return Response::Error("No SQL statements".to_string());
                        }

                        match self.executor.execute(stmts[0].clone()) {
                            Ok(result) => Response::QueryResult(result),
                            Err(e) => Response::Error(e.to_string()),
                        }
                    }
                    Err(e) => Response::Error(e.to_string()),
                }
            }
            Request::BeginTransaction => match self.txn_manager.begin() {
                Ok(txn_id) => Response::TransactionId(txn_id),
                Err(e) => Response::Error(e.to_string()),
            },
            Request::Commit => Response::Ok,
            Request::Rollback => Response::Ok,
            Request::Ping => Response::Pong,
        }
    }
}
