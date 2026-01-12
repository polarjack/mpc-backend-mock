use axum::{
    extract::{Query, State},
    Json,
};
use zeus_axum::response::EncapsulatedJson;

use crate::{
    entity::{CreateUserRequest, CreateUserResponse, DeleteUserParams, User, UserInfo},
    web::{controller::Result, extractor::AuthUser as AuthUserExtractor},
    ServiceState,
};

/// Create a new user
///
/// This endpoint creates a new user in both Keycloak and the database.
/// The user is first created in Keycloak, and upon success, a corresponding
/// record is created in the database with the Keycloak user ID.
#[utoipa::path(
    post,
    operation_id = "create_user",
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "User created successfully", body = CreateUserResponse),
        (status = 400, description = "Invalid request (e.g., invalid email format)"),
        (status = 409, description = "User already exists (in database or Keycloak)")
    ),
    tag = "Users"
)]
pub async fn create_user(
    State(state): State<ServiceState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<EncapsulatedJson<CreateUserResponse>> {
    // Create user in Keycloak and database
    let user = state.user_management_service.create_user(&request.email).await?;

    Ok(EncapsulatedJson::ok(CreateUserResponse { user }))
}

/// Get current user information
///
/// This endpoint returns information about the currently authenticated user.
/// It requires a valid JWT token in the Authorization header.
#[utoipa::path(
    get,
    operation_id = "get_current_user",
    path = "/api/v1/users/me",
    responses(
        (status = 200, description = "User information retrieved successfully", body = UserInfo),
        (status = 401, description = "Unauthorized - missing or invalid token"),
        (status = 404, description = "User not found in database")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn get_current_user(
    State(state): State<ServiceState>,
    AuthUserExtractor(auth_user): AuthUserExtractor,
) -> Result<EncapsulatedJson<UserInfo>> {
    // Get user from database using the Keycloak user ID from the JWT token
    let user: User =
        state.user_management_service.get_user_by_keycloak_id(&auth_user.keycloak_user_id).await?;

    // Combine database user with Keycloak info from the token
    let user_info = UserInfo {
        user,
        username: auth_user.username,
        email_verified: Some(auth_user.email_verified),
    };

    Ok(EncapsulatedJson::ok(user_info))
}

/// Delete a user by email (for testing purposes only)
// sample path /api/v1/users?email={email}
#[utoipa::path(
    delete,
    operation_id = "delete_user",
    path = "/api/v1/users",
    params(
        ("email" = String, Path, description = "Email of the user to delete")
    ),
    responses(
        (status = 200, description = "User deleted successfully", body = ()),
        (status = 400, description = "Invalid request (e.g., invalid email format)"),
        (status = 404, description = "User not found in database")
    ),
    tag = "Users"
)]
pub async fn delete_user(
    State(state): State<ServiceState>,
    Query(params): Query<DeleteUserParams>,
) -> Result<EncapsulatedJson<String>> {
    // Delete user in Keycloak and database
    let delete_user_id = state.user_management_service.delete_user_by_email(&params.email).await?;

    Ok(EncapsulatedJson::ok(delete_user_id.to_string()))
}
