#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiVersion {
    pub version: &'static str,
    pub is_deprecated: bool,
    pub sunset_date: Option<&'static str>,
    pub is_current: bool,
}

#[derive(Debug, Clone)]
pub struct ApiVersionManager {
    versions: Vec<ApiVersion>,
}

impl Default for ApiVersionManager {
    fn default() -> Self {
        Self {
            versions: vec![
                ApiVersion {
                    version: "v1",
                    is_deprecated: true,
                    sunset_date: Some("2026-12-31"),
                    is_current: false,
                },
                ApiVersion {
                    version: "v2",
                    is_deprecated: false,
                    sunset_date: None,
                    is_current: true,
                },
            ],
        }
    }
}

impl ApiVersionManager {
    pub fn is_version_supported(&self, version: &str) -> bool {
        self.versions.iter().any(|item| item.version == version)
    }

    pub fn version_info(&self, version: &str) -> Option<&ApiVersion> {
        self.versions.iter().find(|item| item.version == version)
    }

    pub fn current_version(&self) -> &'static str {
        self.versions
            .iter()
            .find(|item| item.is_current)
            .map_or("v1", |item| item.version)
    }

    pub fn supported_versions(&self) -> Vec<&'static str> {
        self.versions.iter().map(|item| item.version).collect()
    }

    pub fn deprecated_versions(&self) -> Vec<&ApiVersion> {
        self.versions
            .iter()
            .filter(|item| item.is_deprecated)
            .collect()
    }
}
