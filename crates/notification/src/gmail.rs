//! Gmail API client implementation for sending emails via domain-wide
//! delegation.

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use google_cloud_token::TokenSourceProvider;
use lettre::Message;
use serde::{Deserialize, Serialize};

use crate::{Error, Notification, NotificationClient};

/// Gmail API scopes required for sending emails.
const SCOPES: [&str; 1] = ["https://www.googleapis.com/auth/gmail.send"];

/// Configuration for the Gmail client.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Google Workspace user to impersonate for domain-wide delegation.
    /// This will also be used as the sender address.
    pub impersonate_user: String,
}

/// Gmail API client for sending emails.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    token_source: std::sync::Arc<dyn google_cloud_token::TokenSource>,
    from_address: String,
}

impl Client {
    /// Creates a new Gmail client with domain-wide delegation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create token source provider
    /// - Authentication configuration is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// use notification::gmail::{Client, Config};
    ///
    /// # async fn example() -> Result<(), notification::Error> {
    /// let config = Config {
    ///     impersonate_user: "sender@example.com".to_string(),
    /// };
    ///
    /// let client = Client::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: Config) -> Result<Self, Error> {
        tracing::info!(impersonate_user = %config.impersonate_user, "Using domain-wide delegation for Gmail API");

        let auth_config = google_cloud_auth::project::Config::default()
            .with_scopes(&SCOPES)
            .with_sub(&config.impersonate_user);

        let token_source_provider =
            google_cloud_auth::token::DefaultTokenSourceProvider::new(auth_config).await.map_err(
                |e| {
                    tracing::error!(error = ?e, "Failed to create token source provider");
                    Error::CreateMailer
                },
            )?;

        Ok(Self {
            http: reqwest::Client::new(),
            token_source: token_source_provider.token_source(),
            from_address: config.impersonate_user,
        })
    }
}

#[async_trait]
impl NotificationClient for Client {
    async fn send_notification(&self, notification: &Notification) -> Result<(), Error> {
        let Notification::ActivationEmail { to, link } = notification;

        let email = build_activation_email(&self.from_address, to, link)?;
        let encoded_email = URL_SAFE.encode(email.formatted());

        let token = self.token_source.token().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to get access token");
            Error::CreateMailer
        })?;

        let auth_header =
            if token.starts_with("Bearer ") { token.clone() } else { format!("Bearer {token}") };

        let response = self
            .http
            .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "raw": encoded_email }))
            .send()
            .await
            .map_err(|source| Error::HttpRequest { source })?;

        if !response.status().is_success() {
            if let Ok(response_text) = response.text().await {
                tracing::error!("Failed to send email: {response_text}");
            }
            return Err(Error::SendEmail);
        }

        tracing::info!(to = %to, "Successfully sent activation email");
        Ok(())
    }
}

/// Builds an activation email message.
///
/// # Errors
///
/// Returns an error if the email addresses are invalid or the message cannot be
/// built.
fn build_activation_email(from: &str, to: &str, link: &str) -> Result<Message, Error> {
    let body = format!(
        "<h1>Welcome to Zionx!</h1><p>Please click the link below to activate your account:</p><a \
         href=\"{link}\">{link}</a>"
    );

    Message::builder()
        .from(from.parse().map_err(|_| Error::BuildEmail)?)
        .to(to.parse().map_err(|_| Error::BuildEmail)?)
        .subject("Activate your Account")
        .header(lettre::message::header::ContentType::TEXT_HTML)
        .body(body)
        .map_err(|_| Error::BuildEmail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_activation_email() {
        let result = build_activation_email(
            "sender@example.com",
            "recipient@example.com",
            "https://example.com/activate?token=abc123",
        );

        assert!(result.is_ok());
        let message = result.unwrap();
        let formatted = String::from_utf8(message.formatted()).unwrap();

        assert!(formatted.contains("sender@example.com"));
        assert!(formatted.contains("recipient@example.com"));
        assert!(formatted.contains("Activate your Account"));
        assert!(formatted.contains("Welcome to Zionx!"));
        // URL may be encoded in the email, so just check the domain
        assert!(formatted.contains("example.com/activate"));
    }

    #[test]
    fn test_build_activation_email_invalid_from() {
        let result = build_activation_email(
            "invalid-email",
            "recipient@example.com",
            "https://example.com/activate",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_build_activation_email_invalid_to() {
        let result = build_activation_email(
            "sender@example.com",
            "invalid-email",
            "https://example.com/activate",
        );

        assert!(result.is_err());
    }
}
