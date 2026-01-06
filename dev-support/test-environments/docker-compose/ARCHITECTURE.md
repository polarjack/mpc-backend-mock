# MPC Backend Docker Architecture

## Overview

The MPC backend development environment uses **separated Docker Compose stacks** for better isolation and flexibility.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Development Environment                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌───────────────────────────┐  ┌──────────────────────────┐   │
│  │   MPC Backend Stack       │  │   Keycloak Stack         │   │
│  │  (pgonly.yml)             │  │  (keycloak.yml)          │   │
│  │                           │  │                          │   │
│  │  ┌─────────────────────┐ │  │  ┌────────────────────┐ │   │
│  │  │  mpc-postgres       │ │  │  │ keycloak-postgres  │ │   │
│  │  │  Port: 8432         │ │  │  │ Port: 5433         │ │   │
│  │  │  DB: mpc            │ │  │  │ DB: keycloak       │ │   │
│  │  │  User: mpc_postgres │ │  │  │ User: keycloak     │ │   │
│  │  └─────────────────────┘ │  │  └────────────────────┘ │   │
│  │           ▲               │  │           ▲            │   │
│  │           │               │  │           │            │   │
│  └───────────┼───────────────┘  │  ┌────────┼──────────┐ │   │
│              │                  │  │ mpc-keycloak      │ │   │
│              │                  │  │ Port: 8080        │ │   │
│       ┌──────┴────────┐         │  │ Admin: admin      │ │   │
│       │ MPC Backend   │         │  └───────────────────┘ │   │
│       │ (Rust App)    │─────────┼───────────►            │   │
│       │ Port: 14444   │  Auth   │      HTTP/JWT          │   │
│       └───────────────┘         │                        │   │
│                                 └────────────────────────┘   │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

## Stack Details

### 1. MPC Backend Stack

**File**: `docker-compose.pgonly.yml`
**Network**: `mpc-postgres-only_default`

```yaml
Services:
  - mpc-postgres:
      Image: postgres:17.4
      Port: 8432 (host) -> 5432 (container)
      Database: mpc
      Credentials: mpc_postgres / mpc_password
      Purpose: Application data storage
```

### 2. Keycloak Stack

**File**: `docker-compose.keycloak.yml`
**Network**: `mpc-keycloak_default`

```yaml
Services:
  - keycloak-postgres:
      Image: postgres:17.4
      Port: 5433 (host) -> 5432 (container)
      Database: keycloak
      Credentials: keycloak / keycloak
      Purpose: Keycloak configuration and user data

  - keycloak:
      Image: quay.io/keycloak/keycloak:26.4.7
      Port: 8080
      Admin: admin / admin
      Purpose: Authentication and authorization
      Depends: keycloak-postgres
```

### 3. Full Environment (Optional)

**File**: `docker-compose.yaml`
**Network**: `mpc-backend-mock_default`

```yaml
Services:
  - postgres: PostgreSQL for blockchain data (port 5432)
  - bitcoind: Bitcoin Core in regtest mode
  - electrs: Electrum server
  - btc-rpc-explorer: Bitcoin blockchain explorer
```

**Note**: Keycloak should be started separately using `docker-compose.keycloak.yml`

## Design Rationale

### Why Separate Stacks?

#### 1. **Isolation**

- Keycloak data is completely independent from application data
- Database failures in one stack don't affect the other
- Clear separation of concerns: authentication vs. application

#### 2. **Independent Lifecycle**

- Update Keycloak without touching application database
- Reset application data without losing Keycloak users/config
- Scale each component independently

#### 3. **Production-Like Architecture**

- Mirrors production where Keycloak often runs separately
- Easier to migrate to managed Keycloak services (e.g., Red Hat SSO)
- Better security boundaries

#### 4. **Development Flexibility**

- Run only what you need
- Faster startup when you don't need both
- Test different Keycloak versions easily

#### 5. **Port Clarity**

```
8432 - MPC Application Database
5433 - Keycloak Database
8080 - Keycloak HTTP
14444 - MPC Backend API
14446 - MPC Metrics
14447 - MPC Health Check (gRPC)
```

## Network Communication

### Between Stacks

- Stacks are on **different Docker networks**
- Communication happens via **localhost** (host network)
- MPC Backend connects to Keycloak at `http://localhost:8080`

### Within Keycloak Stack

- Keycloak connects to its PostgreSQL via **service name**: `keycloak-postgres`
- Internal Docker DNS resolution
- No ports exposed to host for inter-service communication

