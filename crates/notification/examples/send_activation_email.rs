//! Example: Send an activation email using Gmail API with domain-wide
//! delegation.
//!
//! # Prerequisites
//!
//! 1. Set up Google Workspace service account with domain-wide delegation
//! 2. Set `GOOGLE_APPLICATION_CREDENTIALS` environment variable
//! 3. Configure the impersonate user and recipient email
//!
//! # Usage
//!
//! ```bash
//! export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account-key.json"
//! cargo run --example send_activation_email
//! ```

use notification::{
    gmail::{Client, Config},
    Notification, NotificationClient,
};

#[tokio::main]
async fn main() -> Result<(), notification::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting email notification example");

    // Configure the Gmail client
    // Replace with your actual domain email
    let config = Config { impersonate_user: "noreply@yourdomain.com".to_string() };

    tracing::info!("Creating Gmail client with domain-wide delegation");
    let client = Client::new(config).await?;

    // Create an activation email notification
    let notification = Notification::ActivationEmail {
        to: "user@example.com".to_string(),
        link: "https://yourdomain.com/activate?token=abc123def456".to_string(),
    };

    tracing::info!("Sending activation email");
    client.send_notification(&notification).await?;

    tracing::info!("✓ Email sent successfully!");
    Ok(())
}
