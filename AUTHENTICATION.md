# Authentication & Authorization Flow

This document details the authentication and authorization mechanisms used in the MPC Backend Mock service.

## Overview

The service uses **Keycloak** as an external Identity Provider (IdP) with **JWT (JSON Web Token)** based authentication. All protected endpoints require a valid JWT Bearer token.

## Architecture Diagram

```
┌─────────────┐                ┌──────────────┐
│   Frontend  │                │   Keycloak   │
│             │                │      IdP     │
└─────┬───────┘                └──────┬───────┘
      │                               │
      │ 1. Login Request              │
      │ (username/password)           │
      │──────────────────────────────>│
      │                               │
      │ 2. JWT Access Token           │
      │<──────────────────────────────│
      │                               │
┌─────▼───────┐                       │
│   Store     │                       │
│   Token     │                       │
└─────────────┘                       │
      │                               │
      │ 3. API Request                │
      │ Authorization: Bearer <token> │
┌─────▼─────────────────────────────┐ │
│      MPC Backend Server           │ │
│ ┌───────────────────────────────┐ │ │
│ │   JWT Auth Middleware         │ │ │
│ │  - Extract Bearer token       │ │ │
│ │  - Validate JWT (exp, iat)    │ │ │
│ │  - Extract claims (sub, email)│ │ │
│ └──────────┬────────────────────┘ │ │
│            │ Valid?                │ │
│      ┌─────▼─────┐                 │ │
│      │   YES     │                 │ │
│      └─────┬─────┘                 │ │
│ ┌──────────▼──────────────────┐   │ │
│ │  Protected Route Handler    │   │ │
│ │  - Access AuthUser          │   │ │
│ │  - Fetch user from DB       │   │ │
│ │  - Execute business logic   │   │ │
│ └─────────────────────────────┘   │ │
└───────────────────────────────────┘ │
      │                               │
      │ 4. API Response               │
      │ (User data)                   │
      ▼                               │
   Frontend                           │
```

## Authentication Flow

### 1. User Registration (First Time)

```
POST /api/v1/users
Content-Type: application/json

{
  "email": "user@example.com"
}

Response:
{
  "data": {
    "user": {
      "id": "uuid",
      "email": "user@example.com",
      "keycloak_user_id": "00000000-0000-0000-0000-000000000000",
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  }
}
```

**Note**: Currently creates user with placeholder `keycloak_user_id`. In future, this should integrate with Keycloak's user creation API.

### 2. User Authentication with Keycloak

Request JWT token from Keycloak:

```bash
curl -X POST "http://localhost:8080/realms/mpc/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=mpc-backend" \
  -d "client_secret=<client-secret>" \
  -d "grant_type=password" \
  -d "username=test@example.com" \
  -d "password=test123"
```

Response:

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 300,
  "refresh_expires_in": 1800,
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "not-before-policy": 0,
  "session_state": "...",
  "scope": "profile email"
}
```

### 3. Accessing Protected Endpoints

Use the access token in the `Authorization` header:

```bash
curl -X GET "http://localhost:14444/api/v1/users/me" \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."
```

Response:

```json
{
  "data": {
    "user": {
      "id": "uuid",
      "email": "test@example.com",
      "keycloak_user_id": "keycloak-uuid",
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    },
    "username": "test@example.com",
    "email_verified": true
  }
}
```

### 4. Token Refresh

When the access token expires, use the refresh token:

```bash
curl -X POST "http://localhost:8080/realms/mpc/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=mpc-backend" \
  -d "client_secret=<client-secret>" \
  -d "grant_type=refresh_token" \
  -d "refresh_token=<refresh-token>"
