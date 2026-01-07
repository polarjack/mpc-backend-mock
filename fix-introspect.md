keycloak token introspect sample

this config is only for backend service to access keycloak

```bash
REALM: mpc
CLIENT_ID: mpc-backend-service
CLIENT_SECRET: rlojUqcDXfDTtbpy3RLACzAlKlV
```

sample request 

```bash
curl -X POST http://localhost:8080/realms/mpc/protocol/openid-connect/token/introspect \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -u "mpc-backend-service:rlojUqcDXfDTtbpy3RLACzAlKlVcdJmw" \
  -d "token=eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJEZm1TTVBuMHRwbUM3WWJJRm5PcjlUM2hTdzBnUlVRQUd2MUNNcjBQQ2hjIn0.eyJleHAiOjE3Njc3NzM2OTYsImlhdCI6MTc2Nzc3MzM5NiwiYXV0aF90aW1lIjoxNzY3NzcwNzIzLCJqdGkiOiJvbnJ0YWM6YTNmM2I4NjQtNTc1My1mMzQzLTE4ZmItN2U5OTlkOTNmNTEyIiwiaXNzIjoiaHR0cDovL2xvY2FsaG9zdDo4MDgwL3JlYWxtcy9tcGMiLCJhdWQiOiJhY2NvdW50Iiwic3ViIjoiNzZkMGE4M2YtODA1Zi00Y2ZmLThhODAtMmM0ZTJlNzk0MDE5IiwidHlwIjoiQmVhcmVyIiwiYXpwIjoibXBjLWZyb250ZW5kIiwic2lkIjoiNjgzZWVmNzMtNWMzYS05YzViLTk3M2EtMmFlOTlmMzJmN2YyIiwiYWNyIjoiMCIsImFsbG93ZWQtb3JpZ2lucyI6WyJodHRwOi8vbG9jYWxob3N0OjMwMDEiLCJodHRwOi8vbG9jYWxob3N0OjMwMDAiLCJodHRwOi8vbG9jYWxob3N0OjUxNzMiXSwicmVhbG1fYWNjZXNzIjp7InJvbGVzIjpbIm9mZmxpbmVfYWNjZXNzIiwidW1hX2F1dGhvcml6YXRpb24iLCJkZWZhdWx0LXJvbGVzLW1wYyJdfSwicmVzb3VyY2VfYWNjZXNzIjp7ImFjY291bnQiOnsicm9sZXMiOlsibWFuYWdlLWFjY291bnQiLCJtYW5hZ2UtYWNjb3VudC1saW5rcyIsInZpZXctcHJvZmlsZSJdfX0sInNjb3BlIjoib3BlbmlkIGVtYWlsIHByb2ZpbGUiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsIm5hbWUiOiJzYW1wbGUgc2FtcGxlIiwicHJlZmVycmVkX3VzZXJuYW1lIjoic2FtcGxlQHNhbXBsZS5jb20iLCJnaXZlbl9uYW1lIjoic2FtcGxlIiwiZmFtaWx5X25hbWUiOiJzYW1wbGUiLCJlbWFpbCI6InNhbXBsZUBzYW1wbGUuY29tIn0.NSGmOv9tpql1YS_xFzAfAP2MIrVY5prHcHINpIcFihYjAInwXNwZyZRRIr11ArBCGcPKZD5XdteC75FHvyaSRpfdrjgEUVu6YQfwvEWzFr9fBmv6pg5hzEvFUufXg2Nm3r2PcGJ-rAqDMEm41nMQhWV3BX9i903_-Lq3DJFxpB8VN22Yskl4CwJPB2JtrFNj4o_LQj-iSw6xdYA69vX6r-5n-6yj9AxZVKDM8AwZv6wZVwlXnajIkg2ocBR8IU7GaozriCQbI9d8TH0j--ZXcM-wNWMkjwPcHf2AtDTDoxtMpBhmqPl9vhhDik6EFMbLVV1JlKxACvYhmThvrpQvTA"
  ```

success response

