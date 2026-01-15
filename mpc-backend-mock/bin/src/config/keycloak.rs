use serde::{Deserialize, Serialize};

/// JWT validation method
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum JwtValidationMethod {
    /// Local JWT validation using JWKS (faster, cached)
    #[default]
    Jwks,
    /// Server-side token introspection (real-time, authoritative)
    Introspection,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeycloakConfig {
    /// Keycloak server URL (e.g., <http://localhost:8080>)
    #[serde(default = "KeycloakConfig::default_server_url")]
    pub server_url: String,

    /// Keycloak realm name (e.g., "mpc")
    #[serde(default = "KeycloakConfig::default_realm")]
    pub realm: String,

    /// Client ID for backend service
    #[serde(default = "KeycloakConfig::default_client_id")]
    pub client_id: String,

    /// Client secret for backend service
    #[serde(default = "KeycloakConfig::default_client_secret")]
    pub client_secret: String,

    /// Admin client ID for user management operations (service account)
    #[serde(default = "KeycloakConfig::default_admin_client_id")]
    pub admin_client_id: String,

    /// Admin client secret for user management operations (service account)
    #[serde(default = "KeycloakConfig::default_admin_client_secret")]
    pub admin_client_secret: String,

    /// Enable TLS certificate verification
    #[serde(default = "KeycloakConfig::default_verify_ssl")]
    pub verify_ssl: bool,

    /// JWT validation method
    #[serde(default)]
    pub jwt_validation_method: JwtValidationMethod,
}

impl KeycloakConfig {
    #[inline]
    pub fn default_server_url() -> String { "http://localhost:8080".to_string() }

    #[inline]
    pub fn default_realm() -> String { "mpc".to_string() }

    #[inline]
    pub fn default_client_id() -> String { "mpc-backend".to_string() }

    #[inline]
    pub fn default_client_secret() -> String { "changeme".to_string() }

    #[inline]
    pub fn default_admin_client_id() -> String { "admin-cli".to_string() }

    #[inline]
    pub fn default_admin_client_secret() -> String { "changeme".to_string() }

    #[inline]
    pub const fn default_verify_ssl() -> bool { true }
}

impl Default for KeycloakConfig {
    fn default() -> Self {
        Self {
            server_url: Self::default_server_url(),
            realm: Self::default_realm(),
            client_id: Self::default_client_id(),
            client_secret: Self::default_client_secret(),
            admin_client_id: Self::default_admin_client_id(),
            admin_client_secret: Self::default_admin_client_secret(),
            verify_ssl: Self::default_verify_ssl(),
            jwt_validation_method: JwtValidationMethod::default(),
        }
    }
}
