use std::process;

use chrono::Utc;
use mpc_backend_mock_core::{ServerInfo, PROGRAM_NAME};
use snafu::ResultExt;
use tokio::runtime::Runtime;

use crate::{
    config::{load_server_config, Config},
    error,
    error::{Error, Result},
    shadow::{BRANCH, PKG_VERSION, SHORT_COMMIT},
};

/// Run the server
#[allow(clippy::cognitive_complexity, clippy::result_large_err)]
pub fn run_server(config: Config) -> Result<()> {
    let Config { ref log, ref bitcoin, ref solana, .. } = config;

    log.registry();

    let server_info = ServerInfo {
        version: PKG_VERSION.to_string(),
        commit_hash: SHORT_COMMIT.to_string(),
        bitcoin_network: bitcoin.network.clone(),
        branch: BRANCH.to_string(),
        solana_cluster: solana.endpoint.cluster.to_string(),
        start_time: Utc::now(),
    };

    tracing::info!("{PROGRAM_NAME} is initializing, pid: {}", process::id());
    tracing::info!("Server info: {server_info:?}");

    tracing::info!("Initializing Tokio runtime");

    let exit_status = match Runtime::new().context(error::InitializeTokioRuntimeSnafu) {
        Ok(runtime) => runtime.block_on({
            async move {
                let config = load_server_config(config).await?;

                mpc_backend_mock_server::serve_with_shutdown(config, server_info)
                    .await
                    .map_err(Error::from)
            }
        }),

        Err(err) => Err(err),
    };

    if let Err(ref error) = exit_status {
        tracing::error!(%error);
    }

    tracing::info!("{PROGRAM_NAME} is shutdown");
    exit_status
}
