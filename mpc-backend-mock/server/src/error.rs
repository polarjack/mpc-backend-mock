use std::borrow::Cow;

use snafu::Snafu;

use crate::web;

pub type Result<T> = std::result::Result<T, Error>;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display(
        "Can not initialize Postgres pool with endpoint \
         `postgres://{username}@{host}:{port}/{database}`, error: {source}"
    ))]
    InitializePostgresPool {
        host: Cow<'static, str>,
        port: u16,
        username: Cow<'static, str>,
        database: Cow<'static, str>,
        source: sqlx::error::Error,
    },

    #[snafu(display("Fail to migrate postgres schema, error: {source}",))]
    MigrateSchema { source: sqlx::migrate::MigrateError },

    #[snafu(display("Error occurs while creating Bitcoin RPC client, error: {source}"))]
    CreateBitcoinRpcClient { source: eris_bitcoin_rpc_client::Error },

    #[snafu(display("Failed to access Bitcoin endpoint via {endpoint}, error: {source}"))]
    CheckBitcoinEndpoint { endpoint: http::Uri, source: eris_bitcoin_rpc_client::Error },

    #[snafu(display("{source}"))]
    Metrics { source: zeus_metrics::Error },

    #[snafu(display("Error occurs while starting tonic server, error: {source}"))]
    StartTonicServer { source: tonic::transport::Error },

    #[snafu(display("{source}"))]
    Web { source: web::Error },

    #[snafu(display("Failed to convert string `{address}` into solana public key: {source}"))]
    InvalidSolanaAddress { address: String, source: solana_sdk::pubkey::ParsePubkeyError },

    #[snafu(display("Failed to initialize JWKS client: {message}"))]
    InitializeJwksClient { message: String },

    #[snafu(display("Failed to initialize Keycloak admin client: {message}"))]
    InitializeKeycloakAdmin { message: String },

    #[snafu(display("Failed to initialize Keycloak client: {message}"))]
    InitializeKeycloakClient { message: String },
}

impl From<zeus_metrics::Error> for Error {
    fn from(source: zeus_metrics::Error) -> Self { Self::Metrics { source } }
}

impl From<web::Error> for Error {
    fn from(source: web::Error) -> Self { Self::Web { source } }
}
