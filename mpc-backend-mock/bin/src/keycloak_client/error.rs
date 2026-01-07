use snafu::{Location, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to initialize Keycloak client: {message}"))]
    InitializeClient { message: String },

    #[snafu(display("Failed to create Keycloak user: {source}, location: {location}"))]
    CreateUser {
        #[snafu(implicit)]
        location: Location,
        source: keycloak::KeycloakError,
    },

    #[snafu(display("Failed to get Keycloak user: {source}, location: {location}"))]
    GetUser {
        #[snafu(implicit)]
        location: Location,
        source: keycloak::KeycloakError,
    },

    #[snafu(display("User not found in Keycloak: {user_id}, location: {location}"))]
    UserNotFound {
        #[snafu(implicit)]
        location: Location,
        user_id: String,
    },

    #[snafu(display("Failed to validate JWT token: {source}, location: {location}"))]
    ValidateToken {
        #[snafu(implicit)]
        location: Location,
        source: jsonwebtoken::errors::Error,
    },

    #[snafu(display("Failed to fetch Keycloak public key: {source}, location: {location}"))]
    FetchPublicKey {
        #[snafu(implicit)]
        location: Location,
        source: reqwest::Error,
    },

    #[snafu(display("Failed to parse public key: {message}, location: {location}"))]
    ParsePublicKey {
        #[snafu(implicit)]
        location: Location,
        message: String,
    },

    #[snafu(display("Failed to authenticate with Keycloak: {source}, location: {location}"))]
    Authentication {
        #[snafu(implicit)]
        location: Location,
        source: keycloak::KeycloakError,
    },

    #[snafu(display("Failed to check Keycloak health: {source}, location: {location}"))]
    HealthCheck {
        #[snafu(implicit)]
        location: Location,
        source: reqwest::Error,
    },

    #[snafu(display("Invalid email format: {email}, location: {location}"))]
    InvalidEmail {
        #[snafu(implicit)]
        location: Location,
        email: String,
    },

    #[snafu(display("User already exists: {email}, location: {location}"))]
    UserAlreadyExists {
        #[snafu(implicit)]
        location: Location,
        email: String,
    },

    #[snafu(display("Failed to introspect token: {source}, location: {location}"))]
    IntrospectToken {
        #[snafu(implicit)]
        location: Location,
        source: reqwest::Error,
    },

    #[snafu(display("Failed to parse introspection response: {source}, location: {location}"))]
    ParseIntrospectionResponse {
        #[snafu(implicit)]
        location: Location,
        source: serde_json::Error,
    },
}
