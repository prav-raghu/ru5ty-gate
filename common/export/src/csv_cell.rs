use serde_json::Value;

use crate::export_row::cell_text;

const FORMULA_PREFIXES: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

pub fn csv_cell_text(value: &Value) -> String {
    let text = cell_text(value);
    if matches!(value, Value::String(_)) && text.starts_with(FORMULA_PREFIXES) {
        format!("'{text}")
    } else {
        text
    }
}
