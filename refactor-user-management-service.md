While in the file `user_management.rs`, keycloak admin client is not setup while the struct is initialized.

Would like to refactor to move the Keycloak admin client initialization OUT of the service layer entirely. The `KeycloakAdmin` client should be initialized at application startup (in `lib.rs`), and then passed as a reference to `UserManagementService::new()`.

## Implementation Plan - COMPLETED ✓

### Summary of Changes

Successfully moved the Keycloak admin client initialization out of the service layer and into the application startup phase. This follows the dependency injection pattern and separates infrastructure concerns from business logic.

### Completed Steps

1. **Read current UserManagementService implementation** ✓
   - Identified on-the-fly client creation in `check_user_exists_in_keycloak()` and `create_keycloak_user()`
   - Located `get_keycloak_admin()` helper method that was creating clients for each operation

2. **Update UserManagementService struct** ✓
   - Removed `keycloak_config: KeycloakConfig` field
   - Added `keycloak_admin: Arc<KeycloakAdmin>` field (wrapped in Arc for Clone support)
   - Added `realm: String` field to store the realm name

3. **Update UserManagementService constructor** ✓
   - Changed signature: `pub fn new(db: PgPool, keycloak_admin: Arc<KeycloakAdmin>, realm: String) -> Self`
   - Made it synchronous (removed `async`)
   - Removed all Keycloak initialization logic

4. **Refactor methods using Keycloak** ✓
   - Updated `check_user_exists_in_keycloak()` to use `self.keycloak_admin` directly
   - Updated `create_keycloak_user()` to use `self.keycloak_admin` directly
   - Removed `get_keycloak_admin()` helper method (no longer needed)
   - Changed all references from `self.keycloak_config.realm` to `self.realm`

5. **Create Keycloak admin client initialization in lib.rs** ✓
   - Added `initialize_keycloak_admin()` async function in [`lib.rs:259-287`](mpc-backend-mock/server/src/lib.rs#L259-L287)
   - Handles HTTP client creation, token acquisition, and admin client setup
   - Added proper error handling with `InitializeKeycloakAdmin` error variant

6. **Update ServiceState** ✓
   - Changed `ServiceState::new()` back to synchronous
   - Updated signature to accept `Arc<KeycloakAdmin>` and `String` (realm)
   - Removed async/await from ServiceState initialization

7. **Update all call sites** ✓
   - Updated [`lib.rs:53`](mpc-backend-mock/server/src/lib.rs#L53) to initialize Keycloak admin with `Arc::new()`
   - Updated [`user_api_test.rs`](mpc-backend-mock/server/tests/user_api_test.rs) to initialize Keycloak admin
   - Updated [`jwt_auth_test.rs`](mpc-backend-mock/server/tests/jwt_auth_test.rs) to initialize Keycloak admin

8. **Test the refactored service** ✓
   - ✅ Build successful: `cargo build --workspace`
   - ✅ Clippy passed: `cargo clippy --workspace --all-targets`
   - All compilation errors resolved
   - Only pre-existing warnings remain (unrelated to this refactoring)

### Technical Details

**Architecture Improvement:**
- **Before:** Service layer created Keycloak admin clients on-demand for each operation
- **After:** Keycloak admin client is initialized once at application startup and injected into services

**Benefits:**
1. **Separation of Concerns:** Infrastructure setup separated from business logic
2. **Performance:** Client reuse eliminates redundant authentication calls
3. **Testability:** Easier to mock the Keycloak admin client in tests
4. **Consistency:** Follows the same pattern as other clients (Bitcoin RPC, ZPL, etc.)
5. **Error Handling:** Startup failures are caught early, not during request handling

**Files Modified:**
- [`user_management.rs`](mpc-backend-mock/server/src/service/user_management.rs) - Service refactored
- [`lib.rs`](mpc-backend-mock/server/src/lib.rs) - Added initialization function
- [`error.rs`](mpc-backend-mock/server/src/error.rs) - Added `InitializeKeycloakAdmin` error
- [`web/mod.rs`](mpc-backend-mock/server/src/web/mod.rs) - Updated ServiceState
- [`user_api_test.rs`](mpc-backend-mock/server/tests/user_api_test.rs) - Updated tests
- [`jwt_auth_test.rs`](mpc-backend-mock/server/tests/jwt_auth_test.rs) - Updated tests
