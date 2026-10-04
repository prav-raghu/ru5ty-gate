use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct VenueSettings {
    pub id: String,
    #[serde(default = "VenueSettings::default_name")]
    pub name: String,
    #[serde(default)]
    pub gateway_name: Option<String>,
}

impl VenueSettings {
    fn default_name() -> String {
        "default".to_owned()
    }
}
