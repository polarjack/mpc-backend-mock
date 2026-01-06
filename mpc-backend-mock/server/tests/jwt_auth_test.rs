use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use eris_bitcoin_ext::WellKnownNetwork as BitcoinNetwork;
use eris_bitcoin_rpc_client::Authentication as BitcoinRpcAuthentication;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use mpc_backend_mock_server::entity::{CreateUserRequest, CreateUserResponse};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// JWT Claims structure matching the one in the middleware
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestClaims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub email_verified: Option<bool>,
}

/// Helper to create a test JWT token
fn create_test_jwt(keycloak_user_id: &str, email: &str) -> String {
    let claims = TestClaims {
        sub: keycloak_user_id.to_string(),
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 3600, // 1 hour from now
        email: Some(email.to_string()),
        preferred_username: Some(email.to_string()),
        email_verified: Some(true),
    };

    // Create a test JWT with insecure signing (for testing only)
    // NOTE: This uses the same insecure approach as the middleware's dev mode
    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret("test-secret".as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to create test JWT")
}

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

    // Create mock Bitcoin and ZPL clients
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
        admin_username: "admin".to_string(),
        admin_password: "admin".to_string(),
        verify_ssl: false,
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
        keycloak_config.realm,
    );

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
async fn test_jwt_validation_with_valid_token() {
    let server = create_test_server().await;
    let pool = create_test_pool().await;
    let test_email = format!("jwt-test-{}@example.com", Uuid::new_v4());

    // First create a user
    let create_response = server
        .post("/api/v1/users")
        .json(&CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123!".to_string(),
        })
        .await;

    assert_eq!(create_response.status_code(), StatusCode::OK);
    let created: CreateUserResponse = create_response.json();

    // Create a JWT token with the user's keycloak ID
    let jwt_token = create_test_jwt(&created.user.keycloak_user_id.to_string(), &test_email);

    // Access protected endpoint with valid token
    let response = server
        .get("/api/v1/users/me")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_str(&format!("Bearer {}", jwt_token)).unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // Cleanup
    cleanup_test_user(&pool, &test_email).await;
}

#[tokio::test]
async fn test_jwt_validation_with_expired_token() {
    let server = create_test_server().await;

    // Create an expired token (exp in the past)
    let claims = TestClaims {
        sub: Uuid::new_v4().to_string(),
        iat: chrono::Utc::now().timestamp() - 7200, // 2 hours ago
        exp: chrono::Utc::now().timestamp() - 3600, // 1 hour ago (expired)
        email: Some("test@example.com".to_string()),
        preferred_username: Some("test@example.com".to_string()),
        email_verified: Some(true),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret("test-secret".as_bytes());
    let expired_token = encode(&header, &claims, &encoding_key).unwrap();

    // Try to access with expired token
    let response = server
        .get("/api/v1/users/me")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_str(&format!("Bearer {}", expired_token)).unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_validation_with_malformed_token() {
    let server = create_test_server().await;

    // Test with various malformed tokens
    let malformed_tokens = vec![
        "not.a.jwt",   // Not a valid JWT structure
        "abc.def.ghi", // Invalid base64
        "eyJ.eyJ.sig", // Incomplete JWT
    ];

    for token in malformed_tokens {
        let response = server
            .get("/api/v1/users/me")
            .add_header(
                axum::http::HeaderName::from_static("authorization"),
                axum::http::HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            )
            .await;

        assert_eq!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "Token '{}' should be rejected",
            token
        );
    }
}

#[tokio::test]
async fn test_jwt_validation_with_missing_claims() {
    let server = create_test_server().await;

    // Create a token with missing required claims (no 'sub')
    #[derive(Serialize)]
    struct IncompleteClaims {
        email: String,
        exp: i64,
    }

    let claims = IncompleteClaims {
        email: "test@example.com".to_string(),
        exp: chrono::Utc::now().timestamp() + 3600,
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret("test-secret".as_bytes());
    let incomplete_token = encode(&header, &claims, &encoding_key).unwrap();

    let response = server
        .get("/api/v1/users/me")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_str(&format!("Bearer {}", incomplete_token)).unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_returns_user_info() {
    let server = create_test_server().await;
    let pool = create_test_pool().await;
    let test_email = format!("protected-test-{}@example.com", Uuid::new_v4());

    // Create a user
    let create_response = server
        .post("/api/v1/users")
        .json(&CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123!".to_string(),
        })
        .await;

    assert_eq!(create_response.status_code(), StatusCode::OK);
    let created: CreateUserResponse = create_response.json();

    // Create a valid JWT
    let jwt_token = create_test_jwt(&created.user.keycloak_user_id.to_string(), &test_email);

    // Access protected endpoint
    let response = server
        .get("/api/v1/users/me")
        .add_header(
            axum::http::HeaderName::from_static("authorization"),
            axum::http::HeaderValue::from_str(&format!("Bearer {}", jwt_token)).unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // Verify response contains user info
    let body = response.text();
    assert!(body.contains(&test_email), "Response should contain user email");
    assert!(body.contains(&created.user.id.to_string()), "Response should contain user ID");

    // Cleanup
    cleanup_test_user(&pool, &test_email).await;
}
