pub mod entity;
mod error;
mod grpc;
mod service;
mod web;

use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc};

use eris_bitcoin_rpc_client::Client as BitcoinRpcClient;
use futures::{future::BoxFuture, FutureExt};
use mpc_backend_mock_core::{
    config::{BitcoinConfig, Config, KeycloakConfig, PostgresConfig, SolanaConfig},
    ServerInfo,
};
use sigfinn::{ExitStatus, LifecycleManager, Shutdown};
use snafu::ResultExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
    Executor, PgPool,
};
use tracing::Instrument;
use zeus_metrics::DefaultMetrics;
use zeus_protobuf_types::health_check::HealthServer;
use zpl_rpc_client::RpcClient as ZplRpcClient;

use self::grpc::HealthCheckService;
pub use self::{
    error::{Error, Result},
    web::{controller, middleware::JwksClient, ApiDoc, ServiceState},
};

const MIGRATOR: Migrator = Migrator { ignore_missing: true, ..sqlx::migrate!() };

/// # Errors
/// Returns errors when server fails to start
pub async fn serve_with_shutdown(config: Config, server_info: ServerInfo) -> Result<()> {
    let Config { postgres, web, bitcoin, solana, metrics, health_check_listen_address, keycloak } =
        config;

    let database = initialize_postgres_pool(&postgres).await?;

    let bitcoin_rpc_client = initialize_bitcoin_rpc_client(&bitcoin).await?;

    let _solana_rpc_client = initialize_solana_rpc_client(solana.endpoint.url.to_string());

    let zpl_rpc_client = initialize_zpl_rpc_client(solana).await;

    let jwks_client = initialize_jwks_client(&keycloak)?;

    let keycloak_admin = Arc::new(initialize_keycloak_admin(&keycloak).await?);

    let service_state = ServiceState::new(
        database.clone(),
        &bitcoin_rpc_client,
        zpl_rpc_client,
        jwks_client,
        keycloak_admin,
        keycloak.realm,
    );

    let lifecycle_manager = LifecycleManager::<Error>::new();

    let _handle = lifecycle_manager
        .spawn(
            "Health check server",
            create_grpc_health_check_server_future(
                health_check_listen_address,
                bitcoin_rpc_client,
                database.clone(),
            ),
        )
        .spawn(
            "Http Server",
            create_web_http_server_future(web.listen_address, service_state, server_info),
        );

    if metrics.enable {
        let default_metrics = DefaultMetrics::new()?;

        let _handle = lifecycle_manager.spawn(
            "Metrics server",
            create_metrics_server_future(metrics.listen_address, default_metrics),
        );
    }

    if let Ok(Err(err)) = lifecycle_manager.serve().await {
        tracing::error!("{err}");
        Err(err)
    } else {
        Ok(())
    }
}

#[tracing::instrument(
    skip(password, database, ssl_mode, max_connections),
    fields(
        host = %host,
        port = port,
        username = %username
    )
)]
async fn initialize_postgres_pool(
    PostgresConfig {
        host,
        port,
        username,
        role,
        password,
        database,
        ssl_mode,
        max_connections,
        application_name,
    }: &PostgresConfig,
) -> Result<PgPool> {
    tracing::info!("Initializing database");

    let connect_opts = PgConnectOptions::new_without_pgpass()
        .host(host)
        .port(*port)
        .username(username)
        .password(password)
        .database(database)
        .ssl_mode(*ssl_mode);

    // append application name if provided
    let connect_opts = if let Some(app_name) = application_name {
        connect_opts.application_name(app_name)
    } else {
        connect_opts
    };

    let pool_opts = {
        let opts = PgPoolOptions::new().max_connections(*max_connections);

        if let Some(role) = role {
            let set_role = format!(r#"SET SESSION ROLE = "{role}";"#);
            opts.after_connect(move |conn, _meta| {
                let set_role = set_role.clone();
                async move {
                    let _ = conn.execute(set_role.as_str()).await?;
                    Ok(())
                }
                .boxed()
            })
        } else {
            opts
        }
    };
    let pool =
        pool_opts.connect_with(connect_opts).await.context(error::InitializePostgresPoolSnafu {
            host: host.to_string(),
            port: *port,
            username: username.to_string(),
            database: database.to_string(),
        })?;

    MIGRATOR
        .run(&pool)
        .instrument(tracing::info_span!("migrate"))
        .await
        .context(error::MigrateSchemaSnafu)?;

    Ok(pool)
}

#[tracing::instrument(
    skip(endpoint),
    fields(
        endpoint = %endpoint.endpoint,
        network = %endpoint.network,
        block_number_to_confirm = block_number_to_confirm
    )
)]
async fn initialize_bitcoin_rpc_client(
    BitcoinConfig { endpoint, block_number_to_confirm }: &BitcoinConfig,
) -> Result<BitcoinRpcClient> {
    tracing::info!("Initializing Bitcoin RPC client");

    let bitcoin_rpc_client = BitcoinRpcClient::new(endpoint.clone(), None)
        .await
        .context(error::CreateBitcoinRpcClientSnafu)?;

    // Check if Bitcoin RPC endpoint is accessible by getting current block height
    let (current_bitcoin_block_height, current_confirmed_bitcoin_block_height) = {
        let block_count = bitcoin_rpc_client
            .get_block_count()
            .await
            .context(error::CheckBitcoinEndpointSnafu { endpoint: endpoint.endpoint.clone() })?;
        (block_count, block_count.saturating_sub(*block_number_to_confirm))
    };

    tracing::info!(
        "Bitcoin (block height: {current_bitcoin_block_height}, confirmed block height: \
         {current_confirmed_bitcoin_block_height})"
    );

    Ok(bitcoin_rpc_client)
}

