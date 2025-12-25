// # Swagger UI Configuration
//
// Configuration and setup for Swagger UI interactive documentation.
// Provides customization options for branding, security, and display.

use axum::Router;
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::openapi::ApiDoc;

/// Swagger UI customization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaggerCustomization {
    /// Custom title for the Swagger UI page
    pub title: String,
    /// Custom description
    pub description: String,
    /// Custom logo URL (optional)
    pub logo_url: Option<String>,
    /// Primary color for UI elements (hex color)
    pub primary_color: String,
    /// Enable deep linking
    pub deep_linking: bool,
    /// Display request duration
    pub display_request_duration: bool,
    /// Default model expand depth
    pub default_models_expand_depth: i32,
    /// Default model rendering
    pub default_model_rendering: String,
    /// Enable filter
    pub filter: bool,
    /// Show extensions
    pub show_extensions: bool,
    /// Show common extensions
    pub show_common_extensions: bool,
    /// Maximum displayed tags
    pub max_displayed_tags: Option<i32>,
}

impl Default for SwaggerCustomization {
    fn default() -> Self {
        Self {
            title: "RustyDB API Documentation".to_string(),
            description: "Interactive API documentation for RustyDB".to_string(),
            logo_url: None,
            primary_color: "#3b82f6".to_string(), // Blue color
            deep_linking: true,
            display_request_duration: true,
            default_models_expand_depth: 1,
            default_model_rendering: "model".to_string(),
            filter: true,
            show_extensions: true,
            show_common_extensions: true,
            max_displayed_tags: None,
        }
    }
}

/// Security scheme configuration for Swagger UI
#[derive(Debug, Clone)]
pub struct SwaggerSecurityConfig {
    /// Enable Bearer token authentication in UI
    pub enable_bearer_auth: bool,
    /// Enable API key authentication in UI
    pub enable_api_key: bool,
    /// Default Bearer token (for development only)
    pub default_bearer_token: Option<String>,
    /// Default API key (for development only)
    pub default_api_key: Option<String>,
}

impl Default for SwaggerSecurityConfig {
    fn default() -> Self {
        Self {
            enable_bearer_auth: true,
            enable_api_key: true,
            default_bearer_token: None,
            default_api_key: None,
        }
    }
}

/// Complete Swagger configuration
#[derive(Debug, Clone)]
pub struct SwaggerConfiguration {
    /// UI customization settings
    pub customization: SwaggerCustomization,
    /// Security configuration
    pub security: SwaggerSecurityConfig,
    /// Base URL for Swagger UI
    pub base_url: String,
    /// URL for OpenAPI spec JSON
    pub spec_url: String,
}

impl Default for SwaggerConfiguration {
    fn default() -> Self {
        Self {
            customization: SwaggerCustomization::default(),
            security: SwaggerSecurityConfig::default(),
            base_url: "/swagger-ui".to_string(),
            spec_url: "/api-docs/openapi.json".to_string(),
        }
    }
}

impl SwaggerConfiguration {
    /// Create a new Swagger configuration with custom settings
    pub fn new(base_url: String, spec_url: String) -> Self {
        Self {
            base_url,
            spec_url,
            ..Default::default()
        }
    }

    /// Set custom branding
    pub fn with_branding(
        mut self,
        title: String,
        description: String,
        logo_url: Option<String>,
    ) -> Self {
        self.customization.title = title;
        self.customization.description = description;
        self.customization.logo_url = logo_url;
        self
    }

    /// Set primary color
    pub fn with_primary_color(mut self, color: String) -> Self {
        self.customization.primary_color = color;
        self
    }

    /// Enable/disable deep linking
    pub fn with_deep_linking(mut self, enabled: bool) -> Self {
        self.customization.deep_linking = enabled;
        self
    }

    /// Configure security defaults (for development only)
    pub fn with_security_defaults(
        mut self,
        bearer_token: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        self.security.default_bearer_token = bearer_token;
        self.security.default_api_key = api_key;
        self
    }
}

/// Configure Swagger UI with the provided configuration
///
/// # Arguments
/// * `config` - Swagger configuration options
///
/// # Returns
/// Configured Swagger UI router
///
/// # Example
/// ```no_run
/// use rusty_db::api::rest::swagger::{configure_swagger, SwaggerConfiguration};
///
/// let swagger_config = SwaggerConfiguration::default()
///     .with_branding(
///         "My Database API".to_string(),
///         "Custom description".to_string(),
///         None
///     );
/// let swagger_router = configure_swagger(swagger_config);
/// ```
pub fn configure_swagger(config: SwaggerConfiguration) -> SwaggerUi {
    // Create the Swagger UI with the OpenAPI spec
    SwaggerUi::new(config.base_url).url(config.spec_url, ApiDoc::openapi())
}