## Data Persistence

### Volumes

```yaml
MPC Backend Stack:
  - mpc-postgres-data: Application database data

Keycloak Stack:
  - keycloak-postgres-data: Keycloak database data
  - keycloak-data: Keycloak config and themes
```

### Volume Management

```bash
# List all volumes
docker volume ls

# Inspect volume
docker volume inspect mpc-postgres-only_mpc-postgres-data
docker volume inspect mpc-keycloak_keycloak-postgres-data

# Remove volumes (when stacks are down)
docker volume rm mpc-postgres-only_mpc-postgres-data
docker volume rm mpc-keycloak_keycloak-postgres-data
```

## Common Operations

### Start Everything

```bash
./reset.sh
```

### Start Selectively

```bash
# Only MPC database
docker compose -f docker-compose.pgonly.yml up -d

# Only Keycloak
docker compose -f docker-compose.keycloak.yml up -d
```

### Stop Everything

```bash
docker compose -f docker-compose.pgonly.yml down
docker compose -f docker-compose.keycloak.yml down
```

### Clean Reset (Delete All Data)

```bash
docker compose -f docker-compose.pgonly.yml down -v
docker compose -f docker-compose.keycloak.yml down -v
```

### View Logs

```bash
# MPC database
docker logs mpc-postgres -f

# Keycloak database
docker logs keycloak-postgres -f

# Keycloak
docker logs mpc-keycloak -f
```

## Migration from Previous Setup

### What Changed?

**Before** (monolithic):

- Single `docker-compose.pgonly.yml` with both services
- Shared PostgreSQL with multiple databases
- Keycloak database created via init script
- Single Docker network

**After** (separated):

- Two independent compose files
- Each service has its own PostgreSQL instance
- No init scripts needed
- Separate Docker networks

### Migration Steps

1. **Stop old setup**:

   ```bash
   docker compose -f docker-compose.pgonly.yml down -v
   ```

2. **Start new setup**:

   ```bash
   ./reset.sh
   ```

3. **Reconfigure Keycloak**:

   ```bash
   cd dev-support/test-environments/docker-compose
   ./setup-keycloak.sh
   ```

4. **Update config.yaml**:
   - Keycloak URL remains: `http://localhost:8080`
   - No changes needed to backend configuration

## Production Considerations

When deploying to production:

1. **Use External Databases**
   - Managed PostgreSQL (AWS RDS, GCP Cloud SQL, etc.)
   - Separate instances for MPC and Keycloak

2. **Use Managed Keycloak**
   - Red Hat SSO / RHSSO
   - AWS Cognito integration
   - Azure AD B2C

3. **Networking**
   - Private subnets for databases
   - Load balancers for Keycloak
   - TLS/HTTPS everywhere

4. **Secrets Management**
   - Use secrets managers (AWS Secrets Manager, HashiCorp Vault)
   - Rotate credentials regularly
   - Never hardcode passwords

5. **Monitoring**
   - Database metrics and alerts
   - Keycloak health checks
   - Connection pool monitoring

## Troubleshooting

### Issue: Keycloak can't connect to its database

**Check**:

```bash
# Is Keycloak PostgreSQL running?
docker ps | grep keycloak-postgres

# Is it healthy?
docker exec keycloak-postgres pg_isready -U keycloak

# Check Keycloak logs
docker logs mpc-keycloak
```

**Solution**: Ensure `keycloak-postgres` is healthy before Keycloak starts. The `depends_on` with health check should handle this.

### Issue: MPC Backend can't connect to Keycloak

**Check**:

```bash
# Is Keycloak accessible?
curl http://localhost:8080/health/ready

# Check MPC backend config
grep -A 5 "keycloak:" config.yaml
```

**Solution**:

- Use `http://localhost:8080` not `http://keycloak:8080`
- Stacks are on different networks, use host networking

### Issue: Port conflicts

**Check**:

```bash
lsof -i :8432  # MPC PostgreSQL
lsof -i :5433  # Keycloak PostgreSQL
lsof -i :8080  # Keycloak
```

**Solution**: Modify port mappings in respective compose files.

## Further Reading

- [Docker Compose Networking](https://docs.docker.com/compose/networking/)
- [PostgreSQL Docker Official Image](https://hub.docker.com/_/postgres)
- [Keycloak on Docker](https://www.keycloak.org/server/containers)
- [Database Separation Best Practices](https://www.postgresql.org/docs/current/server-start.html)
