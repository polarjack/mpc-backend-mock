use std::{sync::Arc, time::Duration};

use jsonwebtoken::jwk::{Jwk, JwkSet};
use snafu::{ResultExt, Snafu};
use tokio::sync::RwLock;

/// JWKS client for fetching and caching public keys from Keycloak
#[derive(Clone)]
pub struct JwksClient {
    jwks_url: String,
    http_client: reqwest::Client,
    cache: Arc<RwLock<JwksCache>>,
}

/// Cached JWKS data
struct JwksCache {
    jwks: Option<JwkSet>,
    last_fetch: Option<std::time::Instant>,
}

impl JwksClient {
    /// Create a new JWKS client
    ///
    /// # Arguments
    /// * `keycloak_url` - Base Keycloak URL (e.g., <http://localhost:8080>)
    /// * `realm` - Keycloak realm name (e.g., "mpc")
    pub fn new(keycloak_url: &str, realm: &str) -> Result<Self, JwksError> {
        let jwks_url = format!("{keycloak_url}/realms/{realm}/protocol/openid-connect/certs");

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context(HttpClientSnafu)?;

        Ok(Self {
            jwks_url,
            http_client,
            cache: Arc::new(RwLock::new(JwksCache { jwks: None, last_fetch: None })),
        })
    }

    /// Get a JWK by key ID (kid)
    ///
    /// This method will fetch from cache if available and fresh (< 5 minutes
    /// old), otherwise it will fetch fresh JWKS from Keycloak
    pub async fn get_jwk(&self, kid: &str) -> Result<Jwk, JwksError> {
        // Check cache first
        let cache = self.cache.read().await;
        if let Some(ref jwks) = cache.jwks {
            if let Some(last_fetch) = cache.last_fetch {
                // Cache is valid for 5 minutes
                if last_fetch.elapsed() < Duration::from_secs(300) {
                    if let Some(jwk) = jwks.find(kid) {
                        tracing::debug!("Found JWK in cache for kid: {}", kid);
                        return Ok(jwk.clone());
                    }
                }
            }
        }
        drop(cache);

        // Fetch fresh JWKS
        tracing::info!("Fetching fresh JWKS from {}", self.jwks_url);
        let jwks = self.fetch_jwks().await?;

        // Find the key
        let jwk = jwks.find(kid).ok_or(JwksError::KeyNotFound { kid: kid.to_string() })?;
        let jwk_clone = jwk.clone();

        // Update cache
        let mut cache = self.cache.write().await;
        cache.jwks = Some(jwks);
        cache.last_fetch = Some(std::time::Instant::now());
        drop(cache);

        Ok(jwk_clone)
    }

    /// Fetch JWKS from Keycloak
    async fn fetch_jwks(&self) -> Result<JwkSet, JwksError> {
        let response = self.http_client.get(&self.jwks_url).send().await.context(FetchJwksSnafu)?;

        if !response.status().is_success() {
            return Err(JwksError::FetchFailed {
                status: response.status().as_u16(),
                url: self.jwks_url.clone(),
            });
        }

        let jwks: JwkSet = response.json().await.context(ParseJwksSnafu)?;
        tracing::debug!("Fetched JWKS with {} keys", jwks.keys.len());

        Ok(jwks)
    }

    /// Force refresh the JWKS cache
    pub async fn refresh(&self) -> Result<(), JwksError> {
        let jwks = self.fetch_jwks().await?;

        let mut cache = self.cache.write().await;
        cache.jwks = Some(jwks);
        cache.last_fetch = Some(std::time::Instant::now());
        drop(cache);

        Ok(())
    }
}

/// JWKS client errors
#[derive(Debug, Snafu)]
pub enum JwksError {
    /// Failed to create HTTP client
    #[snafu(display("Failed to create HTTP client: {source}"))]
    HttpClient { source: reqwest::Error },

    /// Failed to fetch JWKS
    #[snafu(display("Failed to fetch JWKS: {source}"))]
    FetchJwks { source: reqwest::Error },

    /// Failed to parse JWKS response
    #[snafu(display("Failed to parse JWKS: {source}"))]
    ParseJwks { source: reqwest::Error },

    /// JWKS fetch returned non-success status
    #[snafu(display("JWKS fetch failed with status {status} from {url}"))]
    FetchFailed { status: u16, url: String },

    /// Key not found in JWKS
    #[snafu(display("Key with kid '{kid}' not found in JWKS"))]
    KeyNotFound { kid: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_url_construction() {
        let client =
            JwksClient::new("http://localhost:8080", "mpc").expect("Failed to create client");
        assert_eq!(
            client.jwks_url,
            "http://localhost:8080/realms/mpc/protocol/openid-connect/certs"
        );
    }
}
