# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an MPC (Multi-Party Computation) backend mock service written in Rust, designed to integrate with an MPC frontend. It's a blockchain-focused web service that interacts with Bitcoin and Solana networks.

**Key Technologies:**

- Rust 1.83+ (workspace with 6 crates)
- PostgreSQL 17.4 with sqlx
- Axum (web framework)
- gRPC with Tonic
- Blockchain: Bitcoin RPC client, Solana SDK, ZPL (Zeus Program Library) RPC client
- Nix flake for development environment

## Common Commands

### Building & Running

```bash
# Build the project
cargo build --bin mpc-backend-mock

# Run the server (requires config.yaml)
cargo run -p mpc-backend-mock -- --config config.yaml run

# Alternative run command (uses convenience script)
./run.sh
```

### Testing & Quality

```bash
# Run all tests
cargo test --workspace

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test --workspace

# Run linter (clippy is very strict - see workspace lints)
cargo clippy --workspace --all-targets

# Format code
cargo fmt --all

# Format all code types (Rust, SQL, TOML, etc.)
nix fmt  # or: treefmt
```

### Database Migrations

```bash
# Run migrations
cargo sqlx migrate run --ignore-missing

# Revert last migration
cargo sqlx migrate revert --ignore-missing

# Reset database (Docker Compose)
./reset.sh
```

### Keycloak Setup (Authentication)

```bash
# Start Keycloak with Docker Compose (standalone stack)
cd dev-support/test-environments/docker-compose
docker compose -f docker-compose.keycloak.yml up -d

# Run automated setup script (creates realm, client, and test user)
./setup-keycloak.sh

# Or manually configure via Keycloak Admin Console at http://localhost:8080
# Default credentials: admin / admin
```

**Keycloak Configuration:**

- Realm: `mpc`
- Client ID: `mpc-backend`
- Test User: `test@example.com` / `test123`
- JWT-based authentication with Bearer tokens

For detailed setup instructions, see: `dev-support/test-environments/docker-compose/KEYCLOAK_SETUP.md`

### Development Tools

```bash
# Generate OpenAPI documentation
cargo run -p mpc-backend-mock -- openapi

# Generate shell completions
cargo run -p mpc-backend-mock -- completion bash

# Output default configuration
cargo run -p mpc-backend-mock -- default-config

# Show version info
cargo run -p mpc-backend-mock -- version
```

### Nix Development

```bash
# Enter development shell
nix develop

# Run checks (build, format, clippy, tests)
nix flake check
```

## Architecture

### Workspace Structure

The project is organized as a Cargo workspace with 6 crates:

**Core crates (mpc-backend-mock/):**

- `mpc-backend-mock/bin` - CLI application entry point with config management and KMS client
- `mpc-backend-mock/core` - Core types, models, and configuration structures
- `mpc-backend-mock/server` - Server implementation with web API, gRPC, and services

**Shared utility crates (crates/):**

- `crates/cli-common` - Common CLI functionality (logging config)
- `crates/protobuf-types` - Protocol buffer definitions (health check service)
- `crates/metrics` - Prometheus metrics server
- `crates/web` - Axum web utilities (response types, error handling)

### Server Architecture

The server runs three concurrent services managed by `LifecycleManager`:

1. **HTTP API Server** (default port 14444)
   - Axum-based REST API with OpenAPI documentation
   - Controllers in `mpc-backend-mock/server/src/web/controller/`
   - Middleware: tracing, compression, path normalization
   - Service layer pattern with `ServiceState` dependency injection

2. **gRPC Health Check Server** (default port 14447)
   - Tonic-based gRPC server
   - Checks Bitcoin RPC client and PostgreSQL connectivity
   - Implementation in `mpc-backend-mock/server/src/grpc/`

3. **Metrics Server** (default port 14446, optional)
   - Prometheus metrics endpoint
   - Enabled via `metrics.enable` in config

