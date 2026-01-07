# JWT Token Validation Enhancement Plan

## Objective

Add support for JWT token validation via Keycloak's introspection endpoint as an alternative to the existing JWKS-based local validation. This allows users to choose between:

1. **JWKS-based validation** (current): Fast, local validation using public keys
2. **Introspection-based validation** (new): Server-side validation with real-time token status

## Current Architecture

### Existing JWKS-Based Validation

**Location**: [`mpc-backend-mock/server/src/web/middleware/auth.rs`](mpc-backend-mock/server/src/web/middleware/auth.rs)

**Flow**:
1. Extract JWT token from `Authorization: Bearer <token>` header
2. Decode JWT header to get key ID (`kid`)
3. Fetch corresponding JWK from JWKS client (cached for 5 minutes)
4. Verify token signature using RSA256 public key
5. Validate claims: `exp`, `iat`, `iss`, `aud`
6. Extract user information and inject `AuthUser` into request extensions

**Pros**:
- Fast (local validation, no network call per request)
- Reduced load on Keycloak server
- Works offline once JWKS is cached

**Cons**:
- Cannot detect tokens that have been revoked server-side
- 5-minute cache means revocations have delay
- No real-time token status

### Existing Introspection Implementation

**Location**: [`mpc-backend-mock/bin/src/keycloak_client/mod.rs`](mpc-backend-mock/bin/src/keycloak_client/mod.rs#L229-L328)

**Function**: `async fn introspect_token(&self, token: &str) -> Result<TokenIntrospectionResponse>`

**Flow**:
1. Obtain admin access token via password grant (`admin-cli` client)
2. Call Keycloak's RFC 7662 introspection endpoint
3. Return token metadata including `active` status, `sub`, `exp`, `username`, etc.

**Pros**:
- Real-time validation (detects revoked tokens immediately)
- Authoritative status from Keycloak server
- More secure for sensitive operations

**Cons**:
- Slower (network call to Keycloak per request)
- Higher load on Keycloak server
- Requires Keycloak to be reachable

## Implementation Plan

### 1. Configuration Enhancement

**File**: [`mpc-backend-mock/bin/src/config/keycloak.rs`](mpc-backend-mock/bin/src/config/keycloak.rs)

Add a new configuration field to choose validation method:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum JwtValidationMethod {
    /// Local JWT validation using JWKS (faster, cached)
    #[serde(rename = "jwks")]
    Jwks,
    /// Server-side token introspection (real-time, authoritative)
    #[serde(rename = "introspection")]
    Introspection,
}

impl Default for JwtValidationMethod {
    fn default() -> Self {
        Self::Jwks // Keep existing behavior as default
    }
}

// Add to KeycloakConfig:
pub struct KeycloakConfig {
    // ... existing fields ...

    /// JWT validation method
    #[serde(default)]
    pub jwt_validation_method: JwtValidationMethod,
}
```

**Configuration example** (`config.yaml`):

```yaml
keycloak:
  server_url: "http://localhost:8080"
  realm: "mpc"
  jwt_validation_method: "jwks"  # or "introspection"
```

### 2. ServiceState Enhancement

**File**: [`mpc-backend-mock/server/src/web/mod.rs`](mpc-backend-mock/server/src/web/mod.rs#L79-L84)

Add `KeycloakClient` to `ServiceState`:

```rust
#[derive(Clone)]
pub struct ServiceState {
    pub bitcoin_rpc_client: BitcoinRpcClient,
    pub zpl_rpc_client: ZplRpcClient,
    pub user_management_service: UserManagementService,
    pub jwks_client: middleware::JwksClient,
    pub keycloak_client: Option<Arc<mpc_backend_mock::keycloak_client::KeycloakClient>>, // NEW
    pub jwt_validation_method: JwtValidationMethod, // NEW
}
```

**Rationale for `Option<Arc<KeycloakClient>>`**:
- `Option` because introspection client is only needed when `jwt_validation_method = Introspection`
- `Arc` for shared ownership (already created at application startup for `UserManagementService`)

### 3. Middleware Enhancement

**File**: [`mpc-backend-mock/server/src/web/middleware/auth.rs`](mpc-backend-mock/server/src/web/middleware/auth.rs)

#### 3.1 Update Middleware Signature

```rust
pub async fn jwt_auth_middleware(
    axum::extract::State(service_state): axum::extract::State<ServiceState>, // CHANGED
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_token_from_headers(&headers)?;

    tracing::debug!("Authenticating JWT token using {:?} method", service_state.jwt_validation_method);

    // Route to appropriate validation method
    let claims = match service_state.jwt_validation_method {
        JwtValidationMethod::Jwks => {
            validate_token_jwks(token, &service_state.jwks_client).await?
        }
        JwtValidationMethod::Introspection => {
            validate_token_introspection(token, &service_state).await?
        }
    };

    // ... rest of the middleware (parse UUID, create AuthUser, inject into request)
}
```

#### 3.2 Rename Existing Validation Function

```rust
// Rename: validate_token -> validate_token_jwks
async fn validate_token_jwks(token: &str, jwks_client: &JwksClient) -> Result<Claims, AuthError> {
    // ... existing JWKS validation logic ...
}
```

#### 3.3 Add New Introspection Validation Function

```rust
async fn validate_token_introspection(
    token: &str,
    service_state: &ServiceState,
) -> Result<Claims, AuthError> {
    let keycloak_client = service_state
        .keycloak_client
        .as_ref()
        .ok_or_else(|| AuthError::InvalidConfiguration(
            "Introspection validation requires KeycloakClient".to_string()
        ))?;

    // Call introspection endpoint
    let introspection = keycloak_client
        .introspect_token(token)
        .await
        .map_err(|e| AuthError::IntrospectionError(e.to_string()))?;

    // Check if token is active
    if !introspection.active {
        return Err(AuthError::InvalidToken("Token is not active".to_string()));
    }

    // Convert introspection response to Claims
    Ok(Claims {
        sub: introspection.sub.ok_or_else(||
            AuthError::InvalidToken("Missing 'sub' claim".to_string()))?,
        iat: introspection.iat.unwrap_or(0),
        exp: introspection.exp.ok_or_else(||
            AuthError::InvalidToken("Missing 'exp' claim".to_string()))?,
        aud: introspection.aud,
        iss: introspection.iss,
        email: None, // Not provided by standard introspection
        preferred_username: introspection.username,
        email_verified: None, // Not provided by standard introspection
    })
}
```

#### 3.4 Add New Error Variants

```rust
pub enum AuthError {
    // ... existing variants ...

    /// Invalid configuration
    InvalidConfiguration(String),

    /// Token introspection error
    IntrospectionError(String),
}
```

### 4. Server Initialization

**File**: [`mpc-backend-mock/server/src/lib.rs`](mpc-backend-mock/server/src/lib.rs)

Update the server initialization to:
1. Create `KeycloakClient` instance (can reuse existing from `UserManagementService`)
2. Pass it to `ServiceState`
3. Pass validation method from config

```rust
// Around line 260-305 where KeycloakAdmin is initialized
let keycloak_client = mpc_backend_mock::keycloak_client::KeycloakClient::new(config.keycloak.clone())
    .await
    .map_err(|err| Error::InitializeKeycloakClient {
        message: format!("Failed to initialize Keycloak client: {err}"),
    })?;

let keycloak_client = Arc::new(keycloak_client);

// Update ServiceState construction
let service_state = ServiceState {
    // ... existing fields ...
    keycloak_client: match config.keycloak.jwt_validation_method {
        JwtValidationMethod::Introspection => Some(keycloak_client.clone()),
        JwtValidationMethod::Jwks => None,
    },
    jwt_validation_method: config.keycloak.jwt_validation_method.clone(),
};
```

### 5. Router Configuration

**File**: [`mpc-backend-mock/server/src/web/mod.rs`](mpc-backend-mock/server/src/web/mod.rs)

Update the router to pass `ServiceState` instead of just `JwksClient` to the auth middleware:

```rust
// Change from:
.route_layer(middleware::from_fn_with_state(
    service_state.jwks_client.clone(),
    middleware::jwt_auth_middleware,
))

// To:
.route_layer(middleware::from_fn_with_state(
    service_state.clone(),
    middleware::jwt_auth_middleware,
))
```

### 6. Testing

**File**: [`mpc-backend-mock/server/tests/jwt_auth_test.rs`](mpc-backend-mock/server/tests/jwt_auth_test.rs) (new or extend existing)

Add integration tests for introspection-based validation:

```rust
#[tokio::test]
async fn test_introspection_validation_valid_token() {
    // Test that valid tokens are accepted via introspection
}

#[tokio::test]
async fn test_introspection_validation_revoked_token() {
    // Test that revoked tokens are rejected via introspection
}

#[tokio::test]
async fn test_introspection_validation_expired_token() {
    // Test that expired tokens are rejected via introspection
}

#[tokio::test]
async fn test_configuration_switch() {
    // Test switching between JWKS and introspection methods
}
```

## Migration Path

### For Users

**No breaking changes** - existing users will continue using JWKS validation by default.

To opt into introspection validation, add to `config.yaml`:

```yaml
keycloak:
  jwt_validation_method: "introspection"
```

### Choosing the Right Method

**Use JWKS when**:
- High request volume (lower latency required)
- Token revocation is not critical
- Keycloak server load is a concern

**Use Introspection when**:
- Real-time token revocation is required
- Security is paramount (e.g., financial operations)
- Request volume is moderate
- You need authoritative token status

## Files to Modify

1. ✅ [`plan.md`](plan.md) - This plan document
2. [ ] [`mpc-backend-mock/bin/src/config/keycloak.rs`](mpc-backend-mock/bin/src/config/keycloak.rs) - Add `JwtValidationMethod` enum and config field
3. [ ] [`mpc-backend-mock/server/src/web/mod.rs`](mpc-backend-mock/server/src/web/mod.rs) - Update `ServiceState` and router
4. [ ] [`mpc-backend-mock/server/src/web/middleware/auth.rs`](mpc-backend-mock/server/src/web/middleware/auth.rs) - Implement dual validation
5. [ ] [`mpc-backend-mock/server/src/lib.rs`](mpc-backend-mock/server/src/lib.rs) - Update server initialization
6. [ ] [`mpc-backend-mock/server/tests/`](mpc-backend-mock/server/tests/) - Add integration tests
7. [ ] [`CLAUDE.md`](CLAUDE.md) - Update documentation

## Success Criteria

- [x] Existing JWKS validation continues to work
- [ ] New introspection validation can be enabled via configuration
- [ ] Both methods produce compatible `AuthUser` objects
- [ ] Tests verify both validation methods
- [ ] Documentation updated with guidance on choosing methods
- [ ] Zero breaking changes for existing users
