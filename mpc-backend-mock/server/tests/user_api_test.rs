use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use eris_bitcoin_ext::WellKnownNetwork as BitcoinNetwork;
use eris_bitcoin_rpc_client::Authentication as BitcoinRpcAuthentication;
use mpc_backend_mock_server::entity::{CreateUserRequest, CreateUserResponse};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// Helper function to create a test database pool
async fn create_test_pool() -> sqlx::PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/mpc_backend_mock".to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper to create a test router for integration tests
async fn create_test_app() -> axum::Router {
    let pool = create_test_pool().await;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.expect("Failed to run migrations");

    // Create mock Bitcoin and ZPL clients for testing
    // These are mock endpoints since we don't need actual blockchain connections
    // for user API tests
    let bitcoin_endpoint = eris_bitcoin_rpc_client::RpcEndpoint {
        endpoint: "http://localhost:8332".parse().unwrap(),
        indexer_endpoint: None,
        authentication: BitcoinRpcAuthentication::default(),
        support_quicknode_blockbook: false,
        network: BitcoinNetwork::Regtest,
    };

    let bitcoin_rpc_client = eris_bitcoin_rpc_client::Client::new(bitcoin_endpoint, None)
        .await
        .expect("Failed to create mock Bitcoin RPC client");

    // Use devnet endpoint creator from zpl_rpc_client
    let zpl_endpoint = zpl_rpc_client::Endpoint::devnet();
    let zpl_rpc_client = zpl_rpc_client::RpcClient::new(
        zpl_endpoint,
        solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        None,
        zpl_rpc_client::config::TransactionSimulation {
            enable: false,
            send_failed_solana_transaction: false,
        },
    );

    // Create mock JWKS client for testing
    let jwks_client = mpc_backend_mock_server::JwksClient::new("http://localhost:8080", "mpc")
        .expect("Failed to create mock JWKS client");

    // Initialize Keycloak admin client for testing
    let keycloak_config = mpc_backend_mock_core::config::KeycloakConfig {
        server_url: "http://localhost:8080".to_string(),
        realm: "mpc".to_string(),
        client_id: "mpc-backend-service".to_string(),
        client_secret: "test-secret".to_string(),
        admin_username: "admin".to_string(),
        admin_password: "admin".to_string(),
        verify_ssl: false,
        jwt_validation_method: mpc_backend_mock_core::config::JwtValidationMethod::Jwks,
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!keycloak_config.verify_ssl)
        .build()
        .expect("Failed to build HTTP client");

    let admin_token = keycloak::KeycloakAdminToken::acquire(
        &keycloak_config.server_url,
        &keycloak_config.admin_username,
        &keycloak_config.admin_password,
        &client,
    )
    .await
    .expect("Failed to authenticate with Keycloak");

    let keycloak_admin =
        Arc::new(keycloak::KeycloakAdmin::new(&keycloak_config.server_url, admin_token, client));

    let service_state = mpc_backend_mock_server::ServiceState::new(
        pool,
        &bitcoin_rpc_client,
        zpl_rpc_client,
        jwks_client,
        keycloak_admin,
        keycloak_config.realm.clone(),
        None,
        keycloak_config.jwt_validation_method.clone(),
    );

    // Create router using the exported controller module
    mpc_backend_mock_server::controller::api_v1_router(&service_state)
}

/// Helper to create the test server
async fn create_test_server() -> TestServer {
    let app = create_test_app().await;
    TestServer::new(app).expect("Failed to create test server")
}

/// Helper to clean up test user by email
async fn cleanup_test_user(pool: &sqlx::PgPool, email: &str) {
    let _ = sqlx::query!("DELETE FROM users WHERE email = $1", email).execute(pool).await;
}

#[tokio::test]
async fn test_create_user_success() {
    let server = create_test_server().await;
    let test_email = format!("test-{}@example.com", Uuid::new_v4());

    // Create user
    let response =
        server.post("/api/v1/users").json(&CreateUserRequest { email: test_email.clone() }).await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let created_user: CreateUserResponse = response.json();
    assert_eq!(created_user.user.email, test_email);
    assert_ne!(created_user.user.id, Uuid::nil());

    // Cleanup
    let pool = create_test_pool().await;
    cleanup_test_user(&pool, &test_email).await;
}

#[tokio::test]
async fn test_create_user_duplicate_email() {
    let server = create_test_server().await;
    let test_email = format!("test-duplicate-{}@example.com", Uuid::new_v4());

    // Create user first time
    let response1 =
        server.post("/api/v1/users").json(&CreateUserRequest { email: test_email.clone() }).await;

    assert_eq!(response1.status_code(), StatusCode::OK);

    // Try to create same user again
    let response2 =
        server.post("/api/v1/users").json(&CreateUserRequest { email: test_email.clone() }).await;

    assert_eq!(response2.status_code(), StatusCode::CONFLICT);

    // Cleanup
    let pool = create_test_pool().await;
    cleanup_test_user(&pool, &test_email).await;
}

#[tokio::test]
async fn test_create_user_invalid_email() {
    let server = create_test_server().await;

    // Test various invalid email formats
    let invalid_emails =
        vec!["not-an-email", "@example.com", "missing-at.com", "missing-domain@", ""];

    for invalid_email in invalid_emails {
        let response = server
            .post("/api/v1/users")
            .json(&CreateUserRequest { email: invalid_email.to_string() })
            .await;

        assert_eq!(
            response.status_code(),
            StatusCode::BAD_REQUEST,
            "Email '{}' should be invalid",
            invalid_email
        );
    }
}

#[tokio::test]
async fn test_create_user_missing_fields() {
    let server = create_test_server().await;

    // Test with empty JSON
    let response = server.post("/api/v1/users").json(&json!({})).await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_user_me_without_auth() {
    let server = create_test_server().await;

    // Try to access protected endpoint without authentication
    let response = server.get("/api/v1/users/me").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_user_me_with_invalid_token() {
    let server = create_test_server().await;

    // Try to access with invalid token
    let response = server
        .get("/api/v1/users/me")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_static("Bearer invalid-token"),
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_user_me_with_malformed_auth_header() {
    let server = create_test_server().await;

    // Test various malformed Authorization headers
    let malformed_headers = vec![
        "invalid-token",      // Missing "Bearer " prefix
        "Bearer",             // Missing token
        "Basic dXNlcjpwYXNz", // Wrong auth scheme
    ];

    for header_value in malformed_headers {
        let response = server
            .get("/api/v1/users/me")
            .add_header(
                axum::http::HeaderName::from_static("authorization"),
                axum::http::HeaderValue::from_str(header_value).unwrap(),
            )
            .await;

        assert_eq!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "Header '{}' should be rejected",
            header_value
        );
    }
}
