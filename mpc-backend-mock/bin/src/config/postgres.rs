use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::postgres::PgSslMode;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PostgresConfig {
    #[serde(default = "PostgresConfig::default_host")]
    pub host: String,

    #[serde(default = "PostgresConfig::default_port")]
    pub port: u16,

    #[serde(default = "PostgresConfig::default_database")]
    pub database: String,

    #[serde(default = "PostgresConfig::default_username")]
    pub username: String,

    #[serde(default = "PostgresConfig::default_password")]
    pub password: String,

    #[serde(default = "PostgresConfig::default_role")]
    pub role: Option<String>,

    #[serde(
        default = "PostgresConfig::default_ssl_mode",
        serialize_with = "serialize_pg_ssl_mode",
        deserialize_with = "deserialize_pg_ssl_mode"
    )]
    pub ssl_mode: PgSslMode,

    #[serde(default = "PostgresConfig::default_max_connections")]
    pub max_connections: u32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_name: Option<String>,
}

impl PostgresConfig {
    #[inline]
    pub fn default_host() -> String { "127.0.0.1".to_string() }

    #[inline]
    pub const fn default_port() -> u16 { 5432 }

    #[inline]
    pub fn default_database() -> String { "postgres".to_string() }

    #[inline]
    pub fn default_username() -> String { "postgres".to_string() }

    #[inline]
    pub fn default_password() -> String { "postgres".to_string() }

    #[inline]
    pub const fn default_role() -> Option<String> { None }

    #[inline]
    pub const fn default_ssl_mode() -> PgSslMode { PgSslMode::Prefer }

    #[inline]
    pub const fn default_max_connections() -> u32 { 100 }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            database: Self::default_database(),
            username: Self::default_username(),
            password: Self::default_password(),
            role: Self::default_role(),
            ssl_mode: Self::default_ssl_mode(),
            max_connections: Self::default_max_connections(),
            application_name: None,
        }
    }
}

impl From<PostgresConfig> for mpc_backend_mock_core::config::PostgresConfig {
    fn from(
        PostgresConfig {
            host,
            port,
            database,
            username,
            password,
            role,
            ssl_mode,
            max_connections,
            application_name,
        }: PostgresConfig,
    ) -> Self {
        Self {
            host,
            port,
            database,
            username,
            password,
            role,
            ssl_mode,
            max_connections,
            application_name,
        }
    }
}

/// SAFETY: `serialize_with` use reference
#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_pg_ssl_mode<S>(ssl_mode: &PgSslMode, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ssl_mode_str = match ssl_mode {
        PgSslMode::Disable => "disable",
        PgSslMode::Allow => "allow",
        PgSslMode::Prefer => "prefer",
        PgSslMode::Require => "require",
        PgSslMode::VerifyCa => "verify-ca",
        PgSslMode::VerifyFull => "verify-full",
    };
    s.serialize_str(ssl_mode_str)
}

fn deserialize_pg_ssl_mode<'de, D>(deserializer: D) -> Result<PgSslMode, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    PgSslMode::from_str(&buf).map_err(serde::de::Error::custom)
}
