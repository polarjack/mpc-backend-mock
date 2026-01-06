pub mod controller;
pub mod error;
pub mod extractor;
pub mod middleware;

use std::{future::Future, net::SocketAddr, sync::Arc};

use axum::{
    extract::Request, http, response::IntoResponse, routing, Extension, Json, Router, ServiceExt,
};
use eris_bitcoin_rpc_client::Client as BitcoinRpcClient;
use keycloak::KeycloakAdmin;
use mpc_backend_mock_core::ServerInfo;
use snafu::ResultExt;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::{Layer, ServiceBuilder};
use tower_http::{
    compression::CompressionLayer, normalize_path::NormalizePathLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use zeus_axum::{json_response, response::EncapsulatedJsonError};
use zpl_rpc_client::RpcClient as ZplRpcClient;

pub use self::{controller::ApiDoc, error::Error};
use crate::service::UserManagementService;

pub async fn new_api_server<ShutdownSignal>(
    socket_address: SocketAddr,
    service_state: ServiceState,
    server_info: ServerInfo,
    shutdown_signal: ShutdownSignal,
) -> Result<(), Error>
where
    ShutdownSignal: Future<Output = ()> + Send + 'static,
{
    let middleware_stack =
        ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CompressionLayer::new());

    let router = {
        let router = Router::new()
            // For GKE load balancer default health check
            .route("/", routing::get(controller::server_info))
            .route(
                "/openapi.json",
                routing::get(openapi_json),
            )
            .merge(controller::api_v1_router(&service_state))
            .layer(Extension(server_info))
            .layer(middleware_stack)
            .fallback(fallback);
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(router)
    };

    let listener = TcpListener::bind(&socket_address).await.context(error::BindTcpServerSnafu)?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|err| Error::ServeHttpServer { message: err.to_string() })
}

// SAFETY: `axum` handler must be async
#[allow(clippy::unused_async)]
async fn fallback(uri: http::Uri) -> axum::response::Response {
    json_response! {
        status: http::StatusCode::NOT_FOUND,
        error: zeus_axum::response::Error {
            type_: zeus_axum::response::ErrorType::NotFound,
            message: format!("No route for {uri}"),
            additional_fields: indexmap::IndexMap::default(),
        }
    }
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> { Json(ApiDoc::openapi()) }

#[derive(Clone)]
pub struct ServiceState {
    pub bitcoin_rpc_client: BitcoinRpcClient,
    pub zpl_rpc_client: ZplRpcClient,
    pub user_management_service: UserManagementService,
    pub jwks_client: middleware::JwksClient,
}

impl ServiceState {
    /// Create a new service state
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        database: PgPool,
        bitcoin_rpc_client: &BitcoinRpcClient,
        zpl_rpc_client: ZplRpcClient,
        jwks_client: middleware::JwksClient,
        keycloak_admin: Arc<KeycloakAdmin>,
        keycloak_realm: String,
    ) -> Self {
        let user_management_service =
            UserManagementService::new(database.clone(), keycloak_admin, keycloak_realm);

        Self {
            bitcoin_rpc_client: bitcoin_rpc_client.clone(),
            zpl_rpc_client,
            user_management_service,
            jwks_client,
        }
    }
}
