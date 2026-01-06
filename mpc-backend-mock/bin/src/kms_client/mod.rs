mod error;
pub mod gcp;

use async_trait::async_trait;
pub use error::{Error, Result};

#[allow(unused)]
#[async_trait]
pub trait KeyManagementServiceClient {
    async fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>>;
}
