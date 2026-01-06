pub mod error;

use keycloak::{
    types::{CredentialRepresentation, UserRepresentation},
    KeycloakAdmin, KeycloakAdminToken,
};
use snafu::ResultExt;
use uuid::Uuid;

use self::error::{
    AuthenticationSnafu, CreateUserSnafu, GetUserSnafu, HealthCheckSnafu, Result, UserNotFoundSnafu,
};
use crate::config::KeycloakConfig;

/// Keycloak client wrapper for user management and authentication
#[allow(dead_code)]
#[derive(Clone)]
pub struct KeycloakClient {
    realm: String,
    client: reqwest::Client,
    server_url: String,
    admin_username: String,
    admin_password: String,
}

impl KeycloakClient {
    /// Create a new Keycloak client
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot be initialized
    #[allow(dead_code)]
    pub async fn new(config: KeycloakConfig) -> Result<Self> {
        let client = if config.verify_ssl {
            reqwest::Client::new()
        } else {
            reqwest::Client::builder().danger_accept_invalid_certs(true).build().map_err(|e| {
                error::Error::InitializeClient {
                    message: format!("Failed to build reqwest client: {e}"),
                }
            })?
        };

        Ok(Self {
            realm: config.realm,
            client,
            server_url: config.server_url,
            admin_username: config.admin_username,
            admin_password: config.admin_password,
        })
    }

    /// Check if Keycloak is healthy and reachable
    ///
    /// # Errors
    ///
    /// Returns an error if Keycloak health check fails
    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<bool> {
        let health_url = format!("{}/health/ready", self.server_url);
        let response = self.client.get(&health_url).send().await.context(HealthCheckSnafu)?;

        Ok(response.status().is_success())
    }

    /// Check if a user exists in Keycloak by email
    ///
    /// # Errors
    ///
    /// Returns an error if the Keycloak API call fails
    #[allow(dead_code)]
    pub async fn user_exists_by_email(&self, email: &str) -> Result<bool> {
        let admin = self.get_admin_client().await?;

        // Search for user by email
        let users = admin
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
            .context(GetUserSnafu)?;

        // Check if any user with exact email match exists
        Ok(users.iter().any(|u| u.email.as_ref().is_some_and(|e| e == email)))
    }

    /// Create a new user in Keycloak
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication with Keycloak fails
    /// - User creation fails
    /// - User retrieval after creation fails
    #[allow(dead_code)]
    pub async fn create_user(&self, email: &str, password: &str) -> Result<Uuid> {
        let admin = self.get_admin_client().await?;

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
        admin.realm_users_post(&self.realm, user).await.context(CreateUserSnafu)?;

        // Retrieve the created user to get the UUID
        let users = admin
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
            .context(GetUserSnafu)?;

        // Find the user with matching email
        let created_user = users
            .iter()
            .find(|u| u.email.as_ref().is_some_and(|e| e == email))
            .ok_or_else(|| UserNotFoundSnafu { user_id: email.to_string() }.build())?;

        // Parse the Keycloak user ID
        let keycloak_id = created_user
            .id
            .as_ref()
            .ok_or_else(|| UserNotFoundSnafu { user_id: email.to_string() }.build())?;

        Uuid::parse_str(keycloak_id)
            .map_err(|_| UserNotFoundSnafu { user_id: email.to_string() }.build())
    }

    /// Get an authenticated admin client
    async fn get_admin_client(&self) -> Result<KeycloakAdmin> {
        // Acquire admin token using username/password authentication
        let admin_token = KeycloakAdminToken::acquire(
            &self.server_url,
            &self.admin_username,
            &self.admin_password,
            &self.client,
        )
        .await
        .context(AuthenticationSnafu)?;

        // Create admin client with the acquired token
        let admin = KeycloakAdmin::new(&self.server_url, admin_token, self.client.clone());

        Ok(admin)
    }
}
