use std::collections::BTreeMap;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("email template '{0}' not found")]
    NotFound(String),
}

fn source(name: &str) -> Option<&'static str> {
    match name {
        "verify-email" => Some(include_str!("../templates/verify-email.html")),
        "reset-password" => Some(include_str!("../templates/reset-password.html")),
        "admin-login-notification" => {
            Some(include_str!("../templates/admin-login-notification.html"))
        }
        "admin-password-reset" => Some(include_str!("../templates/admin-password-reset.html")),
        "admin-forgot-password" => Some(include_str!("../templates/admin-forgot-password.html")),
        "admin-onboarding-notification" => Some(include_str!(
            "../templates/admin-onboarding-notification.html"
        )),
        _ => None,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn render_template(
    name: &str,
    variables: &BTreeMap<String, String>,
    current_year: i32,
) -> Result<String, TemplateError> {
    let mut html = source(name)
        .ok_or_else(|| TemplateError::NotFound(name.to_owned()))?
        .to_owned();
    for (key, value) in variables {
        html = html.replace(&format!("{{{{{key}}}}}"), &escape_html(value));
    }
    Ok(html.replace("{{currentYear}}", &current_year.to_string()))
}
