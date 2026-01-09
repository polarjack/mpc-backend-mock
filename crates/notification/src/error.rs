use snafu::Snafu;

/// Errors that can occur in the notification crate.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to create mailer client.
    #[snafu(display("Failed to create mailer client"))]
    CreateMailer,

    /// Failed to build email.
    #[snafu(display("Failed to build email message"))]
    BuildEmail,

    /// Failed to send email.
    #[snafu(display("Failed to send email"))]
    SendEmail,

    /// HTTP request failed.
    #[snafu(display("HTTP request failed: {source}"))]
    HttpRequest {
        /// The underlying reqwest error.
        source: reqwest::Error,
    },
}
