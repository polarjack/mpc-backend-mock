use std::borrow::Cow;

use snafu::{Location, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to initialize token source: {source}, location: {location}"))]
    InitializeTokenSource {
        #[snafu(implicit)]
        location: Location,
        source: google_cloud_auth::error::Error,
    },

    #[snafu(display("Unexpected HTTP response: {source}, location: {location}"))]
    UnexpectedHttpResponse {
        #[snafu(implicit)]
        location: Location,
        source: reqwest::Error,
    },

    #[snafu(display(
        "Other client side error: {message}, status code: {status}, location: {location}"
    ))]
    OtherClientSide {
        #[snafu(implicit)]
        location: Location,
        status: reqwest::StatusCode,
        message: String,
    },

    #[snafu(display("Server side error: {message}, status code: {status}, location: {location}"))]
    ServerSide {
        #[snafu(implicit)]
        location: Location,
        status: reqwest::StatusCode,
        message: String,
    },

    #[snafu(display("Unexpected JSON response: {}, operation: {operation}", serde_json::to_string_pretty(response).expect("should be serializable")))]
    UnexpectedJsonResponse {
        #[snafu(implicit)]
        location: Location,
        operation: Cow<'static, str>,
        response: serde_json::Value,
    },

    #[snafu(display(
        "Failed to deserialize JSON from HTTP response, error: {source}, location: {location}"
    ))]
    DeserializeJsonResponse {
        #[snafu(implicit)]
        location: Location,
        source: serde_json::Error,
    },

    #[snafu(display(
        "Failed to deserialize JSON from HTTP response, error: {source}, location: {location}"
    ))]
    Basse64Decode {
        #[snafu(implicit)]
        location: Location,
        source: base64::DecodeError,
    },
}
