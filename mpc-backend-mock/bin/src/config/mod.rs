mod bitcoin;
mod error;
mod health_check;
mod key_management_service;
mod keycloak;
mod metrics;
mod postgres;
mod solana;
mod web;

use std::path::{Path, PathBuf};

use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use zeus_cli_common::config::LogConfig;

use self::key_management_service::KeyManagementService;
pub use self::{
    bitcoin::BitcoinConfig,
    error::Error,
    health_check::HealthCheckConfig,
    keycloak::{JwtValidationMethod, KeycloakConfig},
    metrics::MetricsConfig,
    postgres::PostgresConfig,
    solana::SolanaConfig,
    web::WebConfig,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub web: WebConfig,

    #[serde(default)]
    pub postgres: PostgresConfig,

    #[serde(default)]
    pub health_check: HealthCheckConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,

    pub bitcoin: BitcoinConfig,

    pub solana: SolanaConfig,

    pub key_management_service: Option<KeyManagementService>,

    #[serde(default)]
    pub keycloak: KeycloakConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: LogConfig::default(),
            web: WebConfig::default(),
            postgres: PostgresConfig::default(),
            health_check: HealthCheckConfig::default(),
            metrics: MetricsConfig::default(),
            bitcoin: BitcoinConfig::devnet(),
            solana: SolanaConfig::devnet(),
            key_management_service: None,
            keycloak: KeycloakConfig::default(),
        }
    }
}

impl Config {
    #[inline]
    pub fn default_path() -> PathBuf {
        [
            mpc_backend_mock_core::PROJECT_CONFIG_DIR.to_path_buf(),
            PathBuf::from(mpc_backend_mock_core::CONFIG_NAME),
        ]
        .into_iter()
        .collect()
    }

    #[inline]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut config: Self = {
            let data = std::fs::read_to_string(&path)
                .context(error::OpenConfigSnafu { filename: path.as_ref().to_path_buf() })?;

            serde_yaml::from_str(&data)
                .context(error::ParseConfigSnafu { filename: path.as_ref().to_path_buf() })?
        };

        config.log.file_path = match config.log.file_path.map(|path| {
            path.try_resolve()
                .map(|path| path.to_path_buf())
                .with_context(|_| error::ResolveFilePathSnafu { file_path: path.clone() })
        }) {
            Some(Ok(path)) => Some(path),
            Some(Err(err)) => return Err(err),
            None => None,
        };

        Ok(config)
    }
}

#[inline]
pub async fn load_server_config(
    Config {
        web,
        postgres,
        metrics,
        health_check,
        bitcoin,
        solana,
        keycloak,
        key_management_service: kms,
        ..
    }: Config,
) -> Result<mpc_backend_mock_core::config::Config, Error> {
    let _kms = if let Some(kms) = kms {
        tracing::info!("Load KMS client");
        Some(kms.load().await?)
    } else {
        None
    };
    let bitcoin = bitcoin.try_into()?;

    Ok(mpc_backend_mock_core::config::Config {
        web: web.into(),
        postgres: postgres.into(),
        metrics: metrics.into(),
        health_check_listen_address: health_check.socket_address(),
        bitcoin,
        solana: solana.into(),
        keycloak: mpc_backend_mock_core::config::KeycloakConfig {
            server_url: keycloak.server_url,
            realm: keycloak.realm,
            client_id: keycloak.client_id,
            client_secret: keycloak.client_secret,
            admin_username: keycloak.admin_username,
            admin_password: keycloak.admin_password,
            verify_ssl: keycloak.verify_ssl,
            jwt_validation_method: match keycloak.jwt_validation_method {
                crate::config::JwtValidationMethod::Jwks => {
                    mpc_backend_mock_core::config::JwtValidationMethod::Jwks
                }
                crate::config::JwtValidationMethod::Introspection => {
                    mpc_backend_mock_core::config::JwtValidationMethod::Introspection
                }
            },
        },
    })
}
