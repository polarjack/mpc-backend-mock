# MPC Backend Mock

A blockchain-focused Multi-Party Computation (MPC) backend service written in Rust, designed to integrate with an MPC frontend. The service provides user management with Keycloak authentication and interacts with Bitcoin and Solana networks.

## Features

- **User Management**: Create and manage users with Keycloak integration
- **JWT Authentication**: Secure endpoints with JWT token validation
- **Blockchain Integration**: Bitcoin and Solana RPC client support
- **gRPC Health Checks**: Service health monitoring
- **OpenAPI Documentation**: Auto-generated API docs
- **PostgreSQL Database**: Persistent storage with sqlx
- **Prometheus Metrics**: Optional metrics endpoint

## Quick Start

### Prerequisites

- Rust 1.83+
- PostgreSQL 17.4+
- Docker & Docker Compose (for Keycloak)
- Nix (optional, for development environment)

### 1. Database Setup

```bash
# Start PostgreSQL with Docker Compose
cd dev-support/test-environments/docker-compose
docker compose -f docker-compose.pgonly.yml up -d

# Run migrations
cargo sqlx migrate run --ignore-missing
```

### 2. Keycloak Setup

```bash
# Start Keycloak
cd dev-support/test-environments/docker-compose
docker compose -f docker-compose.keycloak.yml up -d

# Run automated setup (creates realm, client, and test user)
./setup-keycloak.sh
```