#[tracing::instrument]
fn initialize_solana_rpc_client(url: String) -> Arc<RpcClient> {
    tracing::info!("Initializing Solana RPC client");
    let commitment_config = CommitmentConfig::confirmed();

    Arc::new(RpcClient::new_with_commitment(url, commitment_config))
}

#[tracing::instrument(
    skip(endpoint),
    fields(
        url = %endpoint.url,
        cluster = %endpoint.cluster
    )
)]
async fn initialize_zpl_rpc_client(SolanaConfig { endpoint }: SolanaConfig) -> ZplRpcClient {
    tracing::info!("Initializing ZPL RPC client");

    ZplRpcClient::new(
        endpoint,
        CommitmentConfig::confirmed(),
        None,
        zpl_rpc_client::config::TransactionSimulation {
            enable: false,
            send_failed_solana_transaction: false,
        },
    )
}

#[tracing::instrument(
    skip(keycloak),
    fields(
        server_url = %keycloak.server_url,
        realm = %keycloak.realm
    )
)]
fn initialize_jwks_client(keycloak: &KeycloakConfig) -> Result<JwksClient> {
    tracing::info!("Initializing JWKS client");

    tracing::info!(
        "Keycloak JWKS endpoint: {}/realms/{}/protocol/openid-connect/certs",
        keycloak.server_url,
        keycloak.realm
    );

    JwksClient::new(&keycloak.server_url, &keycloak.realm).map_err(|err| {
        Error::InitializeJwksClient { message: format!("Failed to create JWKS client: {err}") }
    })
}

#[tracing::instrument(
    skip(keycloak),
    fields(
        server_url = %keycloak.server_url,
        realm = %keycloak.realm
    )
)]
async fn initialize_keycloak_admin(keycloak: &KeycloakConfig) -> Result<keycloak::KeycloakAdmin> {
    tracing::info!("Initializing Keycloak admin client");

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!keycloak.verify_ssl)
        .build()
        .map_err(|err| Error::InitializeKeycloakAdmin {
            message: format!("Failed to build HTTP client: {err}"),
        })?;

    // Acquire admin token using username/password authentication
    let admin_token = keycloak::KeycloakAdminToken::acquire(
        &keycloak.server_url,
        &keycloak.admin_username,
        &keycloak.admin_password,
        &client,
    )
    .await
    .map_err(|err| Error::InitializeKeycloakAdmin {
        message: format!("Failed to authenticate with Keycloak: {err}"),
    })?;

    // Create admin client with the acquired token
    let keycloak_admin = keycloak::KeycloakAdmin::new(&keycloak.server_url, admin_token, client);

    // GET /admin/realms/{realm} for verification
    let realm = keycloak_admin.realm_get(&keycloak.realm).await.map_err(|err| {
        Error::InitializeKeycloakAdmin {
            message: format!("Failed to verify Keycloak realm: {err}"),
        }
    })?;

    // Verify that the realm exists and matches the expected realm name
    if realm.realm.as_deref() != Some(&keycloak.realm) {
        return Err(Error::InitializeKeycloakAdmin {
            message: format!(
                "Keycloak realm '{}' does not exist or does not match the configured realm",
                keycloak.realm
            ),
        });
    }

    tracing::info!("Keycloak admin client initialized successfully");

    Ok(keycloak_admin)
}

fn create_web_http_server_future(
    listen_address: SocketAddr,
    service_state: ServiceState,
    server_info: ServerInfo,
) -> impl FnOnce(Shutdown) -> BoxFuture<'static, ExitStatus<Error>> {
    move |shutdown_signal| {
        async move {
            tracing::info!("Listen Web HTTP server endpoint on {listen_address}");

            let result =
                web::new_api_server(listen_address, service_state, server_info, shutdown_signal)
                    .await;

            match result {
                Ok(()) => {
                    tracing::info!("HTTP server is shut down gracefully");
                    ExitStatus::Success
                }
                Err(err) => ExitStatus::FatalError(Error::from(err)),
            }
        }
        .boxed()
    }
}

fn create_metrics_server_future<Metrics>(
    listen_address: SocketAddr,
    metrics: Metrics,
) -> impl FnOnce(Shutdown) -> Pin<Box<dyn Future<Output = ExitStatus<Error>> + Send>>
where
    Metrics: zeus_metrics::Metrics + 'static,
{
    move |signal| {
        async move {
            tracing::info!("Listen metrics endpoint on {listen_address}");
            let result = zeus_metrics::start_metrics_server(listen_address, metrics, signal).await;
            match result {
                Ok(()) => {
                    tracing::info!("Metrics server is shut down gracefully");
                    ExitStatus::Success
                }
                Err(err) => ExitStatus::FatalError(Error::from(err)),
            }
        }
        .boxed()
    }
}

fn create_grpc_health_check_server_future(
    listen_address: SocketAddr,
    bitcoin_rpc_client: BitcoinRpcClient,
    database: PgPool,
) -> impl FnOnce(Shutdown) -> BoxFuture<'static, ExitStatus<Error>> {
    move |signal| {
        async move {
            tracing::info!("Listen gRPC health check endpoint on {listen_address}");

            let result = tonic::transport::Server::builder()
                .add_service(HealthServer::new(HealthCheckService::new(
                    bitcoin_rpc_client,
                    database,
                )))
                .serve_with_shutdown(listen_address, signal)
                .await
                .context(error::StartTonicServerSnafu);

            match result {
                Ok(()) => {
                    tracing::info!("gRPC health check server is shut down gracefully");
                    ExitStatus::Success
                }
                Err(err) => ExitStatus::FatalError(err),
            }
        }
        .boxed()
    }
}
