use std::fmt;

use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use google_cloud_token::TokenSourceProvider;
use snafu::ResultExt;

use crate::kms_client::{
    error,
    error::{Error, Result},
    KeyManagementServiceClient,
};

const SCOPES: [&str; 1] = ["https://www.googleapis.com/auth/cloud-platform"];

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: http::Uri,
    token: String,
    project_id: String,
    location: String,
    key_ring: String,
    crypto_key: String,
}

impl Client {
    /// Create a new client with the default http client.
    /// # Errors
    pub async fn new(
        project_id: String,
        location: String,
        key_ring: String,
        crypto_key: String,
    ) -> Result<Self> {
        Self::with_http_client(reqwest::Client::new(), project_id, location, key_ring, crypto_key)
            .await
    }

    /// Create a new client with the provided http client.
    /// # Errors
    pub async fn with_http_client(
        http_client: reqwest::Client,
        project_id: String,
        location: String,
        key_ring: String,
        crypto_key: String,
    ) -> Result<Self> {
        let ts = google_cloud_auth::token::DefaultTokenSourceProvider::new(Self::auth_config())
            .await
            .context(error::InitializeTokenSourceSnafu)?
            .token_source();

        Ok(Self {
            http: http_client,
            endpoint: "https://cloudkms.googleapis.com".parse().expect("valid uri"),
            token: ts.token().await.expect("valid token"),
            project_id,
            location,
            key_ring,
            crypto_key,
        })
    }

    /// # Errors
    async fn request(
        &self,
        method: http::Method,
        path: impl fmt::Display,
        params: &[(&str, String)],
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let path = path.to_string();

        let mut reqeust_builder = match method {
            http::Method::POST => {
                let url = http::uri::Builder::from(self.endpoint.clone())
                    .path_and_query(&path)
                    .build()
                    .expect("valid url")
                    .to_string();

                if let Some(ref body) = body {
                    self.http.post(url).json(body)
                } else {
                    self.http.post(url).form(params)
                }
            }
            _ => unreachable!("unsupported http method"),
        };

        reqeust_builder = reqeust_builder.header("Authorization", self.token.clone());

        let maybe_response =
            reqeust_builder.send().await.with_context(|_| error::UnexpectedHttpResponseSnafu)?;

        let status = maybe_response.status();

        if status.is_client_error() {
            return Err(Error::OtherClientSide {
                status: maybe_response.status(),
                location: snafu::location!(),
                message: maybe_response.text().await.unwrap_or_default(),
            });
        }

        if status.is_server_error() {
            return Err(Error::ServerSide {
                status: maybe_response.status(),
                location: snafu::location!(),
                message: maybe_response.text().await.unwrap_or_default(),
            });
        }

        let body =
            maybe_response.text().await.with_context(|_| error::UnexpectedHttpResponseSnafu)?;
        serde_json::from_str(&body).with_context(|_| error::DeserializeJsonResponseSnafu {})
    }

    #[inline]
    fn auth_config() -> google_cloud_auth::project::Config<'static> {
        google_cloud_auth::project::Config::default().with_scopes(&SCOPES)
    }
}

#[allow(unused)]
#[async_trait]
impl KeyManagementServiceClient for Client {
    async fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>> {
        let url = format!(
            "/v1/projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}:decrypt",
            self.project_id, self.location, self.key_ring, self.crypto_key
        );

        let opt = serde_json::json!({
            "ciphertext": ciphertext,
        });

        let response = self
            .request(http::Method::POST, url, &[("alt", "json".to_string())], Some(opt))
            .await?;

        let Some(plaintext) = response.get("plaintext").and_then(|val| val.as_str()) else {
            return Err(error::UnexpectedJsonResponseSnafu {
                operation: "cannot parse plaintext".to_string(),
                response,
            }
            .build());
        };

        BASE64_STANDARD.decode(plaintext).context(error::Basse64DecodeSnafu)
    }
}
