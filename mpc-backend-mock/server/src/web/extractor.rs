use std::result::Result;

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
};

use crate::web::{controller::Error, middleware::AuthUser as AuthUserData};

/// Extractor for the `userId` header.
///
/// This extractor retrieves the `userId` header from the request and logs it.
/// It's optional - if the header is missing or invalid, it will be `None`.
///
/// # Example
///
/// ```rust
/// pub async fn handler(
///     UserId(user_id): UserId,
///     // ... other parameters
/// ) -> Result<...> {
///     if let Some(user_id) = user_id {
///         tracing::info!("Request made by user ID: {}", user_id);
///     }
///     // ... handler logic
/// }
/// ```
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UserId(pub Option<String>);

#[async_trait]
impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user_id = parts
            .headers
            .get("User-Id")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.trim().to_string());

        if let Some(ref user_id) = user_id {
            tracing::info!("Request made by user ID: {}", user_id);
        } else {
            tracing::warn!("No user ID found in request headers");
        }

        Ok(Self(user_id))
    }
}

/// Custom query extractor that converts Axum's rejection into our custom error
/// type
#[allow(dead_code)]
#[derive(Debug)]
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state).await.map(|Query(value)| Self(value)).map_err(
            |rejection| {
                // Extract the error message from the rejection
                let error_msg = rejection.body_text();
                // Check if it's a date format error or currency type error
                // Extract the date string if possible
                Error::Model {
                    source: mpc_backend_mock_core::error::Error::InvalidDateFormat {
                        date_str: error_msg,
                    },
                }
            },
        )
    }
}

/// Extractor for authenticated user information
///
/// This extractor retrieves the `AuthUser` data that was inserted by the JWT
/// middleware. It will return a 401 Unauthorized error if the user is not
/// authenticated.
///
/// # Example
///
/// ```rust
/// pub async fn protected_handler(
///     AuthUser(user): AuthUser,
///     // ... other parameters
/// ) -> Result<...> {
///     tracing::info!("Authenticated user: {}", user.keycloak_user_id);
///     // ... handler logic
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser(pub AuthUserData);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = parts
            .extensions
            .get::<AuthUserData>()
            .ok_or((StatusCode::UNAUTHORIZED, "Missing authentication"))?
            .clone();

        Ok(Self(auth_user))
    }
}
