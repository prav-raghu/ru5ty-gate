use std::borrow::Cow;

use ru5ty_gate_types::FieldError;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

fn default_message(error: &ValidationError) -> String {
    if let Some(message) = &error.message {
        return message.to_string();
    }
    let param = |name: &str| error.params.get(name).map(ToString::to_string);
    match error.code.as_ref() {
        "email" => "Must be a valid email address".to_owned(),
        "url" => "Must be a valid URI".to_owned(),
        "length" => {
            let min = error.params.get("min").and_then(serde_json::Value::as_u64);
            let max = error.params.get("max").and_then(serde_json::Value::as_u64);
            let equal = error
                .params
                .get("equal")
                .and_then(serde_json::Value::as_u64);
            let actual = error
                .params
                .get("value")
                .and_then(serde_json::Value::as_str)
                .map(|value| value.chars().count() as u64);
            match (min, max, equal, actual) {
                (_, _, Some(equal), _) => format!("Must be exactly {equal} characters"),
                (Some(min), _, _, Some(actual)) if actual < min => {
                    format!("Must be at least {min} characters")
                }
                (_, Some(max), _, Some(actual)) if actual > max => {
                    format!("Must be at most {max} characters")
                }
                (Some(min), Some(max), _, _) => {
                    format!("Must be between {min} and {max} characters")
                }
                (Some(min), None, _, _) => format!("Must be at least {min} characters"),
                (None, Some(max), _, _) => format!("Must be at most {max} characters"),
                _ => "Invalid length".to_owned(),
            }
        }
        "range" => match (param("min"), param("max")) {
            (Some(min), Some(max)) => format!("Must be between {min} and {max}"),
            (Some(min), None) => format!("Must be greater than or equal to {min}"),
            (None, Some(max)) => format!("Must be less than or equal to {max}"),
            _ => "Out of range".to_owned(),
        },
        "required" => "Is required".to_owned(),
        "regex" => "Invalid format".to_owned(),
        other => other.replace('_', " "),
    }
}

fn to_camel_case(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut upper_next = false;
    for character in name.chars() {
        if character == '_' {
            upper_next = !output.is_empty();
        } else if upper_next {
            output.extend(character.to_uppercase());
            upper_next = false;
        } else {
            output.push(character);
        }
    }
    output
}

fn collect(prefix: &str, errors: &ValidationErrors, output: &mut Vec<FieldError>) {
    for (field, kind) in errors.errors() {
        let field = to_camel_case(field);
        let path = if prefix.is_empty() {
            field
        } else {
            format!("{prefix}.{field}")
        };
        match kind {
            ValidationErrorsKind::Field(items) => {
                for item in items {
                    output.push(FieldError {
                        field: path.clone(),
                        message: default_message(item),
                    });
                }
            }
            ValidationErrorsKind::Struct(nested) => collect(&path, nested, output),
            ValidationErrorsKind::List(items) => {
                for (index, nested) in items {
                    collect(&format!("{path}[{index}]"), nested, output);
                }
            }
        }
    }
}

pub fn field_errors_from_validator(errors: &ValidationErrors) -> Vec<FieldError> {
    let mut output = Vec::new();
    collect("", errors, &mut output);
    output.sort_by(|left, right| left.field.cmp(&right.field));
    output
}

fn between_backticks(message: &str) -> Option<&str> {
    let start = message.find('`')? + 1;
    let end = start + message[start..].find('`')?;
    Some(&message[start..end])
}

pub fn field_errors_from_serde(
    error: &serde_path_to_error::Error<serde_json::Error>,
) -> Vec<FieldError> {
    let inner = error.inner().to_string();
    let path = error.path().to_string();
    let base = if path == "." { String::new() } else { path };
    let join = |name: &str| {
        if base.is_empty() {
            name.to_owned()
        } else {
            format!("{base}.{name}")
        }
    };

    if inner.starts_with("missing field") {
        let name = between_backticks(&inner).unwrap_or("unknown");
        return vec![FieldError {
            field: join(name),
            message: "Is required".to_owned(),
        }];
    }
    if inner.starts_with("unknown field") {
        let name = between_backticks(&inner).unwrap_or("unknown");
        let field = if base.ends_with(name) {
            base.clone()
        } else {
            join(name)
        };
        return vec![FieldError {
            field,
            message: "Unexpected property".to_owned(),
        }];
    }
    let message: Cow<'_, str> = match inner.split(" at line ").next() {
        Some(trimmed) if !trimmed.is_empty() => Cow::Owned(trimmed.to_owned()),
        _ => Cow::Borrowed("Invalid value"),
    };
    vec![FieldError {
        field: if base.is_empty() {
            "body".to_owned()
        } else {
            base
        },
        message: message.into_owned(),
    }]
}
