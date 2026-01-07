use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeus_axum::response::EncapsulatedJsonError;

use super::jwks::JwksClient;
use crate::web::ServiceState;

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID from Keycloak)
    pub sub: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Audience
    pub aud: Option<String>,
    /// Issuer
    pub iss: Option<String>,
    /// Email
    pub email: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// Email verified
    pub email_verified: Option<bool>,
}

/// Authenticated user information extracted from JWT
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID from Keycloak (subject claim)
    pub keycloak_user_id: Uuid,
    /// Email from token
    pub email: Option<String>,
    /// Username from token
    pub username: Option<String>,
    /// Whether email is verified
    pub email_verified: bool,
}

/// JWT authentication middleware
///
/// Validates JWT tokens from the Authorization header and extracts user claims
pub async fn jwt_auth_middleware(
    axum::extract::State(service_state): axum::extract::State<ServiceState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract token from Authorization header
    let token = extract_token_from_headers(&headers)?;

    tracing::debug!(
        "Authenticating JWT token using {:?} method",
        service_state.jwt_validation_method
    );

    // Route to appropriate validation method
    let claims = match service_state.jwt_validation_method {
        mpc_backend_mock_core::config::JwtValidationMethod::Jwks => {
            validate_token_jwks(token, &service_state.jwks_client).await?
        }
        mpc_backend_mock_core::config::JwtValidationMethod::Introspection => {
            validate_token_introspection(token, &service_state).await?
        }
    };

    tracing::info!("Token valid for user ID: {}", &claims.sub);

    // Parse Keycloak user ID from subject claim
    let keycloak_user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::InvalidToken("Invalid user ID format".to_string()))?;

    tracing::info!("Parsed Keycloak user ID: {}", &keycloak_user_id);

    // Create AuthUser from claims
    let auth_user = AuthUser {
        keycloak_user_id,
        email: claims.email,
        username: claims.preferred_username,
        email_verified: claims.email_verified.unwrap_or(false),
    };

    tracing::info!("auth_user created: {:?}", &auth_user);

    // Insert AuthUser into request extensions so it can be extracted by handlers
    drop(request.extensions_mut().insert(auth_user));

    Ok(next.run(request).await)
}

/// Extract bearer token from Authorization header
fn extract_token_from_headers(headers: &HeaderMap) -> Result<&str, AuthError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::InvalidToken("Invalid header encoding".to_string()))?;

    // Check for "Bearer " prefix
    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidToken("Missing Bearer prefix".to_string()));
    }

    // Extract token (skip "Bearer " prefix)
    Ok(&auth_header[7..])
}

/// Validate JWT token with JWKS-based signature verification
///
/// This implementation:
/// - Fetches the public key from Keycloak's JWKS endpoint
/// - Verifies the token signature with the public key
/// - Validates expiration and other standard claims
async fn validate_token_jwks(token: &str, jwks_client: &JwksClient) -> Result<Claims, AuthError> {
    tracing::info!("Validating JWT token: {}", token);

    // Decode header to get algorithm and key ID
    let header = decode_header(token)
        .map_err(|e| AuthError::InvalidToken(format!("Failed to decode header: {e}")))?;

    // Get the key ID from the token header
    let kid = header.kid.ok_or_else(|| {
        AuthError::InvalidToken("Token missing 'kid' (key ID) in header".to_string())
    })?;

    tracing::debug!("Token uses key ID: {}", kid);

    // Fetch the JWK for this key ID
    let jwk = jwks_client.get_jwk(&kid).await.map_err(|e| AuthError::JwksError(e.to_string()))?;

    // Convert JWK to DecodingKey
    let decoding_key = DecodingKey::from_jwk(&jwk)
        .map_err(|e| AuthError::InvalidToken(format!("Failed to parse JWK: {e}")))?;

    // Set up validation parameters
    let mut validation = Validation::new(header.alg);
    // Validate standard claims
    validation.validate_exp = true;
    validation.validate_nbf = false; // Not Before is optional
                                     // validation.validate_aud = false; // Allow any audience for development

    // Optionally validate issuer and audience
    // FIXME: should be configurable
    validation.set_issuer(&["http://localhost:8080/realms/mpc"]);
    validation.set_audience(&["account"]);

    // Decode and validate token with signature verification
    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|e| AuthError::InvalidToken(format!("Token validation failed: {e}")))?;

    tracing::debug!("Token successfully validated for subject: {}", token_data.claims.sub);

    Ok(token_data.claims)
}

/// Validate JWT token using Keycloak's token introspection endpoint
///
/// This implementation:
/// - Calls Keycloak's introspection endpoint to validate the token server-side
/// - Checks if the token is active
/// - Converts the introspection response to Claims structure
async fn validate_token_introspection(
    token: &str,
    service_state: &ServiceState,
) -> Result<Claims, AuthError> {
    tracing::info!("Validating JWT token via introspection");

    // Get the Keycloak client from service state
    let keycloak_client = service_state.keycloak_client.as_ref().ok_or_else(|| {
        AuthError::InvalidConfiguration(
            "Introspection validation requires KeycloakClient to be configured".to_string(),
        )
    })?;

    // Call introspection endpoint
    let introspection = keycloak_client
        .introspect_token(token)
        .await
        .map_err(|e| AuthError::IntrospectionError(format!("Token introspection failed: {e}")))?;

    tracing::debug!("Introspection response: active={}", introspection.active);

    // Check if token is active
    if !introspection.active {
        return Err(AuthError::InvalidToken("Token is not active".to_string()));
    }

    // Convert introspection response to Claims
    let claims = Claims {
        sub: introspection
            .sub
            .ok_or_else(|| AuthError::InvalidToken("Missing 'sub' claim in token".to_string()))?,
        iat: introspection.iat.unwrap_or(0),
        exp: introspection
            .exp
            .ok_or_else(|| AuthError::InvalidToken("Missing 'exp' claim in token".to_string()))?,
        aud: introspection.aud,
        iss: introspection.iss,
        email: None,
        preferred_username: introspection.username,
        email_verified: None,
    };

    tracing::debug!("Token successfully validated via introspection for subject: {}", claims.sub);

    Ok(claims)
}

/// Authentication errors
#[allow(dead_code)]
#[derive(Debug)]
pub enum AuthError {
    /// Missing authentication token
    MissingToken,
    /// Invalid or expired token
    InvalidToken(String),
    /// Insufficient permissions
    InsufficientPermissions,
    /// JWKS fetch error
    JwksError(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Token introspection error
    IntrospectionError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        use zeus_axum::{json_response, response};

        let (status, message) = match self {
            Self::MissingToken => {
                (StatusCode::UNAUTHORIZED, "Missing authentication token".to_string())
            }
            Self::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, format!("Invalid token: {msg}")),
            Self::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "Insufficient permissions".to_string())
            }
            Self::JwksError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Authentication service error: {msg}"))
            }
            Self::InvalidConfiguration(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Authentication configuration error: {msg}"),
            ),
            Self::IntrospectionError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Token introspection error: {msg}"))
            }
        };

        json_response! {
            status: status,
            error: response::Error {
                type_: response::ErrorType::Unauthorized,
                message,
                additional_fields: indexmap::IndexMap::default(),
            }
        }
    }
}
