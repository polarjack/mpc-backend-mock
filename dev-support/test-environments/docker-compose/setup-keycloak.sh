#!/bin/bash

# Keycloak Realm Setup Script
# This script automatically configures Keycloak for the MPC backend

set -e

KEYCLOAK_URL="http://localhost:8080"
ADMIN_USER="admin"
ADMIN_PASSWORD="admin"
REALM_NAME="mpc"
BACKEND_CLIENT_ID="mpc-backend-service"
FRONTEND_CLIENT_ID="mpc-frontend"

echo "=========================================="
echo "Keycloak Setup Script for MPC Backend"
echo "=========================================="
echo ""

# Check if Keycloak is running
echo "Checking if Keycloak is accessible..."
if ! curl -sf "$KEYCLOAK_URL/realms/master" > /dev/null; then
    echo "ERROR: Keycloak is not accessible at $KEYCLOAK_URL"
    echo "Please start Keycloak first:"
    echo "  cd dev-support/test-environments/docker-compose"
    echo "  docker compose -f docker-compose.keycloak.yml up -d"
    echo ""
    echo "Or use the reset script from project root:"
    echo "  ./reset.sh"
    exit 1
fi
echo "✓ Keycloak is accessible"
echo ""

# Get admin access token
echo "Authenticating as admin..."
ADMIN_TOKEN=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=$ADMIN_USER" \
  -d "password=$ADMIN_PASSWORD" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r '.access_token')

if [ "$ADMIN_TOKEN" == "null" ] || [ -z "$ADMIN_TOKEN" ]; then
    echo "ERROR: Failed to authenticate with Keycloak"
    echo "Please check admin credentials"
    exit 1
fi
echo "✓ Admin authentication successful"
echo ""

# Check if realm already exists
echo "Checking if realm '$REALM_NAME' exists..."
REALM_EXISTS=$(curl -s -o /dev/null -w "%{http_code}" \
  -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME" \
  -H "Authorization: Bearer $ADMIN_TOKEN")

if [ "$REALM_EXISTS" == "200" ]; then
    echo "⚠ Realm '$REALM_NAME' already exists"
    read -p "Do you want to delete and recreate it? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Deleting existing realm..."
        curl -s -X DELETE "$KEYCLOAK_URL/admin/realms/$REALM_NAME" \
          -H "Authorization: Bearer $ADMIN_TOKEN"
        echo "✓ Realm deleted"
    else
        echo "Skipping realm creation. Proceeding with client setup..."
    fi
else
    echo "✓ Realm does not exist, will create new one"
fi
echo ""

# Create realm if it doesn't exist
if [ "$REALM_EXISTS" != "200" ] || [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Creating realm '$REALM_NAME'..."
    curl -s -X POST "$KEYCLOAK_URL/admin/realms" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"realm\": \"$REALM_NAME\",
        \"enabled\": true,
        \"loginWithEmailAllowed\": true,
        \"duplicateEmailsAllowed\": false,
        \"resetPasswordAllowed\": true,
        \"rememberMe\": true,
        \"accessTokenLifespan\": 300,
        \"ssoSessionIdleTimeout\": 1800,
        \"ssoSessionMaxLifespan\": 36000,
        \"offlineSessionIdleTimeout\": 2592000,
        \"accessCodeLifespanUserAction\": 300,
        \"accessCodeLifespanLogin\": 1800
      }"
    echo "✓ Realm '$REALM_NAME' created"
    echo ""
fi

# Get a fresh token for the new realm
echo "Getting realm-specific admin token..."
ADMIN_TOKEN=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=$ADMIN_USER" \
  -d "password=$ADMIN_PASSWORD" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r '.access_token')

# Create backend service client (confidential)
echo "Creating backend service client '$BACKEND_CLIENT_ID'..."

# Check if client already exists
BACKEND_CLIENT_EXISTS=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=$BACKEND_CLIENT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id // empty')

if [ -z "$BACKEND_CLIENT_EXISTS" ]; then
    BACKEND_CLIENT_RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"clientId\": \"$BACKEND_CLIENT_ID\",
        \"enabled\": true,
        \"protocol\": \"openid-connect\",
        \"publicClient\": false,
        \"serviceAccountsEnabled\": true,
        \"directAccessGrantsEnabled\": false,
        \"standardFlowEnabled\": false,
        \"implicitFlowEnabled\": false,
        \"authorizationServicesEnabled\": false,
        \"bearerOnly\": false,
        \"consentRequired\": false
      }" \
      -w "\n%{http_code}")

    HTTP_CODE=$(echo "$BACKEND_CLIENT_RESPONSE" | tail -n 1)
    if [ "$HTTP_CODE" == "201" ]; then
        echo "✓ Backend client created"
    else
        echo "✓ Backend client already exists or was created"
    fi
else
    echo "✓ Backend client already exists"
fi
echo ""

# Get backend client details and secret
echo "Retrieving backend client credentials..."
BACKEND_CLIENT_ID_UUID=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=$BACKEND_CLIENT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id')

BACKEND_CLIENT_SECRET=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$BACKEND_CLIENT_ID_UUID/client-secret" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.value')

echo "✓ Backend client secret retrieved"
echo ""

# Get service account user ID
SERVICE_ACCOUNT_USER=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$BACKEND_CLIENT_ID_UUID/service-account-user" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.id')

# Get realm-management client ID
REALM_MGMT_CLIENT_ID=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=realm-management" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id')

