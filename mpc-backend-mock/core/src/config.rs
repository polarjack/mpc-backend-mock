use std::{fmt::Debug, net::SocketAddr};

use sqlx::postgres::PgSslMode;

/// JWT validation method
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum JwtValidationMethod {
    /// Local JWT validation using JWKS (faster, cached)
    #[default]
    Jwks,
    /// Server-side token introspection (real-time, authoritative)
    Introspection,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub web: WebConfig,

    pub postgres: PostgresConfig,

    pub metrics: MetricsConfig,

    pub health_check_listen_address: SocketAddr,

    pub bitcoin: BitcoinConfig,

    pub solana: SolanaConfig,

    pub keycloak: KeycloakConfig,
}

#[derive(Clone, Debug)]
pub struct KeycloakConfig {
    pub server_url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: String,
    pub admin_client_id: String,
    pub admin_client_secret: String,
    pub verify_ssl: bool,
    pub jwt_validation_method: JwtValidationMethod,
}

#[derive(Clone, Debug)]
pub struct BitcoinConfig {
    pub endpoint: eris_bitcoin_rpc_client::RpcEndpoint,

    pub block_number_to_confirm: u64,
}

#[derive(Clone, Debug)]
pub struct SolanaConfig {
    pub endpoint: zpl_rpc_client::Endpoint,
}

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub listen_address: SocketAddr,
}

#[derive(Clone, Debug)]
pub struct MetricsConfig {
    pub enable: bool,

    pub listen_address: SocketAddr,
}

#[derive(Clone, Debug)]
pub struct PostgresConfig {
    pub host: String,

    pub port: u16,

    pub database: String,

    pub username: String,

    pub password: String,

    pub role: Option<String>,

    pub ssl_mode: PgSslMode,

    pub max_connections: u32,

    pub application_name: Option<String>,
}
