pub fn to_e164(phone: &str) -> Option<String> {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    if digits.starts_with("27") && digits.len() == 11 {
        return Some(digits);
    }
    if digits.starts_with('0') && digits.len() == 10 {
        return Some(format!("27{}", &digits[1..]));
    }
    None
}
