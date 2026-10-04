pub fn resolve_redis_url(url: &str, reject_unauthorized: bool) -> String {
    let uses_tls = url.starts_with("rediss://");
    if uses_tls && !reject_unauthorized && !url.ends_with("#insecure") {
        format!("{url}#insecure")
    } else {
        url.to_owned()
    }
}
