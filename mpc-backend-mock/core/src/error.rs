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
    /// Invalid role type provided
    #[snafu(display("Invalid role type: {role}"))]
    InvalidRoleType { role: String },

    #[snafu(display("Invalid date format: '{}'. Expected YYYY-MM-DD", date_str))]
    InvalidDateFormat { date_str: String },
}

impl IntoResponse for Error {
    // SAFETY: allow: high cognitive complexity caused by `tracing` macro
    #[allow(clippy::cognitive_complexity)]
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRoleType { .. } | Self::InvalidDateFormat { .. } => json_response! {
                reason: self,
                status: StatusCode::BAD_REQUEST,
                error: response::Error {
                    type_: response::ErrorType::BadRequest,
                    message: self.to_string(),
                    additional_fields: IndexMap::default(),
                }
            },
        }
    }
}
