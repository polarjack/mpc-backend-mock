#!/bin/bash

COMPOSE_DIR="./dev-support/test-environments/docker-compose"

echo "=========================================="
echo "MPC Backend Environment Reset"
echo "=========================================="
echo ""

# Stop and remove MPC PostgreSQL
echo "Stopping and removing MPC PostgreSQL..."
docker compose -f "$COMPOSE_DIR/docker-compose.pgonly.yml" down -v

# Stop and remove Keycloak stack
echo "Stopping and removing Keycloak stack..."
docker compose -f "$COMPOSE_DIR/docker-compose.keycloak.yml" down -v

echo ""
echo "Starting services..."
echo ""

# Start MPC PostgreSQL
echo "Starting MPC PostgreSQL..."
docker compose -f "$COMPOSE_DIR/docker-compose.pgonly.yml" up -d

# Start Keycloak stack
echo "Starting Keycloak stack (PostgreSQL + Keycloak)..."
docker compose -f "$COMPOSE_DIR/docker-compose.keycloak.yml" up -d

echo ""
echo "Waiting for services to be healthy..."
echo ""

# Wait for MPC PostgreSQL
echo "Waiting for MPC PostgreSQL..."
until docker exec mpc-postgres pg_isready -U mpc_postgres > /dev/null 2>&1; do
  sleep 1
done
echo "✓ MPC PostgreSQL is ready!"

# Wait for Keycloak PostgreSQL
echo "Waiting for Keycloak PostgreSQL..."
until docker exec keycloak-postgres pg_isready -U keycloak > /dev/null 2>&1; do
  sleep 1
done
echo "✓ Keycloak PostgreSQL is ready!"

# Wait for Keycloak
echo "Waiting for Keycloak (this may take 30-60 seconds)..."
until curl -sf http://localhost:8080/realms/master > /dev/null 2>&1; do
  sleep 2
done
echo "✓ Keycloak is ready!"

echo ""
echo "=========================================="
echo "All services are ready!"
echo "=========================================="
echo ""
echo "MPC Backend Services:"
echo "  - MPC PostgreSQL:     localhost:8432"
echo "    Database: mpc"
echo "    User: mpc_postgres / mpc_password"
echo ""
echo "Keycloak Services:"
echo "  - Keycloak PostgreSQL: localhost:5433"
echo "    Database: keycloak"
echo "    User: keycloak / keycloak"
echo "  - Keycloak Admin:      http://localhost:8080"
echo "    Credentials: admin / admin"
echo ""
echo "=========================================="
echo "Next Steps:"
echo "=========================================="
echo "1. Configure Keycloak:"
echo "   cd $COMPOSE_DIR"
echo "   ./setup-keycloak.sh"
echo ""
echo "2. Run database migrations:"
echo "   cargo sqlx migrate run"
echo ""
echo "3. Start the MPC backend server:"
echo "   ./run.sh"
echo ""
echo "For detailed setup instructions, see:"
echo "  $COMPOSE_DIR/KEYCLOAK_SETUP.md"
echo ""
