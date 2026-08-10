//! openNDS Forward Authentication Service (FAS) protocol types.
//!
//! openNDS intercepts an unauthenticated client, then redirects the
//! client's browser to this agent (the FAS) with a query string describing
//! the client and gateway. Once the agent decides to grant access, it must
//! redirect the client's browser back to openNDS's own auth endpoint so
//! openNDS can install the firewall rule that actually lets the client's
//! traffic through.
//!
//! Reference: openNDS FAS documentation
//! (<https://opennds.readthedocs.io/en/latest/fas.html>).

use serde::Deserialize;

/// Query parameters openNDS appends when it redirects a client's browser
/// to the FAS URL.
///
/// `authaction`, `gatewaymac` and `clientif` aren't sent by every openNDS
/// configuration (they depend on `fas_secure_enabled` level and gateway
/// interface setup), so they're optional here.
#[derive(Debug, Clone, Deserialize)]
pub struct FasQuery {
    /// Client's IP address on the gateway's LAN/WLAN.
    pub clientip: String,
    /// Client's MAC address -- the primary key we track sessions by.
    pub clientmac: String,
    /// The openNDS gateway (router) name, as configured in openNDS.
    pub gatewayname: String,
    /// Address openNDS is listening on, used to build the auth callback
    /// URL when `authaction` isn't supplied.
    pub gatewayaddress: String,
    /// Port openNDS is listening on.
    pub gatewayport: String,
    #[serde(default)]
    pub gatewaymac: Option<String>,
    /// The URL the client was originally trying to reach before being
    /// captured; we send them back here (or to a policy-supplied
    /// redirect) after granting access.
    pub originurl: String,
    #[serde(default)]
    pub clientif: Option<String>,
    /// openNDS's per-auth-attempt session token/hash. Must be echoed back
    /// verbatim as `tok` on the auth callback so openNDS can match this
    /// request to the client it's holding captive.
    pub hid: String,
    /// When present (openNDS `fas_secure_enabled >= 1`), the exact URL to
    /// redirect the client to in order to complete authentication.
    /// Preferred over manually constructing the `/opennds_auth/` path.
    #[serde(default)]
    pub authaction: Option<String>,
}

impl FasQuery {
    /// Build the URL to send the client's browser to in order to complete
    /// authentication with openNDS, once the agent has decided to grant
    /// access.
    pub fn auth_redirect_url(&self) -> String {
        let base = self.authaction.clone().unwrap_or_else(|| {
            format!(
                "http://{}:{}/opennds_auth/",
                self.gatewayaddress, self.gatewayport
            )
        });
        let separator = if base.contains('?') { '&' } else { '?' };
        format!(
            "{base}{separator}tok={}&redir={}",
            urlencoding::encode(&self.hid),
            urlencoding::encode(&self.originurl),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(authaction: Option<&str>) -> FasQuery {
        FasQuery {
            clientip: "10.1.0.5".to_string(),
            clientmac: "AA:BB:CC:DD:EE:FF".to_string(),
            gatewayname: "gl-mt6000".to_string(),
            gatewayaddress: "10.1.0.1".to_string(),
            gatewayport: "2050".to_string(),
            gatewaymac: None,
            originurl: "http://example.com/page?x=1".to_string(),
            clientif: None,
            hid: "abc123hash".to_string(),
            authaction: authaction.map(str::to_string),
        }
    }

    #[test]
    fn builds_url_from_gateway_when_no_authaction() {
        let q = query(None);
        let url = q.auth_redirect_url();
        assert!(url.starts_with("http://10.1.0.1:2050/opennds_auth/?"));
        assert!(url.contains("tok=abc123hash"));
        assert!(url.contains("redir=http%3A%2F%2Fexample.com%2Fpage%3Fx%3D1"));
    }

    #[test]
    fn prefers_authaction_when_present() {
        let q = query(Some("http://10.1.0.1:2050/opennds_auth/?extra=1"));
        let url = q.auth_redirect_url();
        assert!(url.starts_with("http://10.1.0.1:2050/opennds_auth/?extra=1&tok="));
    }
}
