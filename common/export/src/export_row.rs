use serde_json::{Map, Value};

pub type ExportRow = Map<String, Value>;

pub fn cell_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

pub fn resolve_headers(configured: Option<&[String]>, rows: &[ExportRow]) -> Vec<String> {
    match configured {
        Some(headers) => headers.to_vec(),
        None => rows
            .first()
            .map(|row| row.keys().cloned().collect())
            .unwrap_or_default(),
    }
}
