/// Fonctions de manipulation de chaînes/slices

/// Compare deux slices d'octets (remplace == qui n'est pas dispo en no_std)
pub fn eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}

/// Convertit en &str avec un message fallback personnalisé
pub fn to_str_or<'a>(bytes: &'a [u8], fallback: &'a str) -> &'a str {
    core::str::from_utf8(bytes).unwrap_or(fallback)
}

/// Convertit en &str (ou "???" si pas valide)
pub fn to_str(bytes: &[u8]) -> &str {
    to_str_or(bytes, "???")
}

/// Vérifie si un slice commence par un préfixe
pub fn starts_with(s: &[u8], prefix: &[u8]) -> bool {
    if s.len() < prefix.len() { return false; }
    eq(&s[..prefix.len()], prefix)
}

/// Trouve la position du premier espace dans un slice
pub fn find_space(s: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < s.len() {
        if s[i] == b' ' { return Some(i); }
        i += 1;
    }
    None
}