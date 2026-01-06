# Keycloak Setup Guide

This guide provides step-by-step instructions for setting up Keycloak for the MPC Backend Mock service.

## Table of Contents

- [Starting Keycloak](#starting-keycloak)
- [Initial Access](#initial-access)
- [Realm Configuration](#realm-configuration)
- [Client Configuration](#client-configuration)
- [User Setup](#user-setup)
- [Service Account Configuration](#service-account-configuration)
- [Testing Authentication](#testing-authentication)

## Starting Keycloak

### Using Docker Compose

Start all services including Keycloak:

```bash
cd dev-support/test-environments/docker-compose
docker compose up -d
```

Or start only PostgreSQL and Keycloak:

```bash
docker compose -f docker-compose.pgonly.yml up -d
```

### Verify Keycloak is Running

Check the health status:

```bash
curl http://localhost:8080/health/ready
```

Expected response: `{"status": "UP", ...}`

View logs:

```bash
docker logs -f mpc-keycloak
```

## Initial Access

### Admin Console

1. Open the Keycloak Admin Console: http://localhost:8080
2. Click on "Administration Console"
3. Login with credentials:
   - Username: `admin`
   - Password: `admin`

**Note:** These are development credentials. Use strong passwords in production.

## Realm Configuration

### Create the MPC Realm

1. In the Admin Console, hover over "master" in the top-left corner
2. Click "Create Realm"
3. Configure the realm:
   - **Realm name**: `mpc`
   - **Enabled**: ON
4. Click "Create"

### Configure Realm Settings

1. Navigate to "Realm settings" in the left menu
2. Configure the following tabs:

#### General Tab

- **User Profile Enabled**: ON (optional, for custom user attributes)
- **User-Managed Access**: OFF (unless you need fine-grained permissions)

#### Login Tab

- **User registration**: OFF (users created via API)
- **Forgot password**: ON (optional)
- **Remember me**: ON (optional)
- **Login with email**: ON

#### Email Tab (Optional)

Configure SMTP settings if you need email verification:

- **From**: noreply@yourdomain.com
- **Host**: your-smtp-server
- **Port**: 587
- **Enable SSL/TLS**: ON

#### Tokens Tab

Configure token lifetimes (recommended for development):

- **Access Token Lifespan**: 5 minutes
- **Access Token Lifespan For Implicit Flow**: 15 minutes
- **Client login timeout**: 5 minutes
- **Login timeout**: 30 minutes
- **Refresh Token Max Reuse**: 0
- **SSO Session Idle**: 30 minutes
- **SSO Session Max**: 10 hours

## Client Configuration

You need to create two clients:

1. **Backend Service Client** (confidential) - for the MPC backend to communicate with Keycloak
2. **Frontend Client** (public) - for the MPC frontend application

### Backend Service Client

1. Navigate to "Clients" in the left menu
2. Click "Create client"
3. Configure **General Settings**:
   - **Client type**: OpenID Connect
   - **Client ID**: `mpc-backend-service`
4. Click "Next"

5. Configure **Capability config**:
   - **Client authentication**: ON
   - **Authorization**: OFF
   - **Authentication flow**:
     - Standard flow: OFF
     - Direct access grants: OFF
     - Service accounts roles: ON
6. Click "Next"

7. Configure **Login settings**:
   - Leave empty (not needed for service account)
8. Click "Save"

9. Go to the "Credentials" tab:
   - Copy the **Client Secret** (you'll need this for the backend configuration)

10. Go to the "Service account roles" tab:
    - Click "Assign role"
    - Filter by clients: Select "realm-management"
    - Assign these roles:
      - `manage-users` (to create/update users)
      - `view-users` (to query users)
      - `query-users` (to search users)

### Frontend Client (Public with PKCE)

The frontend client is configured for OAuth2.0 Authorization Code Flow with PKCE (Proof Key for Code Exchange), which is the recommended flow for Single Page Applications (SPAs) like React+Vite.

1. Navigate to "Clients" in the left menu
2. Click "Create client"
3. Configure **General Settings**:
   - **Client type**: OpenID Connect
   - **Client ID**: `mpc-frontend`
4. Click "Next"

5. Configure **Capability config**:
   - **Client authentication**: OFF (public client)
   - **Authorization**: OFF
   - **Authentication flow**:
     - Standard flow: ON (required for Authorization Code Flow)
     - Direct access grants: ON (for development/testing only)
     - Implicit flow: OFF (deprecated, not secure)
6. Click "Next"

7. Configure **Login settings**:
   - **Root URL**: http://localhost:3000 (or http://localhost:5173 for Vite)
   - **Home URL**: http://localhost:3000
   - **Valid redirect URIs**:
     - http://localhost:3000/\*
     - http://localhost:3001/\*
     - http://localhost:5173/\* (Vite default port)
   - **Valid post logout redirect URIs**: http://localhost:3000/\*
   - **Web origins**:
     - http://localhost:3000
     - http://localhost:3001
     - http://localhost:5173
     - - (to allow CORS from redirect URIs)
8. Click "Save"

9. **Enable PKCE (Important)**:
   - Go to the "Advanced" tab
   - Scroll to **Advanced Settings**
   - Find **Proof Key for Code Exchange Code Challenge Method**
   - Select **S256** (SHA-256 challenge method)
   - Click "Save"

**Why PKCE?**

- PKCE prevents authorization code interception attacks
- Required for public clients (SPAs, mobile apps) that can't securely store secrets
- Uses SHA-256 challenge to ensure the authorization code can only be used by the client that requested it

## User Setup

### Manual User Creation (for testing)

1. Navigate to "Users" in the left menu
2. Click "Create new user"
3. Configure user:
   - **Email**: test@example.com
   - **Email verified**: ON
   - **First name**: Test
   - **Last name**: User
   - **Enabled**: ON
4. Click "Create"

5. Set password:
   - Go to the "Credentials" tab
   - Click "Set password"
   - **Password**: test123
   - **Temporary**: OFF
   - Click "Save"

### Programmatic User Creation

Users will be created via the backend API using the service account. See the API documentation for details.

## Service Account Configuration

### Get Service Account Token

To obtain a token for the backend service account:

```bash
curl -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=mpc-backend-service" \
  -d "client_secret=YOUR_CLIENT_SECRET"
```

Response:

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 300,
  "token_type": "Bearer",
  "not-before-policy": 0,
  "scope": "profile email"
}
```

### Backend Configuration

Add the following to your `config.yaml`:

```yaml
keycloak:
  server_url: "http://localhost:8080"
  realm: "mpc"
  client_id: "mpc-backend-service"
  client_secret: "YOUR_CLIENT_SECRET_HERE"
  admin_username: "admin" # Optional: for admin API access
  admin_password: "admin" # Optional: for admin API access
```

## Testing Authentication

### OAuth2.0 Authorization Code Flow with PKCE (Recommended for Frontend)

This is the recommended flow for React/Vite SPAs. The frontend application should use a library like `@react-keycloak/web` or `keycloak-js` to handle this flow automatically.

#### Step 1: Generate PKCE Code Verifier and Challenge

The frontend generates a random code verifier and creates a SHA-256 hash challenge:

```javascript
// Generate random code verifier (43-128 characters)
const codeVerifier = generateRandomString(128);

// Create SHA-256 hash challenge
const codeChallenge = await sha256(codeVerifier);
const codeChallengeBase64 = base64UrlEncode(codeChallenge);
```

#### Step 2: Redirect to Authorization Endpoint

Direct the user to Keycloak's authorization endpoint with PKCE parameters:

```
http://localhost:8080/realms/mpc/protocol/openid-connect/auth
  ?client_id=mpc-frontend
  &redirect_uri=http://localhost:5173/callback
  &response_type=code
  &scope=openid profile email
  &code_challenge=CODE_CHALLENGE_HERE
  &code_challenge_method=S256
```

#### Step 3: User Authenticates

User logs in with their credentials. Keycloak redirects back to:

```
http://localhost:5173/callback?code=AUTH_CODE&session_state=...
```

#### Step 4: Exchange Authorization Code for Token

The frontend exchanges the authorization code for an access token, providing the original code verifier:

```bash
curl -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "client_id=mpc-frontend" \
  -d "redirect_uri=http://localhost:5173/callback" \
  -d "code=AUTH_CODE" \
  -d "code_verifier=CODE_VERIFIER_HERE"
```

Response:

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 300,
  "refresh_expires_in": 1800,
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "session_state": "...",
  "scope": "openid profile email"
}
```

#### Step 5: Use Access Token to Call Backend API

The frontend includes the access token in API requests:

```bash
curl -X GET http://localhost:14444/api/v1/users/me \
  -H "Authorization: Bearer ACCESS_TOKEN_HERE"
```

The backend validates the token using Keycloak's JWKS endpoint and extracts user claims.

### Test User Login (Simple Flow - Development Only)

### Test Direct Access Grant (for testing only)

```bash
curl -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "client_id=mpc-frontend" \
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
  "session_state": "...",
  "scope": "openid profile email"
}
```

### Validate Token

The backend validates JWT tokens using **JWKS (JSON Web Key Set)** for cryptographic signature verification.

#### Backend Token Validation Flow

1. **Extract Token**: Middleware extracts bearer token from `Authorization` header
2. **Fetch Public Key**: Backend fetches Keycloak's public keys from JWKS endpoint:
   ```
   http://localhost:8080/realms/mpc/protocol/openid-connect/certs
   ```
3. **Verify Signature**: Token signature is verified using the public key (RSA256)
4. **Validate Claims**: Standard JWT claims are validated:
   - `exp` (expiration time)
   - `iat` (issued at time)
   - `iss` (issuer)
   - `aud` (audience) - optional
5. **Extract User Info**: User claims (`sub`, `email`, `preferred_username`) are extracted
6. **Cache Keys**: Public keys are cached for 5 minutes to reduce Keycloak load

#### Manual Token Validation (for debugging)

Get JWKS from Keycloak:

```bash
curl http://localhost:8080/realms/mpc/protocol/openid-connect/certs
```

Decode JWT (use jwt.io or a library):

```javascript
// JWT header contains key ID (kid) used for verification
// JWT payload contains claims (sub, email, exp, etc.)
```

Token introspection (alternative validation method):

```bash
curl -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token/introspect \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=mpc-backend-service" \
  -d "client_secret=YOUR_CLIENT_SECRET" \
  -d "token=ACCESS_TOKEN_HERE"
```

**Note:** The backend uses JWKS verification (not introspection) for better performance.

### Get User Info

```bash
curl -X GET http://localhost:8080/realms/mpc/protocol/openid-connect/userinfo \
  -H "Authorization: Bearer ACCESS_TOKEN_HERE"
```

## Common Operations via Admin API

### Create User via Service Account

```bash
# Get service account token first
TOKEN=$(curl -s -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=mpc-backend-service" \
  -d "client_secret=YOUR_CLIENT_SECRET" | jq -r '.access_token')

# Create user
curl -X POST http://localhost:8080/admin/realms/mpc/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "email": "newuser@example.com",
    "emailVerified": true,
    "enabled": true,
    "firstName": "New",
    "lastName": "User",
    "credentials": [{
      "type": "password",
      "value": "password123",
      "temporary": false
    }]
  }'
```

### Query User

```bash
# Get user by email
curl -X GET "http://localhost:8080/admin/realms/mpc/users?email=test@example.com" \
  -H "Authorization: Bearer $TOKEN"
```

### Update User

```bash
# Get user ID first, then update
curl -X PUT http://localhost:8080/admin/realms/mpc/users/USER_ID \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "firstName": "Updated",
    "lastName": "Name"
  }'
```

## Troubleshooting

### Keycloak won't start

Check logs:

```bash
docker logs mpc-keycloak
```

Common issues:

- PostgreSQL not ready: Wait for postgres to be healthy
- Port 8080 already in use: Change port in docker-compose.yaml
- Database connection failed: Check KC_DB_URL and credentials

### Cannot login to Admin Console

- Verify admin credentials in docker-compose.yaml
- Check KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD environment variables
- Try restarting the container: `docker restart mpc-keycloak`

### Token validation fails

- Check token expiration time
- Verify client configuration (client_id matches)
- Ensure realm name is correct (`mpc`)
- Check that the token is being sent in Authorization header: `Bearer TOKEN`

### User creation fails

- Verify service account has `manage-users` role
- Check that client_secret is correct
- Ensure email is unique
- Check Keycloak logs for detailed error

## Security Recommendations

### For Development

- Current settings are suitable for local development
- HTTP is acceptable (KC_HTTP_ENABLED: true)
- Simple admin password is acceptable

### For Production

- **Use HTTPS**: Set KC_HTTPS_ENABLED: true and provide certificates
- **Strong admin password**: Change KEYCLOAK_ADMIN_PASSWORD
- **External database**: Use dedicated PostgreSQL instance
- **Rotate client secrets**: Change default client_secret regularly
- **Enable email verification**: Configure SMTP and require email verification
- **Set up rate limiting**: Protect against brute force attacks
- **Use environment-specific realms**: Don't mix dev/staging/prod users
- **Regular backups**: Backup Keycloak database and configuration
- **Monitor logs**: Set up log aggregation and alerting

## Additional Resources

- [Keycloak Documentation](https://www.keycloak.org/documentation)
- [Admin REST API](https://www.keycloak.org/docs-api/latest/rest-api/index.html)
- [OpenID Connect Endpoints](https://www.keycloak.org/docs/latest/securing_apps/index.html#endpoints)
- [Server Administration Guide](https://www.keycloak.org/docs/latest/server_admin/)
