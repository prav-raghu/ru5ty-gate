use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FasRequest {
    #[serde(default)]
    pub fas: Option<String>,
}
