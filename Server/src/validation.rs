/*  
    Regole comuni di validazione lato server per nickname e nomi dei gruppi
    - non vuoto, max 32 caratteri
    - solo ASCII
    - niente whitespace
    - deve iniziare con lettera ASCII [A-Za-z]
    - solo alfanumerico ASCII [A-Za-z0-9]
*/

// Tipo di identificatore che stiamo validando
pub enum NameKind {
    Nick,
    Group,
}

impl NameKind {
    fn label(&self) -> &'static str {
        match self { 
            Self::Nick => "nickname", 
            Self::Group => "nome del gruppo"
        }
    }
    fn capital_label(&self) -> &'static str {
        match self {
            Self::Nick => "Nickname",
            Self::Group => "Nome del gruppo"
        }
    }
}

/// Valida un identificatore generico usato come nickname o nome gruppo.
pub fn validate_identifier(kind: NameKind, s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("Il {} non può essere vuoto.", kind.label()));
    }
    if s.len() > 32 {
        return Err(format!("{} troppo lungo (max 32).", kind.capital_label()));
    }
    if !s.is_ascii() {
        return Err("Sono consentiti solo caratteri ASCII.".into());
    }
    let lowered = s.to_ascii_lowercase();
    if lowered == "server" || lowered == "client" {
        return Err(format!("Il {} non può chiamarsi 'server' o 'client'", kind.label()));
    }
    if s.chars().any(|c| c.is_whitespace()) {
        return Err(format!("Il {} non può contenere spazi o caratteri di whitespace.", kind.label()));
    }
    if let Some(first) = s.chars().next() {
        if !first.is_ascii_alphabetic() {
            return Err(format!("Il {} deve iniziare con una lettera (A-Z o a-z).", kind.label()));
        }
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("Il {} può contenere solo lettere e numeri (niente simboli).", kind.label()));
    }
    Ok(())
}

pub fn validate_nick_syntax(s: &str) -> Result<(), String> {
    validate_identifier(NameKind::Nick, s)
}

pub fn validate_group_name_syntax(s: &str) -> Result<(), String> {
    validate_identifier(NameKind::Group, s)
}
