use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User entity representing a user in the database
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct User {
    /// Unique user ID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    /// User's email address
    #[schema(example = "user@example.com")]
    pub email: String,

    /// Keycloak user ID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440001")]
    pub keycloak_user_id: Uuid,

    /// Whether the user account is active
    #[schema(example = true)]
    pub is_active: bool,

    /// Timestamp when the user was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the user was last updated
    pub updated_at: DateTime<Utc>,

    /// Timestamp when the user was deleted (soft delete)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// User information combining database and Keycloak data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// User data from database
    #[serde(flatten)]
    pub user: User,

    /// Username from Keycloak
    #[schema(example = "user@example.com")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Whether email is verified in Keycloak
    #[schema(example = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
}

/// Request to create a new user
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    /// User's email address
    #[schema(example = "user@example.com")]
    pub email: String,
}

/// alias for delete user query parameters
pub type DeleteUserParams = CreateUserRequest;

/// Response after creating a user
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserResponse {
    /// Created user information
    pub user: User,
}
