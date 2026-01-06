use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use http::HeaderMap;

pub mod response;

#[must_use]
pub fn get_request_ip(headers: &HeaderMap, addr: &SocketAddr) -> IpAddr {
    let x_forwarded_for = headers
        .get("X-Forwarded-For")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.split(',').next().map(str::trim).and_then(|ip| IpAddr::from_str(ip).ok()));
    let x_real_ip = headers
        .get("X-Real-IP")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.split(',').next().map(str::trim).and_then(|ip| IpAddr::from_str(ip).ok()));

    tracing::debug!(?x_forwarded_for, ?x_real_ip, ip_address = ?addr.ip());

    x_forwarded_for.unwrap_or_else(|| x_real_ip.unwrap_or_else(|| addr.ip()))
}
