use std::sync::Arc;

use keycloak::{
    types::{CredentialRepresentation, UserRepresentation},
    KeycloakAdmin,
};
use snafu::ResultExt;
use sqlx::PgPool;
use uuid::Uuid;

use super::error::{Error, Result};
use crate::{
    entity::User,
    service::{error, sql_executor::UserSqlExecutor},
};

/// User management service for handling user-related operations
#[derive(Clone)]
pub struct UserManagementService {
    db: PgPool,
    keycloak_admin: Arc<KeycloakAdmin>,
    realm: String,
}

impl UserManagementService {
    /// Create a new user management service
    #[inline]
    #[must_use]
    pub const fn new(db: PgPool, keycloak_admin: Arc<KeycloakAdmin>, realm: String) -> Self {
        Self { db, keycloak_admin, realm }
    }

    /// Create a new user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Email is invalid
    /// - User already exists in database
    /// - User already exists in Keycloak
    /// - Keycloak user creation fails
    /// - Database operation fails
    pub async fn create_user(&self, email: &str, password: &str) -> Result<User> {
        // Validate email format
        if !Self::is_valid_email(email) {
            return Err(Error::InvalidEmail { email: email.to_string() });
        }

        let mut tx = self.db.begin().await.context(error::BeginTransactionSnafu)?;

        // Step 1: Check if user already exists in system database
        let existing_user = tx.get_user_by_email(email).await?;

        if existing_user.is_some() {
            return Err(Error::UserAlreadyExists { email: email.to_string() });
        }

        // Step 2: Check if user exists in Keycloak
        let user_exists_in_keycloak = self.check_user_exists_in_keycloak(email).await?;

        if user_exists_in_keycloak {
            return Err(Error::UserExistsInKeycloak { email: email.to_string() });
        }

        // Step 3: Create user in Keycloak
        let keycloak_user_id = self.create_keycloak_user(email, password).await?;

        // Step 4: Create user in system database with Keycloak user ID
        let user = tx.insert_user(email, &keycloak_user_id, true).await?;

        tx.commit().await.context(error::CommitTransactionSnafu)?;

        Ok(user)
    }

    /// Check if a user exists in Keycloak by email
    async fn check_user_exists_in_keycloak(&self, email: &str) -> Result<bool> {
        // Search for user by email
        let users = self
            .keycloak_admin
            .realm_users_get(
                &self.realm,
                None,
                Some(email.to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .context(error::GetKeycloakUserSnafu)?;

        // Check if any user with exact email match exists
        Ok(users.iter().any(|u| u.email.as_ref().is_some_and(|e| e == email)))
    }

    /// Create a new user in Keycloak
    async fn create_keycloak_user(&self, email: &str, password: &str) -> Result<Uuid> {
        // Create user representation
        let user = UserRepresentation {
            email: Some(email.to_string()),
            username: Some(email.to_string()),
            enabled: Some(true),
            email_verified: Some(false),
            credentials: Some(vec![CredentialRepresentation {
                type_: Some("password".to_string()),
                value: Some(password.to_string()),
                temporary: Some(false),
                ..Default::default()
            }]),
            ..Default::default()
        };

        // Create user in Keycloak
        let _create_user_response = self
            .keycloak_admin
            .realm_users_post(&self.realm, user)
            .await
            .context(error::CreateKeycloakUserSnafu)?;

        // Retrieve the created user to get the UUID
        let users = self
            .keycloak_admin
            .realm_users_get(
                &self.realm,
                None,
                Some(email.to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .context(error::GetKeycloakUserSnafu)?;

        // Find the user with matching email
        let created_user = users
            .iter()
            .find(|u| u.email.as_ref().is_some_and(|e| e == email))
            .ok_or_else(|| Error::KeycloakUserNotFound { email: email.to_string() })?;

        // Parse the Keycloak user ID
        let keycloak_id = created_user
            .id
            .as_ref()
            .ok_or_else(|| Error::KeycloakUserNotFound { email: email.to_string() })?;

        Uuid::parse_str(keycloak_id)
            .map_err(|_| Error::KeycloakUserNotFound { email: email.to_string() })
    }

    /// Get user by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User not found
    /// - Database operation fails
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User> {
        let mut conn = self.db.acquire().await.context(error::AcquireConnectionSnafu)?;

        let user = conn.get_user_by_id(&user_id).await?.ok_or(Error::UserNotFound { user_id })?;

        Ok(user)
    }

    /// Get user by email
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User not found
    /// - Database operation fails
    pub async fn get_user_by_email(&self, email: String) -> Result<User> {
        let mut conn = self.db.acquire().await.context(error::AcquireConnectionSnafu)?;

        let user = conn.get_user_by_email(&email).await?.ok_or(Error::UserNotFound {
            user_id: Uuid::nil(), // Using nil UUID since we don't have the ID
        })?;

        Ok(user)
    }

    /// Get user by Keycloak user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User not found
    /// - Database operation fails
    pub async fn get_user_by_keycloak_id(&self, keycloak_user_id: &Uuid) -> Result<User> {
        let mut conn = self.db.acquire().await.context(error::AcquireConnectionSnafu)?;

        let user = conn
            .get_user_by_keycloak_id(keycloak_user_id)
            .await?
            .ok_or(Error::UserNotFound { user_id: *keycloak_user_id })?;

        Ok(user)
    }

    /// Validate email format
    fn is_valid_email(email: &str) -> bool {
        // Basic email validation
        email.contains('@') && email.contains('.') && email.len() > 3
    }
}
