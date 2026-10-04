use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Csv,
    Excel,
}

impl ExportFormat {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv; charset=utf-8",
            Self::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => ".csv",
            Self::Excel => ".xlsx",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ExportQuery {
    #[serde(default)]
    pub format: ExportFormat,
}
