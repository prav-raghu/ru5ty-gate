use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FasQuery {
    pub clientip: String,
    pub clientmac: String,
    pub gatewayname: String,
    pub gatewayaddress: String,
    pub gatewayport: String,
    #[serde(default)]
    pub gatewaymac: Option<String>,
    pub originurl: String,
    #[serde(default)]
    pub clientif: Option<String>,
    pub hid: String,
    #[serde(default)]
    pub authaction: Option<String>,
}

impl FasQuery {
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
