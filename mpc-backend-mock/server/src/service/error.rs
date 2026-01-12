use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use snafu::Snafu;
use zeus_axum::{json_response, response, response::EncapsulatedJsonError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Fail to begin transaction, error: {source}"))]
    BeginTransaction { source: sqlx::Error },

    #[snafu(display("Fail to commit transaction, error: {source}"))]
    CommitTransaction { source: sqlx::Error },

    #[snafu(display("Fail to roll back transaction, error: {source}"))]
    RollBackTransaction { source: sqlx::Error },

    #[snafu(display("Fail to acquire database connection, error: {source}"))]
    AcquireConnection { source: sqlx::Error },

    #[snafu(display("Fail to get Bitcoin claim balance, error: {source}"))]
    GetBitcoinClaimBalance { source: sqlx::Error },

    #[snafu(display("Fail to get Google OAuth provider by account id, error: {source}"))]
    GetGoogleOAuthProviderByAccountId { source: sqlx::Error },

    #[snafu(display("Fail to create Google OAuth provider record, error: {source}"))]
    GoogleOAuthProviderCreate { source: sqlx::Error },

    #[snafu(display("Fail to update Google OAuth provider record, error: {source}"))]
    GoogleOAuthProviderUpdate { source: sqlx::Error },

    #[snafu(display("Missing file field in multipart request"))]
    MissingFileField,

    #[snafu(display("Missing filename in multipart field"))]
    MissingFileName,

    #[snafu(display("Invalid filename format: {filename}"))]
    InvalidFileName { filename: String },

    #[snafu(display("Duplicate file hash: {file_hash}, existing report: {existing_id}"))]
    DuplicateFileHash { file_hash: String, existing_id: String },

    #[snafu(display("Fail to parse multipart request, error: {source}"))]
    ParseMultipart { source: axum::extract::multipart::MultipartError },

    #[snafu(display("Fail to read file data, error: {source}"))]
    ReadFile { source: axum::extract::multipart::MultipartError },

    #[snafu(display("Fail to parse CSV, error: {source}"))]
    ParseCsv { source: csv::Error },

    #[snafu(display("Fail to parse date, error: {source}"))]
    ParseDate { source: chrono::ParseError },

    #[snafu(display("Missing date in CSV row"))]
    MissingDateInCsvRow,

    #[snafu(display("Invalid numeric value in CSV"))]
    InvalidNumericValue,

    #[snafu(display("User not found: {user_id}"))]
    UserNotFound { user_id: uuid::Uuid },

    #[snafu(display("User already exists: {email}"))]
    UserAlreadyExists { email: String },

    #[snafu(display("Fail to insert user, error: {source}"))]
    InsertUser { source: sqlx::Error },

    #[snafu(display("Fail to get user by id, error: {source}"))]
    GetUserById { source: sqlx::Error },

    #[snafu(display("Fail to delete user by id, error: {source}"))]
    DeleteUserById { source: sqlx::Error },

    #[snafu(display("Fail to get user by email, error: {source}"))]
    GetUserByEmail { source: sqlx::Error },

    #[snafu(display("Fail to get user by keycloak id, error: {source}"))]
    GetUserByKeycloakId { source: sqlx::Error },

    #[snafu(display("Invalid email format: {email}"))]
    InvalidEmail { email: String },

    #[snafu(display("Failed to authenticate with Keycloak, error: {source}"))]
    AuthenticateKeycloak { source: keycloak::KeycloakError },

    #[snafu(display("Failed to check if user exists in Keycloak, error: {source}"))]
    GetKeycloakUser { source: keycloak::KeycloakError },

    #[snafu(display("Failed to create user in Keycloak, error: {source}"))]
    CreateKeycloakUser { source: keycloak::KeycloakError },

    #[snafu(display("Failed to delete user in Keycloak, error: {source}"))]
    DeleteKeycloakUser { source: keycloak::KeycloakError },

    #[snafu(display("User already exists in Keycloak: {email}"))]
    UserExistsInKeycloak { email: String },

    #[snafu(display("Failed to retrieve created Keycloak user: {email}"))]
    KeycloakUserNotFound { email: String },
}

#[allow(clippy::match_single_binding)]
impl IntoResponse for Error {
    // SAFETY: allow: high cognitive complexity caused by `tracing` macro
    #[allow(clippy::cognitive_complexity, clippy::single_match_else)]
    fn into_response(self) -> Response {
        match self {
            Self::DuplicateFileHash { .. }
            | Self::UserAlreadyExists { .. }
            | Self::UserExistsInKeycloak { .. } => json_response! {
                reason: self,
                status: StatusCode::CONFLICT,
                error: response::Error {
                    type_: response::ErrorType::Conflict,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::UserNotFound { .. } | Self::KeycloakUserNotFound { .. } => json_response! {
                reason: self,
                status: StatusCode::NOT_FOUND,
                error: response::Error {
                    type_: response::ErrorType::NotFound,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::InvalidEmail { .. } => json_response! {
                reason: self,
                status: StatusCode::BAD_REQUEST,
                error: response::Error {
                    type_: response::ErrorType::BadRequest,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            _ => json_response! {
                reason: self,
                status: StatusCode::INTERNAL_SERVER_ERROR,
                error: response::Error {
                    type_: response::ErrorType::Internal,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
        }
    }
}
