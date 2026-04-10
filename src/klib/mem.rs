/// Fonctions de manipulation memoire


/// Copie len octets de src vers dst
pub fn memcpy(dst: &mut [u8], src: &[u8], len: usize) {
	let mut i = 0;
	while i < len && i < dst.len() && i < src.len() {
		dst[i] = src[i];
		i += 1;
	}
}

/// Remplit len octets de dst avec la valeur val
pub fn memset(dst: &mut [u8], val: u8, len: usize) {
	let mut i = 0;
	while i < len && i < dst.len() {
		dst[i] = val;
		i += 1;
	}
}

/// Efface len octets de dst (met a 0)
pub fn bzero(dst: &mut [u8], len: usize) {
	memset(dst, 0, len);
}