```json
{
  "exp": 1767773696,
  "iat": 1767773396,
  "auth_time": 1767770723,
  "jti": "onrtac:a3f3b864-5753-f343-18fb-7e999d93f512",
  "iss": "http://localhost:8080/realms/mpc",
  "aud": "account",
  "sub": "76d0a83f-805f-4cff-8a80-2c4e2e794019",
  "typ": "Bearer",
  "azp": "mpc-frontend",
  "sid": "683eef73-5c3a-9c5b-973a-2ae99f32f7f2",
  "acr": "0",
  "allowed-origins": [
    "http://localhost:3001",
    "http://localhost:3000",
    "http://localhost:5173"
  ],
  "realm_access": {
    "roles": [
      "offline_access",
      "uma_authorization",
      "default-roles-mpc"
    ]
  },
  "resource_access": {
    "account": {
      "roles": [
        "manage-account",
        "manage-account-links",
        "view-profile"
      ]
    }
  },
  "scope": "openid email profile",
  "email_verified": false,
  "name": "sample sample",
  "preferred_username": "sample@sample.com",
  "given_name": "sample",
  "family_name": "sample",
  "email": "sample@sample.com",
  "client_id": "mpc-frontend",
  "username": "sample@sample.com",
  "token_type": "Bearer",
  "active": true
}
```

failed response

```json
{
  "active": false
}
```


expect to fix:

would like to fix the impelmentation `fn introspect_token` in the file `server/src/keycloak_client/mod.rs` to support token introspection

also make sure the CLIENT_ID and CLIENT_SECRET will be add to the config

## Todo List

- [x] Add CLIENT_ID and CLIENT_SECRET fields to KeycloakConfig
- [x] Update KeycloakClient to store client_id and client_secret
- [x] Fix introspect_token implementation to use HTTP Basic Auth with client credentials
- [x] Update config.yaml example with new client credentials
- [x] Test compilation and verify all changes work together

## Implementation Summary

### Changes Made

1. **Core KeycloakConfig** ([mpc-backend-mock/core/src/config.rs](mpc-backend-mock/core/src/config.rs))
   - Added `client_id: String` field
   - Added `client_secret: String` field
   - These fields are used for token introspection authentication

2. **Bin KeycloakConfig** ([mpc-backend-mock/bin/src/config/keycloak.rs](mpc-backend-mock/bin/src/config/keycloak.rs))
   - Already had `client_id` and `client_secret` fields defined
   - Default values: `client_id: "mpc-backend"`, `client_secret: "changeme"`

3. **Config Conversion** ([mpc-backend-mock/bin/src/config/mod.rs:137-153](mpc-backend-mock/bin/src/config/mod.rs#L137-L153))
   - Updated conversion from bin config to core config to include `client_id` and `client_secret`

4. **KeycloakClient** ([mpc-backend-mock/server/src/keycloak_client/mod.rs](mpc-backend-mock/server/src/keycloak_client/mod.rs))
   - Added `client_id` and `client_secret` fields to struct
   - Updated constructor to initialize these fields from config
   - **Fixed `introspect_token()` method** (lines 271-305):
     - Removed complex admin-cli token acquisition flow
     - Now uses HTTP Basic Auth with `client_id` and `client_secret` directly
     - Simplified implementation matches the curl example in this document

5. **config.yaml**
   - Added `client_id: "mpc-backend-service"`
   - Added `client_secret: "rlojUqcDXfDTtbpy3RLACzAlKlVcdJmw"`

6. **Test Files Updated**
   - [user_api_test.rs](mpc-backend-mock/server/tests/user_api_test.rs): Added new fields to KeycloakConfig initialization
   - [jwt_auth_test.rs](mpc-backend-mock/server/tests/jwt_auth_test.rs): Added new fields to KeycloakConfig initialization
   - Updated ServiceState::new calls to include new parameters

### Key Implementation Details

The new `introspect_token` method:
- Uses HTTP Basic Authentication with `client_id` and `client_secret` via `.basic_auth()`
- Calls `POST /realms/{realm}/protocol/openid-connect/token/introspect`
- Sends the token as form-urlencoded data: `[("token", token)]`
- Returns `TokenIntrospectionResponse` with `active` status and token metadata
- Simplified from ~60 lines to ~35 lines by removing unnecessary token acquisition

This matches the RFC 7662 Token Introspection standard and the Keycloak API specification exactly as shown in the curl example.

### Verification

✅ **Build**: `cargo build --bin mpc-backend-mock` - Success
✅ **Clippy**: `cargo clippy --workspace --all-targets` - Passing (warnings only, no errors)
✅ **Tests**: Integration tests compile successfully (require running database to execute)
