//! # Notification Crate
//!
//! This crate provides functionality for sending notifications via various
//! channels. Currently supports email notifications via Gmail API with
//! domain-wide delegation.
//!
//! ## Features
//!
//! - Gmail API integration with domain-wide delegation
//! - HTML email support
//! - Activation email templates
//! - Async/await support

mod error;
pub mod gmail;

use async_trait::async_trait;
pub use error::Error;

/// Represents different types of notifications that can be sent.
#[derive(Debug, Clone)]
pub enum Notification {
    /// An activation email with a link for account activation.
    ActivationEmail {
        /// The recipient's email address.
        to: String,
        /// The activation link URL.
        link: String,
    },
}

/// Trait for notification clients that can send notifications.
#[async_trait]
pub trait NotificationClient: Send + Sync {
    /// Sends a notification.
    ///
    /// # Errors
    ///
    /// Returns an error if the notification fails to send.
    async fn send_notification(&self, notification: &Notification) -> Result<(), Error>;
}
