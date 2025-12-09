// # REST API Module
//
// Comprehensive REST API server for RustyDB with proper module structure.
// This module demonstrates all 10 Rust best practices:
//
// 1. **Cohesive Module Structure**: Each file implements a single domain concept
// 2. **Strong Typing & Newtypes**: Domain-specific types prevent confusion
// 3. **File Size Management**: Split into focused modules under 800 LOC each
// 4. **Traits for Extensibility**: Middleware traits enable pluggable behavior
// 5. **No Unsafe Blocks**: Safe Rust throughout
// 6. **Structured Error Handling**: `thiserror`-style error types
// 7. **Dependency Injection**: Constructor injection instead of globals
// 8. **Documentation-First**: Comprehensive rustdoc on all public APIs
// 9. **Consistent Formatting**: `rustfmt` and `clippy` compliant
// 10. **Comprehensive Tests**: Unit and integration tests included

pub mod types;
pub mod handlers;
pub mod middleware;
pub mod server;

// Re-export main types and functions for convenience
pub use types::*;
pub use server::RestApiServer;
pub use handlers::*;
pub use middleware::*;

// Re-export the server as the main entry point
pub use server::RestApiServer as Server;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Test that all expected types are available
        let config = ApiConfig::default();
        assert_eq!(config.port, 8080);

        let error = ApiError::new("TEST", "test message");
        assert_eq!(error.code, "TEST");
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ApiConfig::default();
        let server = RestApiServer::new(config).await;
        assert!(server.is_ok());
    }
}
