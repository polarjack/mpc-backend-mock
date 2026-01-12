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
                    project_id.clone(),
                    location.clone(),
                    key_ring.clone(),
                    crypto_key.clone(),
                )
                .await
                .context(error::InitializeGcpKmsSnafu)?;

                Ok(Arc::new(client))
            }
        }
    }
}
