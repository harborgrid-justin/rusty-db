use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt};
use std::sync::Arc;
use crate::error::DbError;
use crate::catalog::Catalog;
use crate::transaction::TransactionManager;
use crate::execution::Executor;
use crate::parser::SqlParser;
use crate::network::protocol::{Request, Response};

/// Database server
pub struct Server {
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
    executor: Arc<Executor>,
    parser: Arc<SqlParser>,
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
        }
    }
    
    pub async fn run(&self, addr: &str) -> Result<(), DbError> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| DbError::Network(e.to_string()))?;
        
        tracing::info!("RustyDB server listening on {}", addr);
        
        loop {
            let (socket, addr) = listener.accept().await
                .map_err(|e| DbError::Network(e.to_string()))?;
            
            tracing::info!("New connection from {}", addr);
            
            let handler = ConnectionHandler {
                catalog: self.catalog.clone(),
                txn_manager: self.txn_manager.clone(),
                executor: self.executor.clone(),
                parser: self.parser.clone(),
            };
            
            tokio::spawn(async move {
                if let Err(e) = handler.handle(socket).await {
                    tracing::error!("Error handling connection: {}", e);
                }
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
    catalog: Arc<Catalog>,
    txn_manager: Arc<TransactionManager>,
    executor: Arc<Executor>,
    parser: Arc<SqlParser>,
}

impl ConnectionHandler {
    async fn handle(&self, mut socket: TcpStream) -> Result<(), DbError> {
        const MAX_REQUEST_SIZE: usize = 1024 * 1024; // 1MB limit
        let mut buffer = vec![0u8; MAX_REQUEST_SIZE];
        
        loop {
            let n = socket.read(&mut buffer).await
                .map_err(|e| DbError::Network(e.to_string()))?;
            
            if n == 0 {
                break;
            }
            
            // Validate request size
            if n > MAX_REQUEST_SIZE {
                return Err(DbError::Network("Request too large".to_string()));
            }
            
            let request: Request = bincode::deserialize(&buffer[..n])
                .map_err(|e| DbError::Serialization(e.to_string()))?;
            
            let response = self.process_request(request).await;
            
            let response_bytes = bincode::serialize(&response)
                .map_err(|e| DbError::Serialization(e.to_string()))?;
            
            socket.write_all(&response_bytes).await
                .map_err(|e| DbError::Network(e.to_string()))?;
        }
        
        Ok(())
    }
    
    async fn process_request(&self, request: Request) -> Response {
        match request {
            Request::Query { sql } => {
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
            Request::BeginTransaction => {
                match self.txn_manager.begin() {
                    Ok(txn_id) => Response::TransactionId(txn_id),
                    Err(e) => Response::Error(e.to_string()),
                }
            }
            Request::Commit => Response::Ok,
            Request::Rollback => Response::Ok,
            Request::Ping => Response::Pong,
        }
    }
}