```

## JWT Token Structure

### Header

```json
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "key-id"
}
```

### Payload (Claims)

```json
{
  "sub": "keycloak-user-uuid",
  "iat": 1640000000,
  "exp": 1640003600,
  "email": "user@example.com",
  "preferred_username": "user@example.com",
  "email_verified": true,
  "iss": "http://localhost:8080/realms/mpc",
  "aud": "mpc-backend"
}
```

### Required Claims

- `sub` (Subject): Keycloak user ID - used to link with database user
- `exp` (Expiration): Token expiration timestamp
- `iat` (Issued At): Token issue timestamp
- `email`: User's email address (optional but used for display)

## Middleware Implementation

### JWT Auth Middleware

Location: `mpc-backend-mock/server/src/web/middleware/auth.rs`

**Responsibilities:**

1. Extract JWT token from `Authorization` header
2. Validate token format (`Bearer <token>`)
3. Decode and validate JWT claims
4. Check token expiration
5. Create `AuthUser` object with validated claims
6. Inject `AuthUser` into request extensions
7. Return 401 Unauthorized on validation failure

**AuthUser Structure:**

```rust
pub struct AuthUser {
    pub keycloak_user_id: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub email_verified: Option<bool>,
}
```

### Route Protection

Protected routes use the `jwt_auth_middleware`:

```rust
let protected_routes = Router::new()
    .route("/v1/users/me", routing::get(user::get_current_user))
    .layer(middleware::from_fn(jwt_auth_middleware));
