use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use snafu::Snafu;
use zeus_axum::{json_response, response, response::EncapsulatedJsonError};

use crate::service::error::Error as ServiceError;

pub type Result<T> = std::result::Result<T, Error>;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{source}"))]
    Service { source: ServiceError },

    #[snafu(display("{source}"))]
    Model { source: mpc_backend_mock_core::error::Error },

    #[snafu(display("Not allowlist solana address: {}", solana_address))]
    NotInAllowlist { solana_address: String },

    #[snafu(display("IP claim limit exceeded"))]
    IPClaimLimitExceeded,

    #[snafu(display("Bitcoin address claim limit exceeded"))]
    BitcoinAddressClaimLimitExceeded,

    #[snafu(display("Invalid Bitcoin address`{address}`, error: {source}"))]
    InvalidBitcoinAddress { address: String, source: eris_bitcoin_ext::AddressError },

    #[snafu(display("Invalid Solana address `{address}`, error: {source}"))]
    InvalidSolanaAddress { address: String, source: solana_sdk::pubkey::ParsePubkeyError },

    #[snafu(display("Invalid last received time `{last_received_time}`"))]
    InvalidLastReceivedTime { last_received_time: i64 },

    #[snafu(display("Simple user creation failed"))]
    UserCreationFailed { username: String },

    #[snafu(display("Simple user: {username} sign in failed"))]
    SignInFailed { username: String },

    #[snafu(display("User not found: {email}"))]
    UserNotFound { email: String },

    #[snafu(display("User already exists: {email}"))]
    UserAlreadyExists { email: String },

    #[snafu(display("Invalid date format: '{}'. Expected YYYY-MM-DD", date_str))]
    InvalidDateFormat { date_str: String },
}

impl From<ServiceError> for Error {
    fn from(source: ServiceError) -> Self { Self::Service { source } }
}

impl From<mpc_backend_mock_core::error::Error> for Error {
    fn from(source: mpc_backend_mock_core::error::Error) -> Self { Self::Model { source } }
}

impl IntoResponse for Error {
    // SAFETY: allow: high cognitive complexity caused by `tracing` macro
    #[allow(clippy::cognitive_complexity)]
    fn into_response(self) -> Response {
        match self {
            Self::Service { source } => source.into_response(),
            Self::Model { source } => source.into_response(),
            Self::NotInAllowlist { .. } => json_response! {
                reason: self,
                status: StatusCode::UNAUTHORIZED,
                error: response::Error {
                    type_: response::ErrorType::Unauthorized,
                    message: "Authorization failed".to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::SignInFailed { .. } => json_response! {
                reason: self,
                status: StatusCode::UNAUTHORIZED,
                error: response::Error {
                    type_: response::ErrorType::Unauthorized,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::UserNotFound { .. } => json_response! {
                reason: self,
                status: StatusCode::NOT_FOUND,
                error: response::Error {
                    type_: response::ErrorType::NotFound,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::UserAlreadyExists { .. } => json_response! {
                reason: self,
                status: StatusCode::CONFLICT,
                error: response::Error {
                    type_: response::ErrorType::Conflict,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
            Self::InvalidBitcoinAddress { .. }
            | Self::InvalidSolanaAddress { .. }
            | Self::InvalidDateFormat { .. } => {
                json_response! {
                    reason: self,
                    status: StatusCode::BAD_REQUEST,
                    error: response::Error {
                        type_: response::ErrorType::BadRequest,
                        message: self.to_string(),
                        additional_fields: IndexMap::default(),
                    }
                }
            }
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
