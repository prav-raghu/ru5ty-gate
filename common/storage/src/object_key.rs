pub fn build_object_key(folder: Option<&str>, file_name: &str) -> String {
    let safe_name: String = file_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    match folder
        .map(|value| value.trim_matches('/'))
        .filter(|value| !value.is_empty())
    {
        Some(folder) => format!("{folder}/{safe_name}"),
        None => safe_name,
    }
}
