use std::fmt::Debug;

use axum::{
    body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncapsulatedResponse<T, M = ()> {
    #[serde(rename = "_status", with = "http_serde::status_code")]
    status_code: StatusCode,

    #[serde(rename = "_metadata", skip_serializing_if = "Option::is_none")]
    metadata: Option<M>,

    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl<T> From<(StatusCode, T)> for EncapsulatedResponse<T, ()> {
    #[inline]
    fn from((status_code, data): (StatusCode, T)) -> Self {
        Self { status_code, metadata: None, data: Some(data) }
    }
}

impl<T> From<(StatusCode, axum::Json<T>)> for EncapsulatedResponse<T, ()> {
    #[inline]
    fn from((status_code, axum::Json(data)): (StatusCode, axum::Json<T>)) -> Self {
        Self { status_code, metadata: None, data: Some(data) }
    }
}

impl From<StatusCode> for EncapsulatedResponse<(), ()> {
    #[inline]
    fn from(status_code: StatusCode) -> Self { Self { status_code, metadata: None, data: None } }
}

impl<T, M> From<Option<T>> for EncapsulatedResponse<T, M> {
    #[inline]
    fn from(data: Option<T>) -> Self { Self { status_code: StatusCode::OK, metadata: None, data } }
}

impl<T, M> EncapsulatedResponse<T, M> {
    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn ok(data: T) -> Self {
        Self { status_code: StatusCode::OK, metadata: None, data: Some(data) }
    }

    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { status_code: StatusCode::OK, metadata: None, data: None }
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool { self.data.is_none() }

    #[inline]
    #[must_use]
    pub const fn status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn metadata(mut self, metadata: M) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncapsulatedResponseError<E, M = ()> {
    #[serde(rename = "_status", with = "http_serde::status_code")]
    status_code: StatusCode,

    #[serde(rename = "_metadata", skip_serializing_if = "Option::is_none")]
    metadata: Option<M>,

    error: Option<E>,
}

impl<E> From<(StatusCode, E)> for EncapsulatedResponseError<E, ()> {
    #[inline]
    fn from((status_code, error): (StatusCode, E)) -> Self {
        Self { status_code, metadata: None, error: Some(error) }
    }
}

impl<E> From<(StatusCode, axum::Json<E>)> for EncapsulatedResponseError<E, ()> {
    #[inline]
    fn from((status_code, axum::Json(error)): (StatusCode, axum::Json<E>)) -> Self {
        Self { status_code, metadata: None, error: Some(error) }
    }
}

impl From<StatusCode> for EncapsulatedResponseError<()> {
    #[inline]
    fn from(status_code: StatusCode) -> Self { Self { status_code, metadata: None, error: None } }
}

impl<E, M> From<Option<E>> for EncapsulatedResponseError<E, M> {
    #[inline]
    fn from(error: Option<E>) -> Self {
        Self { status_code: StatusCode::INTERNAL_SERVER_ERROR, metadata: None, error }
    }
}

impl<E, M> EncapsulatedResponseError<E, M> {
    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn err(error: E) -> Self {
        Self { status_code: StatusCode::INTERNAL_SERVER_ERROR, metadata: None, error: Some(error) }
    }

    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { status_code: StatusCode::OK, metadata: None, error: None }
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool { self.error.is_none() }

    #[inline]
    #[must_use]
    pub const fn status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn metadata(mut self, metadata: M) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncapsulatedJson<T, M = ()>(pub EncapsulatedResponse<T, M>);

impl<T, M> EncapsulatedJson<T, M> {
    #[inline]
    #[must_use]
    pub fn ok(data: T) -> Self { Self(EncapsulatedResponse::ok(data)) }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn data(self) -> Option<T> { self.0.data }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncapsulatedJsonError<T, M = ()>(pub EncapsulatedResponseError<T, M>);

impl<T, M> EncapsulatedJsonError<T, M> {
    #[inline]
    #[must_use]
    pub fn err(error: T) -> Self { Self(EncapsulatedResponseError::err(error)) }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn error(self) -> Option<T> { self.0.error }

    #[inline]
    #[must_use]
    pub const fn status_code(mut self, status_code: StatusCode) -> Self {
        self.0.status_code = status_code;
        self
    }
}

impl<T> From<(StatusCode, T)> for EncapsulatedJson<T, ()> {
    #[inline]
    fn from((status_code, data): (StatusCode, T)) -> Self { Self((status_code, data).into()) }
}

impl<T> From<(StatusCode, axum::Json<T>)> for EncapsulatedJson<T, ()> {
    #[inline]
    fn from((status_code, axum::Json(data)): (StatusCode, axum::Json<T>)) -> Self {
        Self((status_code, data).into())
    }
}

impl From<StatusCode> for EncapsulatedJson<(), ()> {
    #[inline]
    fn from(status_code: StatusCode) -> Self { Self(status_code.into()) }
}

impl<T, M> From<Option<T>> for EncapsulatedJson<T, M> {
    #[inline]
    fn from(data: Option<T>) -> Self { Self(data.into()) }
}

impl<T, M> EncapsulatedJson<T, M> {
    #[inline]
    #[must_use]
    pub const fn empty() -> Self { Self(EncapsulatedResponse::empty()) }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool { self.0.is_empty() }

    #[inline]
    #[must_use]
    pub const fn status_code(mut self, status_code: StatusCode) -> Self {
        self.0.status_code = status_code;
        self
    }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn metadata(mut self, metadata: M) -> Self {
        self.0.metadata = Some(metadata);
        self
    }
}

impl<T, M> IntoResponse for EncapsulatedJson<T, M>
where
    T: Serialize,
    M: Serialize,
{
    fn into_response(self) -> Response {
        let body = body::Body::from(serde_json::to_vec(&self.0).expect("self.0 is serializable"));

        Response::builder()
            .status(self.0.status_code)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(body)
            .expect("Build `Axum` response successfully; qed")
    }
}

impl<E, M> IntoResponse for EncapsulatedJsonError<E, M>
where
    E: Debug + Serialize,
    M: Serialize,
{
    fn into_response(self) -> Response {
        let body = body::Body::from(serde_json::to_vec(&self.0).expect("self.0 is serializable"));

        let EncapsulatedResponseError { status_code, .. } = self.0;

        Response::builder()
            .status(status_code)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(body)
            .expect("Build `Axum` response successfully; qed")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Error {
    #[serde(rename = "type")]
    pub type_: ErrorType,

    pub message: String,

    #[serde(flatten, skip_serializing_if = "IndexMap::is_empty")]
    pub additional_fields: IndexMap<String, serde_json::Value>,
}

#[macro_export]
macro_rules! json_response {
    (status: $status:expr,data: $data:expr) => {
        EncapsulatedJson::<_>::ok($data).status_code($status).into_response()
    };

    (status: $status:expr,data: $data:expr,metadata: $metadata:expr) => {
        EncapsulatedJson::ok($data).status_code($status).metadata($metadata).into_response()
    };

    (status: $status:expr,error: $error:expr) => {
        EncapsulatedJsonError::<_>::err($error).status_code($status).into_response()
    };

    (status: $status:expr,error: $error:expr,metadata: $metadata:expr) => {
        EncapsulatedJsonError::err($error).status_code($status).metadata($metadata).into_response()
    };

    (reason: $reason:expr,status: $status:expr,error: $error:expr) => {{
        if $status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(status_code = $status.as_u16(), "Internal Server Error: {:?}", $reason);
        } else {
            tracing::warn!(status_code = $status.as_u16(), "Response Error: {:?}", $reason);
        }

        json_response! { status: $status, error: $error }
    }};

    (reason: $reason:expr,status: $status:expr,error: $error:expr,metadata: $metadata:expr) => {{
        if $status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(status_code = $status.as_u16(), "Internal Server Error: {:?}", $reason);
        } else {
            tracing::warn!(status_code = $status.as_u16(), "Response Error: {:?}", $reason);
        }

        json_response! { status: $status, error: $error, metadata: $metadata }
    }};
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorType {
    Unknown,
    Internal,
    Validation,
    Execution,
    NotComplete,
    NotFound,
    Unauthorized,
    BadRequest,
    TooManyRequests,
    Conflict,
    UnprocessableEntity,
    UnavailableForLegalReasons,
}
