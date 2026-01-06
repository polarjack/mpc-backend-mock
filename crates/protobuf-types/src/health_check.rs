mod proto {
    // SAFETY: allow: prost
    #![allow(
        unreachable_pub,
        unused_qualifications,
        unused_results,
        clippy::default_trait_access,
        clippy::derive_partial_eq_without_eq,
        clippy::doc_markdown,
        clippy::future_not_send,
        clippy::large_enum_variant,
        clippy::missing_const_for_fn,
        clippy::missing_errors_doc,
        clippy::must_use_candidate,
        clippy::return_self_not_must_use,
        clippy::similar_names,
        clippy::too_many_lines,
        clippy::use_self,
        clippy::used_underscore_items,
        clippy::wildcard_imports
    )]

    tonic::include_proto!("grpc.health.v1");
}

use std::pin::Pin;

pub use self::proto::{
    health_check_response::ServingStatus as HealthCheckServingStatus,
    health_client::HealthClient,
    health_server::{Health, HealthServer},
    HealthCheckRequest, HealthCheckResponse,
};

pub type WatchStream = Pin<
    Box<dyn futures::Stream<Item = Result<HealthCheckResponse, tonic::Status>> + Send + 'static>,
>;
