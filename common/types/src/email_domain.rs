use crate::disposable_email_domains::DISPOSABLE_EMAIL_DOMAINS;

pub fn is_disposable_email(email: &str) -> bool {
    email
        .rsplit_once('@')
        .map(|(_, domain)| domain.trim().to_ascii_lowercase())
        .is_some_and(|domain| DISPOSABLE_EMAIL_DOMAINS.contains(&domain.as_str()))
}