### Configuration System

Configuration is loaded in this order:

1. Default path: `~/.config/olympus-backend/olympus-backend.yaml`
2. Environment variable: `OLYMPUS_BACKEND_CONFIG_FILE_PATH`
3. CLI flag: `--config` / `-c`

Key configuration modules:

- `BitcoinConfig` - Bitcoin RPC endpoint and network settings
- `SolanaConfig` - Solana RPC endpoint and cluster configuration
- `PostgresConfig` - Database connection with sqlx
- `WebConfig` - HTTP server listen address
- `KeycloakConfig` - Keycloak server URL, realm, client credentials for JWT authentication
- `KeyManagementService` - Optional GCP KMS integration

### Service Layer

Services are located in `mpc-backend-mock/server/src/service/`:

- `UserManagementService` - User-related operations (user creation, retrieval)
  - Initialized with `Arc<KeycloakAdmin>` client for Keycloak operations
  - Keycloak admin client is created once at application startup (see `initialize_keycloak_admin()` in `lib.rs`)
  - Uses dependency injection pattern - receives pre-initialized clients rather than creating them on-demand
- Services use `PgPool` for database access via sqlx
- SQL queries in `mpc-backend-mock/server/sql/` with readme for conventions

### Blockchain Integration

The server integrates with blockchain networks:

- **Bitcoin**: Uses `eris-bitcoin-rpc-client` from ZeusNetworkHQ/eris (custom fork)
- **Solana**: Uses official `solana-client` and SPL token libraries
- **ZPL**: Custom Zeus Program Library RPC client for two-way peg operations
- Block confirmations: Bitcoin network uses configurable confirmation depth

### Database

- PostgreSQL with sqlx for compile-time query checking
- Migrations in `mpc-backend-mock/server/migrations/`
- Uses UUID extension (`uuid-ossp`)
- Connection pooling with configurable pool size
- Optional role-based access with `SET SESSION ROLE`
- User table with soft-delete support (`deleted_at` column)

### SQL Executor Pattern

The codebase uses a **SQL Executor trait pattern** to separate database operations from business logic:

**Pattern Structure:**

1. **SQL Files** (`mpc-backend-mock/server/sql/*/`): Raw SQL queries stored in separate `.sql` files
2. **SQL Executor Traits** (`mpc-backend-mock/server/src/service/sql_executor/`): Async traits defining database operations
3. **Generic Implementation**: Traits are implemented for any type that implements `Executor<'c, Database = Postgres>`
4. **Service Layer**: Services use the executor traits for all database operations

**Example** ([`sql_executor/user.rs`](mpc-backend-mock/server/src/service/sql_executor/user.rs)):

```rust
#[async_trait]
pub trait UserSqlExecutor {
    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>>;
    async fn insert_user(&mut self, email: &str, keycloak_user_id: &Uuid, is_active: bool) -> Result<User>;
    // ... other methods
}

#[async_trait]
impl<E> UserSqlExecutor for E
where
    for<'c> &'c mut E: Executor<'c, Database = Postgres>,
{
    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>> {
        sqlx::query_file_as!(User, "sql/user/get_user_by_email.sql", email)
            .fetch_optional(&mut *self)
            .await
            .context(error::GetUserByEmailSnafu)
    }
}
```

**Benefits:**

- **Separation of Concerns**: SQL queries isolated from business logic
- **Reusability**: Same executor works with `PgPool`, `Transaction`, or `Connection`
- **Type Safety**: `sqlx::query_file_as!` provides compile-time SQL validation
- **Consistency**: All database operations follow the same pattern
- **Testability**: Easy to mock executors for unit testing

**When Adding New Database Operations:**

1. Create SQL file in `mpc-backend-mock/server/sql/<module>/query_name.sql`
2. Add method to appropriate executor trait (or create new one)
3. Implement using `sqlx::query_file_as!` with proper error context
4. Use the executor method in service layer (acquire connection if needed)