# Get required roles
echo "Assigning roles to service account..."
MANAGE_USERS_ROLE=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$REALM_MGMT_CLIENT_ID/roles/manage-users" \
  -H "Authorization: Bearer $ADMIN_TOKEN")

VIEW_USERS_ROLE=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$REALM_MGMT_CLIENT_ID/roles/view-users" \
  -H "Authorization: Bearer $ADMIN_TOKEN")

QUERY_USERS_ROLE=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$REALM_MGMT_CLIENT_ID/roles/query-users" \
  -H "Authorization: Bearer $ADMIN_TOKEN")

# Assign roles to service account
curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/users/$SERVICE_ACCOUNT_USER/role-mappings/clients/$REALM_MGMT_CLIENT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d "[
    $MANAGE_USERS_ROLE,
    $VIEW_USERS_ROLE,
    $QUERY_USERS_ROLE
  ]" > /dev/null

echo "✓ Service account roles assigned"
echo ""

# Create frontend client (public)
echo "Creating frontend client '$FRONTEND_CLIENT_ID'..."

FRONTEND_CLIENT_EXISTS=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=$FRONTEND_CLIENT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id // empty')

if [ -z "$FRONTEND_CLIENT_EXISTS" ]; then
    curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"clientId\": \"$FRONTEND_CLIENT_ID\",
        \"enabled\": true,
        \"protocol\": \"openid-connect\",
        \"publicClient\": true,
        \"directAccessGrantsEnabled\": true,
        \"standardFlowEnabled\": true,
        \"implicitFlowEnabled\": false,
        \"redirectUris\": [
          \"http://localhost:3000/*\",
          \"http://localhost:3001/*\",
          \"http://localhost:5173/*\"
        ],
        \"webOrigins\": [
          \"http://localhost:3000\",
          \"http://localhost:3001\",
          \"http://localhost:5173\",
          \"+\"
        ],
        \"rootUrl\": \"http://localhost:3000\",
        \"baseUrl\": \"http://localhost:3000\",
        \"attributes\": {
          \"pkce.code.challenge.method\": \"S256\"
        }
      }" > /dev/null
    echo "✓ Frontend client created with PKCE support"
else
    echo "✓ Frontend client already exists"
fi
echo ""

# Create test user
echo "Creating test user..."
TEST_USER_EMAIL="test@example.com"

# Check if user exists
TEST_USER_EXISTS=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/users?email=$TEST_USER_EMAIL" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id // empty')

if [ -z "$TEST_USER_EXISTS" ]; then
    # Create user
    curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/users" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"username\": \"testuser\",
        \"email\": \"$TEST_USER_EMAIL\",
        \"emailVerified\": true,
        \"enabled\": true,
        \"firstName\": \"Test\",
        \"lastName\": \"User\"
      }" > /dev/null

    # Get user ID
    TEST_USER_ID=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/users?email=$TEST_USER_EMAIL" \
      -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id')

    # Set password
    curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/users/$TEST_USER_ID/reset-password" \
      -H "Authorization: Bearer $ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"type\": \"password\",
        \"value\": \"test123\",
        \"temporary\": false
      }" > /dev/null

    echo "✓ Test user created: $TEST_USER_EMAIL / test123"
else
    echo "✓ Test user already exists: $TEST_USER_EMAIL"
fi
echo ""

# Summary
echo "=========================================="
echo "Setup Complete!"
echo "=========================================="
echo ""
echo "Realm Configuration:"
echo "  Realm Name: $REALM_NAME"
echo "  Realm URL: $KEYCLOAK_URL/realms/$REALM_NAME"
echo ""
echo "Backend Service Client:"
echo "  Client ID: $BACKEND_CLIENT_ID"
echo "  Client Secret: $BACKEND_CLIENT_SECRET"
echo "  Service Account Roles: manage-users, view-users, query-users"
echo ""
echo "Frontend Client:"
echo "  Client ID: $FRONTEND_CLIENT_ID"
echo "  Type: Public (with PKCE S256)"
echo "  Redirect URIs: http://localhost:3000/*, http://localhost:3001/*, http://localhost:5173/*"
echo "  PKCE: Required (S256)"
echo ""
echo "Test User:"
echo "  Email: $TEST_USER_EMAIL"
echo "  Password: test123"
echo ""
echo "Add this to your config.yaml:"
echo "---"
echo "keycloak:"
echo "  server_url: \"$KEYCLOAK_URL\""
echo "  realm: \"$REALM_NAME\""
echo "  client_id: \"$BACKEND_CLIENT_ID\""
echo "  client_secret: \"$BACKEND_CLIENT_SECRET\""
echo "---"
echo ""
echo "Test the setup:"
echo "  # Get service account token"
echo "  curl -X POST $KEYCLOAK_URL/realms/$REALM_NAME/protocol/openid-connect/token \\"
echo "    -d 'grant_type=client_credentials' \\"
echo "    -d 'client_id=$BACKEND_CLIENT_ID' \\"
echo "    -d 'client_secret=$BACKEND_CLIENT_SECRET'"
echo ""
echo "  # Login as test user"
echo "  curl -X POST $KEYCLOAK_URL/realms/$REALM_NAME/protocol/openid-connect/token \\"
echo "    -d 'grant_type=password' \\"
echo "    -d 'client_id=$FRONTEND_CLIENT_ID' \\"
echo "    -d 'username=$TEST_USER_EMAIL' \\"
echo "    -d 'password=test123'"
echo ""
