use std::str::FromStr;

use eris_bitcoin_ext::WellKnownNetwork as BitcoinNetwork;
use eris_bitcoin_rpc_client::Authentication as BitcoinRpcAuthentication;
use serde::{Deserialize, Serialize};
use zpl_bitcoin_spv::constant::BLOCK_CONFIRMATION_COUNT;

use crate::config::error::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitcoinConfig {
    pub network: String,

    #[serde(with = "http_serde::uri")]
    pub rpc_endpoint: http::Uri,

    /// username and password for authentication, separating by `:`, for
    /// example: "USER:PASSWORD"
    pub rpc_authentication: Option<String>,

    #[serde(with = "http_serde::option::uri")]
    pub indexer_endpoint: Option<http::Uri>,

    pub support_quicknode_blockbook: bool,
}

impl TryFrom<BitcoinConfig> for mpc_backend_mock_core::config::BitcoinConfig {
    type Error = Error;

    fn try_from(source: BitcoinConfig) -> Result<Self, Self::Error> {
        let BitcoinConfig {
            network,
            rpc_endpoint,
            rpc_authentication,
            indexer_endpoint,
            support_quicknode_blockbook,
        } = source;
        let network = BitcoinNetwork::from_str(&network)
            .map_err(|_| Error::ParseBitcoinNetwork { value: network })?;
        let authentication = rpc_authentication
            .map(|auth| BitcoinRpcAuthentication::from_str(&auth).unwrap_or_default())
            .unwrap_or_default();

        let block_number_to_confirm = u64::try_from(BLOCK_CONFIRMATION_COUNT).unwrap_or(6);
        Ok(Self {
            endpoint: eris_bitcoin_rpc_client::RpcEndpoint {
                endpoint: rpc_endpoint,
                indexer_endpoint,
                authentication,
                support_quicknode_blockbook,
                network,
            },
            block_number_to_confirm,
        })
    }
}

impl BitcoinConfig {
    pub fn devnet() -> Self {
        Self {
            network: "regtest".to_string(),
            rpc_endpoint: http::Uri::from_static("http://127.0.0.1:18443"),
            rpc_authentication: None,
            indexer_endpoint: Some(http::Uri::from_static("http://127.0.0.1:50001")),
            support_quicknode_blockbook: false,
        }
    }
}
