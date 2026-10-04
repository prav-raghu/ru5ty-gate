use validator::ValidationError;

pub fn six_digit_code(value: &str) -> Result<(), ValidationError> {
    if value.len() == 6 && value.chars().all(|character| character.is_ascii_digit()) {
        Ok(())
    } else {
        Err(ValidationError::new("pattern").with_message("Must be a 6 digit code".into()))
    }
}
