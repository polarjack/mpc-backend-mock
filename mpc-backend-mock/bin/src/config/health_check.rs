use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HealthCheckConfig {
    #[serde(default = "HealthCheckConfig::default_host")]
    pub host: IpAddr,

    #[serde(default = "HealthCheckConfig::default_port")]
    pub port: u16,
}

impl HealthCheckConfig {
    #[inline]
    pub const fn socket_address(&self) -> SocketAddr { SocketAddr::new(self.host, self.port) }

    #[inline]
    pub const fn default_host() -> IpAddr { mpc_backend_mock_core::DEFAULT_HEALTH_CHECK_HOST }

    #[inline]
    pub const fn default_port() -> u16 { mpc_backend_mock_core::DEFAULT_HEALTH_CHECK_PORT }
}

impl Default for HealthCheckConfig {
    fn default() -> Self { Self { host: Self::default_host(), port: Self::default_port() } }
}
