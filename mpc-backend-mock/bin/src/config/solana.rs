use serde::{Deserialize, Serialize};
use zpl_rpc_client::Endpoint as SolanaEndpoint;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SolanaConfig {
    pub endpoint: SolanaEndpoint,
}

impl SolanaConfig {
    pub fn mainnet() -> Self { Self { endpoint: SolanaEndpoint::mainnet() } }

    pub fn testnet() -> Self { Self { endpoint: SolanaEndpoint::testnet() } }

    pub fn devnet() -> Self { Self { endpoint: SolanaEndpoint::devnet() } }
}

impl From<SolanaConfig> for mpc_backend_mock_core::config::SolanaConfig {
    fn from(source: SolanaConfig) -> Self { Self { endpoint: source.endpoint } }
}
