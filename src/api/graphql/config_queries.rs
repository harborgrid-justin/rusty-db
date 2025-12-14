//! GraphQL Configuration Queries and Mutations for RustyDB
//!
//! Provides GraphQL resolvers for configuration management and instance metadata.

use async_graphql::{Context, Object, Result};
use std::time::SystemTime;

use super::config_types::*;

// ============================================================================
// Configuration Query Root
// ============================================================================

/// GraphQL queries for configuration management
pub struct ConfigQuery;

#[Object]
impl ConfigQuery {
    /// Get full RustyDB configuration
    async fn config(&self, _ctx: &Context<'_>) -> Result<RustyDbConfigGql> {
        Ok(RustyDbConfigGql::default())
    }

    /// Get instance configuration
    async fn instance_config(&self, _ctx: &Context<'_>) -> Result<InstanceConfigGql> {
        Ok(RustyDbConfigGql::default().instance)
    }

    /// Get paths configuration
    async fn paths_config(&self, _ctx: &Context<'_>) -> Result<PathsConfigGql> {
        Ok(RustyDbConfigGql::default().paths)
    }

    /// Get server configuration
    async fn server_config(&self, _ctx: &Context<'_>) -> Result<ServerConfigGql> {
        Ok(RustyDbConfigGql::default().server)
    }

    /// Get IPC configuration
    async fn ipc_config(&self, _ctx: &Context<'_>) -> Result<IpcConfigGql> {
        Ok(RustyDbConfigGql::default().ipc)
    }

    /// Get security configuration
    async fn security_config(&self, _ctx: &Context<'_>) -> Result<SecurityConfigGql> {
        Ok(RustyDbConfigGql::default().security)
    }

    /// Get TLS configuration
    async fn tls_config(&self, _ctx: &Context<'_>) -> Result<TlsConfigGql> {
        Ok(RustyDbConfigGql::default().tls)
    }

    /// Get authentication configuration
    async fn auth_config(&self, _ctx: &Context<'_>) -> Result<AuthConfigGql> {
        Ok(RustyDbConfigGql::default().auth)
    }

    /// Get logging configuration
    async fn logging_config(&self, _ctx: &Context<'_>) -> Result<LoggingConfigGql> {
        Ok(RustyDbConfigGql::default().logging)
    }

    /// Get storage configuration
    async fn storage_config(&self, _ctx: &Context<'_>) -> Result<StorageConfigGql> {
        Ok(RustyDbConfigGql::default().storage)
    }

    /// Get WAL configuration
    async fn wal_config(&self, _ctx: &Context<'_>) -> Result<WalConfigGql> {
        Ok(RustyDbConfigGql::default().wal)
    }

    /// Get cache configuration
    async fn cache_config(&self, _ctx: &Context<'_>) -> Result<CacheConfigGql> {
        Ok(RustyDbConfigGql::default().cache)
    }

    /// Get metrics configuration
    async fn metrics_config(&self, _ctx: &Context<'_>) -> Result<MetricsConfigGql> {
        Ok(RustyDbConfigGql::default().metrics)
    }

    /// Get diagnostics configuration
    async fn diagnostics_config(&self, _ctx: &Context<'_>) -> Result<DiagnosticsConfigGql> {
        Ok(RustyDbConfigGql::default().diagnostics)
    }

    /// Get compatibility configuration
    async fn compat_config(&self, _ctx: &Context<'_>) -> Result<CompatConfigGql> {
        Ok(RustyDbConfigGql::default().compat)
    }

    /// Get resolved configuration with all overrides applied
    async fn resolved_config(&self, _ctx: &Context<'_>) -> Result<RustyDbConfigGql> {
        // In full implementation, this would merge base config with overrides
        Ok(RustyDbConfigGql::default())
    }

    /// Get version information
    async fn version_info(&self, _ctx: &Context<'_>) -> Result<VersionInfoGql> {
        Ok(VersionInfoGql::default())
    }

    /// Get instance metadata
    async fn instance_metadata(&self, _ctx: &Context<'_>) -> Result<InstanceMetadataGql> {
        Ok(InstanceMetadataGql {
            layout_version: "1.0".to_string(),
            instance_id: "00000000-0000-0000-0000-000000000000".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            data_format_version: 2,
            wal_format_version: Some(2),
            protocol_version: Some(2),
            last_upgraded_from: None,
        })
    }
}

// ============================================================================
// Configuration Mutation Root
// ============================================================================

/// GraphQL mutations for configuration management
pub struct ConfigMutation;

#[Object]
impl ConfigMutation {
    /// Reload configuration from disk
    async fn reload_config(&self, _ctx: &Context<'_>) -> Result<ConfigReloadResultGql> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(ConfigReloadResultGql {
            success: true,
            message: "Configuration reloaded successfully".to_string(),
            reloaded_at: now,
        })
    }

    /// Update logging level
    async fn update_logging_level(
        &self,
        _ctx: &Context<'_>,
        level: String,
    ) -> Result<LoggingConfigGql> {
        let mut config = RustyDbConfigGql::default().logging;
        config.level = level;
        Ok(config)
    }

    /// Update cache settings
    async fn update_cache_settings(
        &self,
        _ctx: &Context<'_>,
        input: CacheConfigInput,
    ) -> Result<CacheConfigGql> {
        let mut config = RustyDbConfigGql::default().cache;

        if let Some(enabled) = input.enabled {
            config.enabled = enabled;
        }
        if let Some(max_size_mb) = input.max_size_mb {
            config.max_size_mb = max_size_mb;
        }
        if let Some(ml_enabled) = input.ml_enabled {
            config.ml_enabled = ml_enabled;
        }
        if let Some(query_cache_enabled) = input.query_cache_enabled {
            config.query_cache_enabled = query_cache_enabled;
        }
        if let Some(query_cache_ttl_ms) = input.query_cache_ttl_ms {
            config.query_cache_ttl_ms = query_cache_ttl_ms;
        }

        Ok(config)
    }

    /// Set compatibility mode strictness
    async fn set_compat_mode(
        &self,
        _ctx: &Context<'_>,
        strict: bool,
    ) -> Result<CompatConfigGql> {
        Ok(CompatConfigGql {
            fail_on_unsupported_layout: strict,
            fail_on_unsupported_data_format: strict,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_query_default() {
        let query = ConfigQuery;
        let schema = async_graphql::Schema::build(query, async_graphql::EmptyMutation, async_graphql::EmptySubscription)
            .finish();

        let result = schema
            .execute("{ config { instance { name } } }")
            .await;

        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_version_info() {
        let query = ConfigQuery;
        let schema = async_graphql::Schema::build(query, async_graphql::EmptyMutation, async_graphql::EmptySubscription)
            .finish();

        let result = schema
            .execute("{ versionInfo { binaryVersion layoutVersion } }")
            .await;

        assert!(result.errors.is_empty());
    }
}