### Authentication & Authorization

The server uses **Keycloak** for authentication with JWT-based token validation using JWKS (JSON Web Key Set) for cryptographic signature verification.

**Architecture:**

- Public endpoints: No authentication required (e.g., `POST /api/v1/users`, `GET /api/v1/info`)
- Protected endpoints: Require valid JWT Bearer token (e.g., `GET /api/v1/users/me`)
- JWT middleware: Validates token signature using JWKS, checks expiration, and extracts user claims
- User linking: Database users are linked to Keycloak users via `keycloak_user_id`
- **Keycloak Admin Client**: Initialized once at application startup in [`lib.rs:259-287`](mpc-backend-mock/server/src/lib.rs#L259-L287)
  - Wrapped in `Arc<KeycloakAdmin>` for shared ownership across services
  - Used by `UserManagementService` for user creation and management operations
  - Avoids redundant authentication calls by reusing a single authenticated client

**Keycloak Clients:**

1. **`mpc-backend-service`** (confidential) - Backend service account for admin operations
   - Service account with `manage-users`, `view-users`, `query-users` roles
   - Used for programmatic user creation and management via Keycloak Admin API
   - Credentials configured in `KeycloakConfig` (`admin_username`, `admin_password`)
2. **`mpc-frontend`** (public) - Frontend SPA client with PKCE
   - OAuth2.0 Authorization Code Flow with PKCE (S256)
   - Redirect URIs: http://localhost:3000/_, http://localhost:5173/_ (Vite)
   - Direct access grants enabled for development/testing

**JWT Validation Flow** (`mpc-backend-mock/server/src/web/middleware/`):

1. **JWKS Service** ([`jwks.rs`](mpc-backend-mock/server/src/web/middleware/jwks.rs)):
   - Fetches public keys from Keycloak's JWKS endpoint (`/realms/mpc/protocol/openid-connect/certs`)
   - Caches keys for 5 minutes to reduce Keycloak load
   - Handles key rotation automatically
   - Uses `jsonwebtoken` crate for RSA256 signature verification

2. **Auth Middleware** ([`auth.rs`](mpc-backend-mock/server/src/web/middleware/auth.rs)):
   - Extracts Bearer token from `Authorization` header
   - Decodes JWT header to get key ID (`kid`)
   - Fetches corresponding JWK from JWKS service
   - Verifies token signature with public key (RSA256)
   - Validates standard claims: `exp`, `iat`, `iss`, `aud`
   - Extracts user claims: `sub`, `email`, `preferred_username`, `email_verified`
   - Creates `AuthUser` object and injects into request extensions
   - Returns 401 Unauthorized for invalid/missing/expired tokens

**OAuth2.0 Frontend Flow (PKCE):**

1. Frontend generates code verifier and SHA-256 challenge
2. Redirects user to Keycloak authorization endpoint with PKCE parameters
3. User authenticates with Keycloak
4. Keycloak redirects back with authorization code
5. Frontend exchanges code + verifier for access token
6. Frontend includes token in API requests: `Authorization: Bearer <token>`
7. Backend validates token using JWKS and processes request

**Configuration** ([`keycloak.rs`](mpc-backend-mock/bin/src/config/keycloak.rs)):

- `server_url`: Keycloak base URL (e.g., "http://localhost:8080")
- `realm`: Keycloak realm name (e.g., "mpc")
- `client_id`: Backend service client ID
- `client_secret`: Backend service client secret
- JWKS URL is auto-constructed: `{server_url}/realms/{realm}/protocol/openid-connect/certs`

**Token Introspection:**

The `KeycloakClient` in `mpc-backend-mock/bin/src/keycloak_client/mod.rs` provides token introspection functionality to validate JWT tokens and retrieve their metadata:

```rust
use mpc_backend_mock::keycloak_client::{KeycloakClient, TokenIntrospectionResponse};

let client = KeycloakClient::new(keycloak_config).await?;
let response: TokenIntrospectionResponse = client.introspect_token("eyJhbGciOiJSUzI1NiIsInR5cCI...").await?;

if response.active {
    println!("Token is valid");
    println!("Username: {:?}", response.username);
    println!("Subject (user ID): {:?}", response.sub);
    println!("Expires at: {:?}", response.exp);
    println!("Issued at: {:?}", response.iat);
} else {
    println!("Token is invalid or expired");
}
```

The introspection endpoint validates tokens server-side by calling Keycloak's RFC 7662 token introspection endpoint:
- Endpoint: `POST /realms/{realm}/protocol/openid-connect/token/introspect`
- Authentication: Uses admin credentials via password grant flow
- Returns: Token metadata including active status, subject, expiration, claims

**Testing:**

- Integration tests in `mpc-backend-mock/server/tests/`
- `user_api_test.rs` - Tests user creation and unauthenticated access
- `jwt_auth_test.rs` - Tests JWT validation, expiration, malformed tokens, and protected endpoints
- Test with real Keycloak tokens or use the setup script to create test users

## Code Standards

### Linting

This project uses **extremely strict linting** (see `Cargo.toml` workspace lints):

- Almost all Clippy lints set to `deny` (all, cargo, nursery, pedantic)
- Most Rust lints set to `deny` (including unused code, unsafe code)
- `unsafe_code` is forbidden
- Dead code, unused imports, and unused variables are errors

**Important exceptions:**

- `async_fn_in_trait` is allowed
- `future_not_send`, `module_name_repetitions`, `multiple_crate_versions` allowed in Clippy

When adding code, expect strict enforcement. Run `cargo clippy` frequently.

### Code Organization

- Use `snafu` for error handling (not `thiserror` or manual impls)
- Error types in dedicated `error.rs` modules with context
- All public API types should derive `utoipa::ToSchema` for OpenAPI
- Use `tracing` for logging (not `log` or `println!`)
- Async runtime: Tokio with multi-threaded executor

### Formatting

Multiple formatters are configured in `treefmt.toml`:

- Rust: `cargo fmt` (rustfmt)
- SQL: `sleek` formatter (uppercase keywords, 4 spaces)
- TOML: `taplo`
- Proto: `clang-format`
- Shell: `shfmt` with `shellcheck`
- JSON/YAML/Markdown: `prettier`
- Nix: `nixfmt`

Run `nix fmt` or `treefmt` to format all files.

## Development Workflow

1. **Setup**: Run `nix develop` to enter development environment with all tools
2. **Database**: Use `./reset.sh` to reset PostgreSQL (Docker Compose)
3. **Migrations**: Run `cargo sqlx migrate run` before starting server
4. **Config**: Copy/modify `config.yaml` with your RPC endpoints and database credentials
5. **Run**: Use `./run.sh` or `cargo run -p mpc-backend-mock -- --config config.yaml run`
6. **Test**: Verify with health check endpoint (gRPC) and API endpoints (HTTP)

### Docker Compose

Development database setup in `dev-support/test-environments/docker-compose/`:

- `docker-compose.pgonly.yml` - PostgreSQL only
- `docker-compose.yaml` - Full environment (Bitcoin, Electrs, PostgreSQL)
- Electrs configuration in `electrs-config.toml`

## Project-Specific Notes

- The program name is `olympus-backend` in core constants (mismatch with repo name `mpc-backend-mock`)
- Uses shadow-rs for build metadata (version, commit hash, branch) embedded at compile time
- KMS integration is optional and currently supports Google Cloud KMS
- Bitcoin network defaults to mainnet; Solana defaults to mainnet (configurable)
- The server uses QuickNode endpoints in the example config (requires credentials)
- Private Git dependencies require SSH access to `github.com/ZeusNetworkHQ` repos