**Keycloak Admin Console**: [http://localhost:8080](http://localhost:8080)

- Username: `admin`
- Password: `admin`

**Test User Credentials**:

- Email: `test@example.com`
- Password: `test123`

For manual configuration, see: `dev-support/test-environments/docker-compose/KEYCLOAK_SETUP.md`

### 3. Configuration

Create a `config.yaml` file:

```yaml
web:
  listen_address: "127.0.0.1:14444"

postgres:
  host: "localhost"
  port: 5432
  database: "mpc_backend_mock"
  username: "postgres"
  password: "postgres"
  ssl_mode: "prefer"
  max_connections: 10

keycloak:
  server_url: "http://localhost:8080"
  realm: "mpc"
  client_id: "mpc-backend"
  client_secret: "your-client-secret-here" # Get from Keycloak Admin Console
  admin_username: "admin"
  admin_password: "admin"
  verify_ssl: false # Set to true in production

bitcoin:
  network: "regtest"
  rpc_endpoint: "http://localhost:18443"
  rpc_authentication: "user:password"
  indexer_endpoint: "http://localhost:50001"
  support_quicknode_blockbook: false

solana:
  endpoint:
    url: "http://localhost:8899"
    cluster: "devnet"

metrics:
  enable: true
  listen_address: "127.0.0.1:14446"

health_check:
  listen_address: "127.0.0.1:14447"
```

### 4. Run the Server

```bash
# Build and run
cargo run -p mpc-backend-mock -- --config config.yaml run

# Or use the convenience script
./run.sh
```

The API will be available at: [http://localhost:14444](http://localhost:14444)

## API Endpoints

### Public Endpoints (No Authentication)

#### Create User

```bash
POST /api/v1/users
Content-Type: application/json

{
  "email": "user@example.com"
}
```

#### Server Info

```bash
GET /api/v1/info
```

### Protected Endpoints (Requires Authentication)

#### Get Current User

```bash
GET /api/v1/users/me
Authorization: Bearer <jwt-token>
```

## Authentication Flow

1. **Obtain JWT Token from Keycloak:**

```bash
curl -X POST "http://localhost:8080/realms/mpc/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=mpc-backend" \
  -d "grant_type=password" \
  -d "username=test@example.com" \
  -d "password=test123"
```

2. **Use Token in API Requests:**

```bash
TOKEN="<your-access-token>"

curl -X GET "http://localhost:14444/api/v1/users/me" \
  -H "Authorization: Bearer $TOKEN"
```

## OpenAPI Documentation

Generate and view the OpenAPI specification:

```bash
# Generate openapi.json
cargo run -p mpc-backend-mock -- openapi > openapi.json

# Or access it at runtime
curl http://localhost:14444/openapi.json
```

## Development

### Architecture Patterns

The codebase follows several key architectural patterns:

- **Dependency Injection**: Infrastructure clients (Keycloak, Bitcoin RPC, ZPL) are initialized at application startup and injected into services
- **Separation of Concerns**: Service layer focuses on business logic; infrastructure setup is handled in `lib.rs`
- **SQL Executor Pattern**: Database operations are abstracted into trait-based executors for reusability and testability
- **Resource Sharing**: Expensive clients are wrapped in `Arc<T>` for efficient shared ownership without cloning

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run integration tests with database
DATABASE_URL=postgres://postgres:postgres@localhost:5432/mpc_backend_mock \
  cargo test -p mpc-backend-mock-server

# Run with verbose output
RUST_BACKTRACE=1 cargo test --workspace -- --nocapture
```

### Code Quality

```bash
# Run clippy (strict linting)
cargo clippy --workspace --all-targets

# Format code
cargo fmt --all

# Format all file types (requires Nix)
nix fmt
```

### Database Migrations

```bash
# Create a new migration
cargo sqlx migrate add <migration_name>

# Run migrations
cargo sqlx migrate run

# Revert last migration
cargo sqlx migrate revert
```

## Architecture

### Workspace Structure

- `mpc-backend-mock/bin` - CLI application with config management
- `mpc-backend-mock/core` - Core types and configuration
- `mpc-backend-mock/server` - Server implementation (web API, gRPC, services)
- `crates/cli-common` - Common CLI utilities
- `crates/protobuf-types` - Protocol buffer definitions
- `crates/metrics` - Prometheus metrics server
- `crates/web` - Axum web utilities

### Authentication Architecture

- **Keycloak Integration**: External identity provider for user authentication
  - **Admin Client Initialization**: Keycloak admin client is initialized once at application startup
  - Wrapped in `Arc<KeycloakAdmin>` for efficient shared ownership across services
  - Eliminates redundant authentication calls by reusing a single authenticated client
- **JWT Middleware**: Validates Bearer tokens and extracts user claims
- **User Service**:
  - Links database users to Keycloak users via `keycloak_user_id`
  - Uses dependency injection pattern - receives pre-initialized Keycloak admin client
  - Handles user creation and management via Keycloak Admin API
- **Route Protection**: Middleware-based authentication for protected endpoints

### Database Schema

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) NOT NULL,
    keycloak_user_id UUID NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE  -- Soft delete
);

-- Indexes for performance
CREATE UNIQUE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX idx_users_keycloak_user_id ON users(keycloak_user_id) WHERE deleted_at IS NULL;
```

## Health Checks

### gRPC Health Check

```bash
# Install grpcurl
brew install grpcurl

# Check service health
grpcurl -plaintext localhost:14447 grpc.health.v1.Health/Check
```

### HTTP Health Check

```bash
curl http://localhost:14444/api/v1/info
```

### Prometheus Metrics

```bash
curl http://localhost:14446/metrics
```

## Deployment

### Environment Variables

- `OLYMPUS_BACKEND_CONFIG_FILE_PATH` - Path to configuration file
- `DATABASE_URL` - PostgreSQL connection string (for sqlx migrations)
- `RUST_LOG` - Logging level (e.g., `info`, `debug`, `trace`)

### Production Considerations

1. **JWT Validation**: Implement proper JWKS endpoint validation (currently uses insecure dev mode)
2. **SSL/TLS**: Enable SSL for Keycloak and PostgreSQL connections
3. **Secrets Management**: Use secure secret storage (e.g., GCP KMS)
4. **Rate Limiting**: Add rate limiting for public endpoints
5. **CORS**: Configure CORS for frontend integration
6. **Monitoring**: Enable Prometheus metrics and set up alerting

## Troubleshooting

### Database Connection Issues

```bash
# Check if PostgreSQL is running
docker ps | grep postgres

# Test connection
psql -h localhost -p 5432 -U postgres -d mpc_backend_mock
```

### Keycloak Connection Issues

```bash
# Check if Keycloak is running
docker ps | grep keycloak

# Check Keycloak logs
docker logs keycloak

# Verify realm configuration
curl http://localhost:8080/realms/mpc/.well-known/openid-configuration
```

### Authentication Issues

1. Verify JWT token is not expired
2. Check that `keycloak_user_id` matches between database and Keycloak
3. Ensure `Authorization` header uses correct format: `Bearer <token>`
4. Check server logs for detailed error messages

## Contributing

### Code Standards

- Follow Rust naming conventions and idiomatic patterns
- Extremely strict linting with Clippy (see `Cargo.toml`)
- Use `snafu` for error handling
- All public types must derive `utoipa::ToSchema` for OpenAPI
- Use `tracing` for logging (not `log` or `println!`)
- Unsafe code is forbidden

### Commit Messages

```bash
feat: add user authentication
fix: resolve database connection timeout
docs: update README with Keycloak setup
test: add integration tests for JWT validation
chore: update dependencies
```

## License

GPL-3.0-only

## Support

For issues or questions, please open an issue on GitHub or contact the development team.