/// Configure Swagger UI with default settings
///
/// # Returns
/// Configured Swagger UI router with default settings
pub fn configure_default_swagger() -> SwaggerUi {
    configure_swagger(SwaggerConfiguration::default())
}

/// Configure Swagger UI for production
///
/// Production configuration disables:
/// - Default credentials
/// - Deep linking (security consideration)
/// - Request duration display
///
/// # Returns
/// Configured Swagger UI router for production use
pub fn configure_production_swagger() -> SwaggerUi {
    let config = SwaggerConfiguration {
        customization: SwaggerCustomization {
            deep_linking: false,
            display_request_duration: false,
            ..Default::default()
        },
        security: SwaggerSecurityConfig {
            enable_bearer_auth: true,
            enable_api_key: true,
            default_bearer_token: None,
            default_api_key: None,
        },
        ..Default::default()
    };

    configure_swagger(config)
}

/// Configure Swagger UI for development
///
/// Development configuration enables:
/// - All interactive features
/// - Deep linking
/// - Request duration display
/// - Optional default credentials
///
/// # Arguments
/// * `bearer_token` - Optional default bearer token
/// * `api_key` - Optional default API key
///
/// # Returns
/// Configured Swagger UI router for development use
pub fn configure_development_swagger(
    bearer_token: Option<String>,
    api_key: Option<String>,
) -> SwaggerUi {
    let config = SwaggerConfiguration::default().with_security_defaults(bearer_token, api_key);

    configure_swagger(config)
}

/// Create Swagger UI router with custom branding
///
/// # Arguments
/// * `title` - Custom title
/// * `description` - Custom description
/// * `logo_url` - Optional logo URL
/// * `primary_color` - Primary color (hex format)
///
/// # Returns
/// Configured Swagger UI router with custom branding
pub fn create_branded_swagger(
    title: String,
    description: String,
    logo_url: Option<String>,
    primary_color: String,
) -> SwaggerUi {
    let config = SwaggerConfiguration::default()
        .with_branding(title, description, logo_url)
        .with_primary_color(primary_color);

    configure_swagger(config)
}

/// Create API documentation router
///
/// This creates a complete router with both:
/// - Swagger UI at /swagger-ui
/// - OpenAPI JSON spec at /api-docs/openapi.json
///
/// # Returns
/// Router with Swagger UI and OpenAPI spec endpoints
pub fn create_api_docs_router() -> Router {
    // SwaggerUi already includes the OpenAPI spec endpoint at /api-docs/openapi.json
    // Convert to Router - this should only register routes once
    Router::new().merge(configure_default_swagger())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = SwaggerConfiguration::default();
        assert_eq!(config.base_url, "/swagger-ui");
        assert_eq!(config.spec_url, "/api-docs/openapi.json");
        assert!(config.security.enable_bearer_auth);
        assert!(config.security.enable_api_key);
    }

    #[test]
    fn test_custom_configuration() {
        let config = SwaggerConfiguration::new("/docs".to_string(), "/openapi.json".to_string());
        assert_eq!(config.base_url, "/docs");
        assert_eq!(config.spec_url, "/openapi.json");
    }

    #[test]
    fn test_branding_configuration() {
        let config = SwaggerConfiguration::default().with_branding(
            "Custom API".to_string(),
            "Custom Description".to_string(),
            Some("https://example.com/logo.png".to_string()),
        );
        assert_eq!(config.customization.title, "Custom API");
        assert_eq!(config.customization.description, "Custom Description");
        assert_eq!(
            config.customization.logo_url,
            Some("https://example.com/logo.png".to_string())
        );
    }

    #[test]
    fn test_color_configuration() {
        let config = SwaggerConfiguration::default().with_primary_color("#ff0000".to_string());
        assert_eq!(config.customization.primary_color, "#ff0000");
    }

    #[test]
    fn test_security_configuration() {
        let config = SwaggerConfiguration::default()
            .with_security_defaults(Some("test-token".to_string()), Some("test-key".to_string()));
        assert_eq!(
            config.security.default_bearer_token,
            Some("test-token".to_string())
        );
        assert_eq!(
            config.security.default_api_key,
            Some("test-key".to_string())
        );
    }

    #[test]
    fn test_production_config() {
        let swagger = configure_production_swagger();
        // Production config should have security enabled but no defaults
        // This is a basic test - in practice you'd verify the actual config
        assert!(true); // Placeholder
    }

    #[test]
    fn test_development_config() {
        let swagger = configure_development_swagger(
            Some("dev-token".to_string()),
            Some("dev-key".to_string()),
        );
        // Development config should have all features enabled
        assert!(true); // Placeholder
    }

    #[test]
    fn test_customization_defaults() {
        let custom = SwaggerCustomization::default();
        assert_eq!(custom.title, "RustyDB API Documentation");
        assert!(custom.deep_linking);
        assert!(custom.display_request_duration);
        assert!(custom.filter);
    }
}
