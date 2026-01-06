use std::sync::Arc;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::{
    config::{error, error::Error},
    kms_client,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum KeyManagementService {
    GoogleCloudPlatform {
        project_id: String,
        location: String,
        key_ring: String,
        crypto_key: String,
    },
}

impl KeyManagementService {
    pub async fn load(&self) -> Result<Arc<dyn kms_client::KeyManagementServiceClient>, Error> {
        match self {
            Self::GoogleCloudPlatform { project_id, location, key_ring, crypto_key } => {
                let client = kms_client::gcp::Client::new(
                    project_id.to_string(),
                    location.to_string(),
                    key_ring.to_string(),
                    crypto_key.to_string(),
                )
                .await
                .context(error::InitializeGcpKmsSnafu)?;

                Ok(Arc::new(client))
            }
        }
    }
}
