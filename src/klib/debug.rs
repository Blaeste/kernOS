use core::arch::asm;

/// Écrit un octet sur le port 0xE9 (QEMU debug console)
fn out_e9(b: u8) {
    unsafe { asm!("out dx, al", in("dx") 0xE9u16, in("al") b, options(nostack)); }
}

/// Affiche une string sur la console debug QEMU
pub fn dbg_str(s: &str) {
    for &b in s.as_bytes() {
        out_e9(b);
    }
}

/// Affiche une string + newline
pub fn dbg_ln(s: &str) {
    dbg_str(s);
    out_e9(b'\n');
}

/// Affiche un nombre en hexa
pub fn dbg_hex(val: u64) {
    dbg_str("0x");
    let mut started = false;
    let mut i: i32 = 60;
    while i >= 0 {
        let nibble = ((val >> i) & 0xF) as u8;
        if nibble != 0 || started || i == 0 {
            started = true;
            if nibble < 10 { out_e9(b'0' + nibble); }
            else { out_e9(b'A' + nibble - 10); }
        }
        i -= 4;
    }
}