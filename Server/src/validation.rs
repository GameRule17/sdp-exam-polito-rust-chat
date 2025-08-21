// Server-side nickname validation rules
// - non empty, max 32 chars
// - ASCII only
// - no whitespace
// - must start with ASCII letter [A-Za-z]
// - only ASCII alphanumeric [A-Za-z0-9]

pub fn validate_nick_syntax(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("Il nickname non può essere vuoto.".into());
    }
    if s.len() > 32 {
        return Err("Nickname troppo lungo (max 32).".into());
    }
    if !s.is_ascii() {
        return Err("Sono consentiti solo caratteri ASCII.".into());
    }
    if s.chars().any(|c| c.is_whitespace()) {
        return Err("Il nickname non può contenere spazi o caratteri di whitespace.".into());
    }
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        if !first.is_ascii_alphabetic() {
            return Err("Il nickname deve iniziare con una lettera (A-Z o a-z).".into());
        }
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Il nickname può contenere solo lettere e numeri (niente simboli).".into());
    }
    Ok(())
}
