use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use reqwest::Url;
use tokio::net::lookup_host;

use crate::webhook_error::WebhookError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetPolicy {
    #[default]
    AllowAll,
    PublicOnly,
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    let carrier_grade_nat = octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000;
    let benchmarking = octets[0] == 198 && (octets[1] & 0b1111_1110) == 18;
    let reserved = octets[0] >= 240;
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_documentation()
        || address.is_multicast()
        || carrier_grade_nat
        || benchmarking
        || reserved
        || octets[0] == 0)
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_v4(mapped);
    }
    let first = address.segments()[0];
    let unique_local = (first & 0xfe00) == 0xfc00;
    let link_local = (first & 0xffc0) == 0xfe80;
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || unique_local
        || link_local)
}

fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

impl TargetPolicy {
    pub fn from_production(production: bool) -> Self {
        if production {
            Self::PublicOnly
        } else {
            Self::AllowAll
        }
    }

    pub async fn ensure_allowed(self, raw_url: &str) -> Result<(), WebhookError> {
        if self == Self::AllowAll {
            return Ok(());
        }
        let url = Url::parse(raw_url).map_err(|_| WebhookError::TargetNotAllowed)?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(WebhookError::TargetNotAllowed);
        }
        let host = url.host_str().ok_or(WebhookError::TargetNotAllowed)?;
        let port = url.port_or_known_default().unwrap_or(443);
        if let Ok(address) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
            return if is_public(address) {
                Ok(())
            } else {
                Err(WebhookError::TargetNotAllowed)
            };
        }
        let resolved = lookup_host((host, port))
            .await
            .map_err(|_| WebhookError::TargetNotAllowed)?;
        let mut any = false;
        for socket in resolved {
            any = true;
            if !is_public(socket.ip()) {
                return Err(WebhookError::TargetNotAllowed);
            }
        }
        if any {
            Ok(())
        } else {
            Err(WebhookError::TargetNotAllowed)
        }
    }
}
