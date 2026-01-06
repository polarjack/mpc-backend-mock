// FIXME: remove this after this utoipa issue is fixed: https://github.com/juhaku/utoipa/pull/1423
#![allow(clippy::needless_for_each)]
mod error;
mod user;

use axum::{middleware, routing, Extension, Router};
use mpc_backend_mock_core::ServerInfo;
use utoipa::OpenApi;
use zeus_axum::response::EncapsulatedJson;

pub use self::error::{Error, Result};
use crate::{web::middleware::jwt_auth_middleware, ServiceState};

pub fn api_v1_router(service_state: &ServiceState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/v1/info", routing::get(server_info))
        .route("/v1/users", routing::post(user::create_user));

    // Protected routes (authentication required)
    let protected_routes =
        Router::new().route("/v1/users/me", routing::get(user::get_current_user)).layer(
            middleware::from_fn_with_state(service_state.jwks_client.clone(), jwt_auth_middleware),
        );

    Router::new()
        .nest("/api", public_routes)
        .nest("/api", protected_routes)
        .with_state(service_state.clone())
}

/// Get server info
#[utoipa::path(
    get,
    operation_id = "get_server_info",
    path = "/api/v1/info",
    responses(
        (status = 200, body = ServerInfo)
    )
)]
pub async fn server_info(
    Extension(server_info): Extension<ServerInfo>,
) -> Result<EncapsulatedJson<ServerInfo>> {
    Ok(EncapsulatedJson::ok(server_info))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        server_info,
        user::create_user,
        user::get_current_user,
    ),
    components(schemas(
        ServerInfo,
        crate::entity::User,
        crate::entity::UserInfo,
        crate::entity::CreateUserRequest,
        crate::entity::CreateUserResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Users", description = "User management endpoints")
    )
)]
pub struct ApiDoc;

/// Security scheme for JWT bearer authentication
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}
