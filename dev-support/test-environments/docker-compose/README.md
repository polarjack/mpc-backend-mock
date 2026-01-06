# Docker Compose Configuration for Setting up Test Environment

This directory contains Docker Compose configurations for different test environments.

## Available Configurations

### 1. MPC Backend Development Stack (Recommended)

For backend development, you need two separate stacks running:

#### a) MPC PostgreSQL (`docker-compose.pgonly.yml`)

Database for the MPC backend application.

```bash
docker compose -f docker-compose.pgonly.yml up -d
```

**Services:**

- PostgreSQL 17.4 on port 8432
- Database: `mpc`
- Credentials: `mpc_postgres` / `mpc_password`

#### b) Keycloak Stack (`docker-compose.keycloak.yml`)

Standalone Keycloak with its own dedicated PostgreSQL instance.

```bash
docker compose -f docker-compose.keycloak.yml up -d
```

**Services:**

- Keycloak PostgreSQL 17.4 on port 5433
  - Database: `keycloak`
  - Credentials: `keycloak` / `keycloak`
- Keycloak 26.4.7 on ports 8080 (HTTP) and 9000 (Management)
  - Admin credentials: `admin` / `admin`
  - Admin console: http://localhost:8080
  - Management/Health: http://localhost:9000

#### Quick Setup (Both Stacks)

Use the convenience script from the project root:

```bash
# From project root - starts both MPC PostgreSQL and Keycloak stack
./reset.sh
```

This will:

- Stop and remove all containers and volumes
- Start MPC PostgreSQL (port 8432)
- Start Keycloak stack (PostgreSQL on 5433, Keycloak on 8080)
- Wait for all services to be healthy
- Display connection information

**Then configure Keycloak:**

```bash
cd dev-support/test-environments/docker-compose
./setup-keycloak.sh
```

**Finally, run migrations and start the backend:**

```bash
cargo sqlx migrate run
./run.sh
```

### 2. Full Environment with Bitcoin (`docker-compose.yaml`)

This configuration includes PostgreSQL and Bitcoin-related services for full blockchain testing.

**Note:** Start Keycloak separately using `docker-compose.keycloak.yml`

```bash
# Prepare Bitcoin data directory
mkdir bitcoind-data && sudo chmod a+rwx bitcoind-data && sudo chown -hR 1000:1000 bitcoind-data

# Start Bitcoin services
docker compose up -d

# Start Keycloak in separate stack
docker compose -f docker-compose.keycloak.yml up -d
```

**Services:**

- PostgreSQL 17 on port 5432 (database: `postgres`)
- Bitcoin Core (regtest) on ports 18443/18444
- Electrs on port 50001
- BTC RPC Explorer on port 3002

Plus Keycloak stack (when started separately):

- Keycloak PostgreSQL on port 5433
- Keycloak on port 8080

**Bitcoin Setup:**

```bash
# Create wallet for mining
docker exec -it bitcoind \
    bitcoin-cli \
    -regtest \
    -rpcuser=bitcoin \
    -rpcpassword=bitcoin \
    createwallet "default"
```

## Keycloak Setup

After starting the services, configure Keycloak for authentication:

### Automated Setup (Recommended)

Run the automated setup script:

```bash
cd dev-support/test-environments/docker-compose
./setup-keycloak.sh
```

This script will:

- Create the `mpc` realm
- Create backend service client (`mpc-backend-service`)
- Create frontend client (`mpc-frontend`)
- Assign required roles to service account
- Create a test user (test@example.com / test123)
- Display the client secret for your config.yaml

### Manual Setup

See [KEYCLOAK_SETUP.md](./KEYCLOAK_SETUP.md) for detailed manual configuration steps.

### Access Keycloak

- **Admin Console**: http://localhost:8080
- **Username**: admin
- **Password**: admin

## Configuration Files

After running the Keycloak setup, add the following to your `config.yaml`:

