pub mod error;
mod server;
mod traits;

pub use self::{error::Error, server::start_metrics_server, traits::Metrics};

#[derive(Clone, Debug)]
pub struct DefaultMetrics {
    registry: prometheus::Registry,
}

impl DefaultMetrics {
    // FIXME: we have to check the result in the near future
    #[allow(clippy::unnecessary_wraps, clippy::missing_errors_doc)]
    pub fn new() -> Result<Self, Error> {
        let registry = prometheus::Registry::new();

        Ok(Self { registry })
    }
}

impl Metrics for DefaultMetrics {
    fn gather(&self) -> Vec<prometheus::proto::MetricFamily> { self.registry.gather() }
}

#[cfg(test)]
mod tests {
    use crate::DefaultMetrics;

    #[test]
    fn test_new() { drop(DefaultMetrics::new().unwrap()); }
}
