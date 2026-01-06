use std::time::Duration;

use async_trait::async_trait;
use eris_bitcoin_rpc_client::Client as BitcoinRpcClient;
use sqlx::{Executor, PgPool};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use zeus_protobuf_types::health_check::{
    self as proto, HealthCheckRequest, HealthCheckResponse, HealthCheckServingStatus,
};

#[derive(Clone)]
pub struct HealthCheckService {
    bitcoin_rpc_client: BitcoinRpcClient,

    database: PgPool,
}

impl HealthCheckService {
    #[must_use]
    pub const fn new(bitcoin_rpc_client: BitcoinRpcClient, database: PgPool) -> Self {
        Self { bitcoin_rpc_client, database }
    }

    async fn perform_health_checking(&self) -> Result<(), Box<dyn std::error::Error>> {
        perform_health_checking(&self.bitcoin_rpc_client, &self.database).await
    }
}

#[async_trait]
impl proto::Health for HealthCheckService {
    type WatchStream = ReceiverStream<Result<HealthCheckResponse, Status>>;

    async fn check(
        &self,
        _req: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let status = match self.perform_health_checking().await {
            Ok(()) => HealthCheckServingStatus::Serving,
            Err(err) => {
                tracing::error!("{err}");
                HealthCheckServingStatus::NotServing
            }
        };

        Ok(Response::new(HealthCheckResponse { status: status.into() }))
    }

    async fn watch(
        &self,
        _req: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let (tx, rx) = mpsc::channel(10);

        let bitcoin_rpc_client = self.bitcoin_rpc_client.clone();
        let database = self.database.clone();
        let _unused = tokio::spawn(async move {
            loop {
                let status = match perform_health_checking(&bitcoin_rpc_client, &database).await {
                    Ok(()) => HealthCheckServingStatus::Serving,
                    Err(err) => {
                        tracing::error!("{err}");
                        HealthCheckServingStatus::NotServing
                    }
                };

                if tx.send(Ok(HealthCheckResponse { status: status.into() })).await.is_err() {
                    break;
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn perform_health_checking(
    bitcoin_rpc_client: &BitcoinRpcClient,
    database: &PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!(
        "Checking Bitcoin client with Bitcoin RPC client via endpoint {}",
        &bitcoin_rpc_client.rpc_endpoint()
    );
    let _unused = bitcoin_rpc_client.get_block_count().await?;

    let mut conn = database.acquire().await?;
    let _unused = conn.execute("SELECT 1").await?;

    Ok(())
}
