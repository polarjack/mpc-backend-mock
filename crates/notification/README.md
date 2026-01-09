# Notification Crate

A Rust library for sending notifications via various channels. Currently supports email notifications via Gmail API with domain-wide delegation.

## Features

- **Gmail API Integration**: Send emails using Google's Gmail API
- **Domain-Wide Delegation**: Impersonate users in a Google Workspace domain
- **HTML Email Support**: Send rich HTML emails
- **Async/Await**: Built with Tokio for async operations
- **Type-Safe**: Strongly typed notification system

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
notification = { path = "crates/notification" }
```

## Usage

### Basic Example

```rust
use notification::{Notification, NotificationClient};
use notification::gmail::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), notification::Error> {
    // Configure the Gmail client
    let config = Config {
        impersonate_user: "noreply@example.com".to_string(),
    };

    // Create the client
    let client = Client::new(config).await?;

    // Send an activation email
    let notification = Notification::ActivationEmail {
        to: "user@example.com".to_string(),
        link: "https://example.com/activate?token=abc123".to_string(),
    };

    client.send_notification(&notification).await?;
    println!("Email sent successfully!");

    Ok(())
}
```

## Google Workspace Setup

To use the Gmail API with domain-wide delegation, you need to set up a service account and configure your Google Workspace.

### Prerequisites

- Google Workspace (formerly G Suite) account with admin access
- A domain in Google Workspace

### Step 1: Create a Google Cloud Project

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Note your Project ID

### Step 2: Enable Gmail API

1. In the Cloud Console, go to **APIs & Services** > **Library**
2. Search for "Gmail API"
3. Click **Enable**

### Step 3: Create a Service Account

1. Go to **APIs & Services** > **Credentials**
2. Click **Create Credentials** > **Service Account**
3. Fill in the service account details:
   - **Name**: `email-notification-service`
   - **Description**: "Service account for sending email notifications"
4. Click **Create and Continue**
5. Skip granting roles (optional for basic usage)
6. Click **Done**

### Step 4: Create Service Account Key

1. In the **Service Accounts** list, click on your newly created service account
2. Go to the **Keys** tab
3. Click **Add Key** > **Create New Key**
4. Choose **JSON** format
5. Click **Create**
6. Save the downloaded JSON file securely (e.g., `service-account-key.json`)
7. **Important**: Keep this file secure and never commit it to version control

### Step 5: Note the Client ID

1. In the service account details page, find the **Unique ID** (also called Client ID)
2. Copy this ID - you'll need it for domain-wide delegation
3. It looks like: `1234567890123456789`

### Step 6: Enable Domain-Wide Delegation

1. In the service account details page, check **Enable Google Workspace Domain-wide Delegation**
2. Click **Save**
3. Optionally add a **Product name** (e.g., "Email Notification Service")

### Step 7: Configure Domain-Wide Delegation in Admin Console

1. Go to [Google Workspace Admin Console](https://admin.google.com/)
2. Navigate to **Security** > **Access and data control** > **API Controls**
3. Click **Manage Domain-Wide Delegation**
4. Click **Add new**
5. Enter the following:
   - **Client ID**: The unique ID from Step 5
   - **OAuth Scopes**: `https://www.googleapis.com/auth/gmail.send`
6. Click **Authorize**

### Step 8: Set Up Application Default Credentials

Set the environment variable to point to your service account key:

```bash
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account-key.json"
```

For production, you can also use:

- Google Cloud Run: Automatically uses the service account
- Google Compute Engine: Attach service account to instances
- Google Kubernetes Engine: Use Workload Identity

### Step 9: Configure the Notification Client

```rust
use notification::gmail::{Client, Config};

let config = Config {
    // Use a real email address from your domain
    impersonate_user: "noreply@yourdomain.com".to_string(),
};

let client = Client::new(config).await?;
```

## Required IAM Roles and Permissions

### Service Account Permissions

The service account needs:

1. **Gmail API Access**: Enabled via domain-wide delegation
2. **OAuth Scope**: `https://www.googleapis.com/auth/gmail.send`

### Google Workspace Admin Requirements

The admin user configuring domain-wide delegation must have:

- **Super Admin** role in Google Workspace

### Email Address Requirements

The `impersonate_user` email address must:

- Be a valid user in your Google Workspace domain
- Exist in your domain (can be a user account or group)
- Have Gmail enabled

## Security Best Practices

1. **Service Account Key Storage**:
   - Never commit service account keys to version control
   - Use secret management systems (e.g., Google Secret Manager, AWS Secrets Manager)
   - Rotate keys regularly

2. **Minimal Scopes**:
   - Only request the `gmail.send` scope (not full Gmail access)
   - Avoid using `gmail.readonly` or broader scopes

3. **Audit Logging**:
   - Enable Cloud Audit Logs in Google Cloud
   - Monitor service account usage
   - Review domain-wide delegation permissions regularly

4. **Key Rotation**:
   - Rotate service account keys every 90 days
   - Delete old keys after rotation
   - Use key expiration policies

5. **Environment Variables**:
   - Use environment variables for configuration
   - Never hardcode credentials in source code

## Troubleshooting

### "Failed to create token source provider"

- Verify `GOOGLE_APPLICATION_CREDENTIALS` is set correctly
- Check that the service account key file exists and is valid JSON
- Ensure the service account has domain-wide delegation enabled

### "Failed to send email: 403 Forbidden"

- Verify domain-wide delegation is configured in Admin Console
- Check that the OAuth scope `https://www.googleapis.com/auth/gmail.send` is authorized
- Ensure the impersonated user exists in your domain

### "Failed to send email: 400 Bad Request"

- Check that email addresses are valid
- Verify the `from` address matches the impersonated user
- Ensure the email content is properly formatted

### "Invalid grant: Not a valid email"

- The `impersonate_user` must be a real email address in your domain
- The user account must exist and have Gmail enabled
- Check for typos in the email address

## Examples

See the [examples directory](examples/) for more usage examples:

- `send_activation_email.rs` - Send an activation email with a link

Run examples:

```bash
cargo run --example send_activation_email
```

## Architecture

### Components

- **`Notification` enum**: Defines different notification types
- **`NotificationClient` trait**: Interface for sending notifications
- **`gmail::Client`**: Gmail API implementation
- **`Error` enum**: Error types with context

### Authentication Flow

1. Application loads service account credentials from `GOOGLE_APPLICATION_CREDENTIALS`
2. Client creates token source with domain-wide delegation
3. Token source impersonates specified user
4. Client requests access token with Gmail send scope
5. Access token is used to authenticate Gmail API requests

## License

GPL-3.0-only

## Contributing

Contributions are welcome! Please ensure all tests pass before submitting a PR:

```bash
cargo test --package notification
cargo clippy --package notification
cargo fmt --package notification
```