```

Handlers extract the authenticated user:

```rust
pub async fn get_current_user(
    State(service_state): State<ServiceState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<EncapsulatedJson<UserInfo>> {
    // Access auth_user.keycloak_user_id, auth_user.email, etc.
    let user = service_state
        .user_management_service
        .get_user_by_keycloak_id(&auth_user.keycloak_user_id)
        .await?;

    Ok(EncapsulatedJson::ok(UserInfo { user, ...}))
}
```

## Error Responses

### 401 Unauthorized - Missing Token

```json
{
  "error": {
    "type": "unauthorized",
    "message": "Missing authorization token"
  }
}
```

### 401 Unauthorized - Invalid Token

```json
{
  "error": {
    "type": "unauthorized",
    "message": "Invalid token"
  }
}
```

### 401 Unauthorized - Expired Token

```json
{
  "error": {
    "type": "unauthorized",
    "message": "Invalid token"
  }
}
```

## Security Considerations

### Current Implementation (Development)

⚠️ **WARNING**: The current JWT validation is **INSECURE** and only suitable for development:

- Does NOT validate JWT signature
- Does NOT verify token issuer
- Does NOT check audience claim
- Uses insecure decoding mode

Location: `mpc-backend-mock/server/src/web/middleware/auth.rs`

```rust
// INSECURE: No signature validation
let mut validation = Validation::new(Algorithm::RS256);
validation.insecure_disable_signature_validation();
```

### Production Requirements

For production deployment, implement proper JWT validation:

1. **Fetch JWKS from Keycloak:**

```bash
curl http://localhost:8080/realms/mpc/protocol/openid-connect/certs
```

2. **Validate Signature:** Use public key from JWKS to verify token signature

3. **Verify Issuer:** Ensure `iss` claim matches Keycloak URL

4. **Check Audience:** Validate `aud` claim contains `mpc-backend`

5. **Implement Token Caching:** Cache JWKS keys with TTL

6. **Add Rate Limiting:** Prevent brute force attacks on protected endpoints

7. **Enable CORS:** Configure allowed origins for frontend

8. **Use HTTPS:** Enable TLS for all communications in production

### Example Production JWT Validation

```rust
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use jwks_client::JwksClient;

// Fetch JWKS from Keycloak
let jwks_url = format!("{}/realms/{}/protocol/openid-connect/certs",
    keycloak_url, realm);
let jwks_client = JwksClient::new(jwks_url);

// Get key ID from token header
let header = decode_header(&token)?;
let kid = header.kid.ok_or("Missing kid")?;

// Fetch public key
let jwk = jwks_client.get(&kid).await?;
let decoding_key = DecodingKey::from_jwk(&jwk)?;

// Validate token
let mut validation = Validation::new(Algorithm::RS256);
validation.set_audience(&["mpc-backend"]);
validation.set_issuer(&[issuer_url]);

let token_data = decode::<Claims>(&token, &decoding_key, &validation)?;
```

## Testing

### Integration Tests

Location: `mpc-backend-mock/server/tests/`

**Test Files:**

- `user_api_test.rs` - Tests user creation and unauthenticated access attempts
- `jwt_auth_test.rs` - Tests JWT validation, expiration, malformed tokens, and authenticated access

**Running Tests:**

```bash
# Run all integration tests
DATABASE_URL=postgres://postgres:postgres@localhost:5432/mpc_backend_mock \
  cargo test -p mpc-backend-mock-server

# Run specific test file
cargo test -p mpc-backend-mock-server --test jwt_auth_test

# Run with verbose output
cargo test -p mpc-backend-mock-server -- --nocapture
```

### Manual Testing with curl

```bash
# 1. Get JWT token
TOKEN=$(curl -s -X POST "http://localhost:8080/realms/mpc/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=mpc-backend" \
  -d "grant_type=password" \
  -d "username=test@example.com" \
  -d "password=test123" \
  | jq -r '.access_token')

# 2. Test protected endpoint
curl -X GET "http://localhost:14444/api/v1/users/me" \
  -H "Authorization: Bearer $TOKEN" \
  | jq

# 3. Test without token (should fail with 401)
curl -X GET "http://localhost:14444/api/v1/users/me"

# 4. Test with invalid token (should fail with 401)
curl -X GET "http://localhost:14444/api/v1/users/me" \
  -H "Authorization: Bearer invalid-token"
```

## Future Enhancements

### 1. Role-Based Access Control (RBAC)

Add role checking in middleware:

```rust
pub struct AuthUser {
    pub keycloak_user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,  // Extract from JWT claims
}

// Check roles in handlers
if !auth_user.roles.contains("admin") {
    return Err(Error::InsufficientPermissions);
}
```

### 2. Permission-Based Authorization

Define permissions per endpoint:

```rust
#[derive(Debug, Clone)]
pub enum Permission {
    ReadUser,
    WriteUser,
    DeleteUser,
    AdminAccess,
}

// Middleware checks permissions
async fn require_permission(
    permission: Permission,
) -> impl Fn(Request, Next) -> Future<Response> {
    // Check if user has permission
}
```

### 3. API Key Authentication

Add support for service-to-service authentication:

```rust
// Alternative auth for service accounts
async fn api_key_middleware(/* ... */) {
    // Extract X-API-Key header
    // Validate against stored API keys
}
```

### 4. OAuth2 Scopes

Implement fine-grained access control with OAuth2 scopes:

```rust
// Required scopes per endpoint
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    security(("bearer_auth" = ["read:user", "profile"]))
)]
```

### 5. Session Management

Add session tracking for audit logs:

```rust
// Track active sessions
pub struct UserSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: IpAddr,
}
```

## Troubleshooting

### Common Issues

**1. "Missing authorization token"**

- Ensure `Authorization` header is present
- Verify header format: `Bearer <token>`

**2. "Invalid token"**

- Check token expiration (`exp` claim)
- Verify token was issued by correct Keycloak realm
- Ensure token hasn't been revoked

**3. "User not found"**

- Verify user exists in database with matching `keycloak_user_id`
- Check if user has been soft-deleted (`deleted_at IS NOT NULL`)

**4. Token expired immediately**

- Check system clock synchronization between server and Keycloak
- Verify Keycloak token lifetime settings

### Debug Mode

Enable debug logging for authentication:

```bash
RUST_LOG=mpc_backend_mock_server::web::middleware::auth=debug cargo run
```

This will log:

- Extracted JWT tokens (redacted)
- Token validation results
- Extracted claims
- Authentication errors

## References

- [JWT.io](https://jwt.io/) - JWT token decoder and debugger
- [Keycloak Documentation](https://www.keycloak.org/documentation)
- [OAuth 2.0 Spec](https://oauth.net/2/)
- [OpenID Connect](https://openid.net/connect/)
- [RFC 7519 - JWT](https://datatracker.ietf.org/doc/html/rfc7519)
