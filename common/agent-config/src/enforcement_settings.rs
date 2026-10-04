use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnforcementSettings {
    #[serde(default)]
    pub ndsctl_path: Option<String>,
}