```yaml
keycloak:
  server_url: "http://localhost:8080"
  realm: "mpc"
  client_id: "mpc-backend-service"
  client_secret: "YOUR_CLIENT_SECRET_FROM_SETUP_SCRIPT"
```

## Architecture Benefits

Separating Keycloak into its own stack provides:

- **Isolation**: Keycloak database is independent from application data
- **Independent Scaling**: Scale Keycloak separately from the backend
- **Cleaner Separation**: Clear boundary between authentication and application
- **Port Clarity**:
  - 8432: MPC application database
  - 5433: Keycloak database
  - 8080: Keycloak HTTP service
  - 9000: Keycloak management/metrics

## Useful Commands

### View Logs

```bash
# MPC PostgreSQL
docker compose -f docker-compose.pgonly.yml logs -f

# Keycloak stack
docker compose -f docker-compose.keycloak.yml logs -f

# Specific service in Keycloak stack
docker compose -f docker-compose.keycloak.yml logs -f keycloak
docker compose -f docker-compose.keycloak.yml logs -f keycloak-postgres
```

### Stop Services

```bash
# Stop MPC PostgreSQL
docker compose -f docker-compose.pgonly.yml down

# Stop Keycloak stack
docker compose -f docker-compose.keycloak.yml down

# Stop and remove volumes (clean slate)
docker compose -f docker-compose.pgonly.yml down -v
docker compose -f docker-compose.keycloak.yml down -v
```

### Reset Everything

```bash
# From project root - resets both MPC and Keycloak stacks
./reset.sh
```

### Start Only What You Need

```bash
# Only MPC PostgreSQL (if you have Keycloak elsewhere)
docker compose -f docker-compose.pgonly.yml up -d

# Only Keycloak stack (if you have MPC database elsewhere)
docker compose -f docker-compose.keycloak.yml up -d
```

## Troubleshooting

### Keycloak won't start

- Ensure Keycloak PostgreSQL is healthy: `docker ps | grep keycloak-postgres`
- Check Keycloak logs: `docker logs mpc-keycloak`
- Check Keycloak database logs: `docker logs keycloak-postgres`
- Verify ports are not in use: `lsof -i :8080` and `lsof -i :9000`
- Verify Keycloak can connect to its database:
  ```bash
  docker exec keycloak-postgres pg_isready -U keycloak
  ```
- Check if Keycloak is accessible:

  ```bash
  # Check main service (should return realm info)
  curl http://localhost:8080/realms/master

  # Check management endpoint (Keycloak 26.x uses port 9000 for health)
  curl http://localhost:9000/health/ready
  ```

### MPC PostgreSQL connection failed

- Check if container is running: `docker ps | grep mpc-postgres`
- Test connection: `docker exec mpc-postgres pg_isready -U mpc_postgres`
- Verify port 8432 is not in use: `lsof -i :8432`

### Keycloak database error

If you see "database does not exist" errors:

- Keycloak now has its own dedicated PostgreSQL instance
- Make sure you're using `docker-compose.keycloak.yml` which includes both Keycloak and its database
- The Keycloak database is automatically created by the keycloak-postgres container

### Port conflicts

If ports are already in use, modify the port mappings in the docker-compose files:

- MPC PostgreSQL: Change `8432:5432` to another port in `docker-compose.pgonly.yml`
- Keycloak PostgreSQL: Change `5433:5432` to another port in `docker-compose.keycloak.yml`
- Keycloak: Change `8080:8080` to another port in `docker-compose.keycloak.yml`

### Network Issues Between Services

If the MPC backend can't connect to Keycloak:

- Both stacks are on different Docker networks
- Access Keycloak at `http://localhost:8080` (not container names)
- Ensure your `config.yaml` uses `http://localhost:8080` as the Keycloak URL

## Additional Resources

- [Keycloak Setup Guide](./KEYCLOAK_SETUP.md) - Detailed manual setup
- [Keycloak Documentation](https://www.keycloak.org/documentation)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
