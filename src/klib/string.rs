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

/// Retourne la longueur d'un slice jusqu'au premier octet \0
pub fn strlen(s: &[u8]) -> usize {
    let mut l = 0;
    while l < s.len() && s[l] != 0 { l += 1; }
    l
}

/// Convertit un entier en décimal ASCII dans buf (écrit de droite à gauche).
/// Retourne le slice du résultat dans buf.
/// Ex: itoa(42, &mut buf) → &[b'4', b'2']
pub fn itoa(mut n: u32, buf: &mut [u8]) -> &[u8] {
    if buf.is_empty() { return &[]; }
    if n == 0 {
        buf[buf.len() - 1] = b'0';
        return &buf[buf.len() - 1..];
    }
    let mut i = buf.len();
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

/// Convertit un ASCII en entier.
pub fn atoi(s: &[u8]) -> u32 {
    let mut n: u32 = 0;
    let mut i = 0;
    while i< s.len() && s[i] >= b'0' && s[i] <= b'9' {
        n = n * 10 + (s[i] - b'0') as u32;
        i += 1;
    }
    n
}

/// Écrit un nombre 0-99 sur 2 chiffres dans buf[0..2] (avec zéro padding).
/// Ex: pad2(5, &mut buf) → buf = [b'0', b'5']
pub fn pad2(n: u32, buf: &mut [u8]) {
    buf[0] = b'0' + ((n / 10) % 10) as u8;
    buf[1] = b'0' + (n % 10) as u8;
}

