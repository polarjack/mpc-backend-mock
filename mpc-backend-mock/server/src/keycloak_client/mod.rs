pub mod error;

use keycloak::{
    types::{CredentialRepresentation, UserRepresentation},
    KeycloakAdmin, KeycloakAdminToken,
};
use mpc_backend_mock_core::config::KeycloakConfig;
use snafu::ResultExt;
use uuid::Uuid;

use self::error::{
    AuthenticationSnafu, CreateUserSnafu, GetUserSnafu, HealthCheckSnafu, IntrospectTokenSnafu,
    ParseIntrospectionResponseSnafu, Result, UserNotFoundSnafu,
};

/// Token introspection response from Keycloak
#[derive(Debug, serde::Deserialize)]
pub struct TokenIntrospectionResponse {
    /// Whether the token is active
    pub active: bool,
    /// Token scope
    #[serde(default)]
    pub scope: Option<String>,
    /// Client ID
    #[serde(default)]
    pub client_id: Option<String>,
    /// Username
    #[serde(default)]
    pub username: Option<String>,
    /// Token type
    #[serde(default)]
    pub token_type: Option<String>,
    /// Token expiration timestamp
    #[serde(default)]
    pub exp: Option<i64>,
    /// Token issued at timestamp
    #[serde(default)]
    pub iat: Option<i64>,
    /// Token not before timestamp
    #[serde(default)]
    pub nbf: Option<i64>,
    /// Subject (user ID)
    #[serde(default)]
    pub sub: Option<String>,
    /// Audience
    #[serde(default)]
    pub aud: Option<String>,
    /// Issuer
    #[serde(default)]
    pub iss: Option<String>,
    /// JWT ID
    #[serde(default)]
    pub jti: Option<String>,
}

/// Keycloak client wrapper for user management and authentication
#[derive(Clone)]
pub struct KeycloakClient {
    realm: String,
    client: reqwest::Client,
    server_url: String,
    admin_username: String,
    admin_password: String,
    client_id: String,
    client_secret: String,
}

impl KeycloakClient {
    /// Create a new Keycloak client
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot be initialized
    pub fn new(config: KeycloakConfig) -> Result<Self> {
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
            client_id: config.client_id,
            client_secret: config.client_secret,
        })
    }

    /// Check if Keycloak is healthy and reachable
    ///
    /// # Errors
    ///
    /// Returns an error if Keycloak health check fails
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
        let _create_user_response =
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

    /// Introspect a JWT token to validate it and retrieve token metadata
    ///
    /// This method calls Keycloak's token introspection endpoint to validate a
    /// token and retrieve information about it, including whether it's
    /// active, the subject, expiration time, and other token claims.
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT token to introspect
    ///
    /// # Returns
    ///
    /// Returns a `TokenIntrospectionResponse` containing token metadata and
    /// validation status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication with Keycloak fails
    /// - The introspection request fails
    /// - The response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use mpc_backend_mock::keycloak_client::KeycloakClient;
    /// # use mpc_backend_mock::config::KeycloakConfig;
    /// # let config = KeycloakConfig::default();
    /// let client = KeycloakClient::new(config).await?;
    /// let response = client.introspect_token("eyJhbGciOiJSUzI1NiIsInR5cCI...").await?;
    /// if response.active {
    ///     println!("Token is valid for user: {:?}", response.username);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn introspect_token(&self, token: &str) -> Result<TokenIntrospectionResponse> {
        // Build introspection endpoint URL
        let introspect_url = format!(
            "{}/realms/{}/protocol/openid-connect/token/introspect",
            self.server_url, self.realm
        );

        tracing::info!("Introspecting token at URL: {}", introspect_url);

        // Prepare form data with the token to introspect
        let form_data = [("token", token)];

        // Make POST request to introspection endpoint with HTTP Basic Auth
        // Using client_id and client_secret for authentication
        let response = self
            .client
            .post(&introspect_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&form_data)
            .send()
            .await
            .context(IntrospectTokenSnafu)?;

        // Parse response body
        let response_text = response.text().await.context(IntrospectTokenSnafu)?;

        tracing::info!("Introspection response: {}", response_text);

        // Deserialize JSON response
        let introspection_response: TokenIntrospectionResponse =
            serde_json::from_str(&response_text).context(ParseIntrospectionResponseSnafu)?;

        Ok(introspection_response)
    }
}
