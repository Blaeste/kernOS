/// PIT (Programmable Interval Timer) - génère une interruption à fréquence fixe.
/// Fréquence de base : 1 193 182 Hz. On divise pour obtenir environ 100 Hz (10 ms par tick).

use core::arch::asm;

const PIT_CHANNEL0: u16 = 0x40;         // port données channel 0
const PIT_CMD:      u16 = 0x43;         // port commande
const PIT_FREQ:     u32 = 1_193_182;    // fréquence de base du PIT

static mut TICKS:   u64 = 0;

/// Configure le PIT à environ 100Hz
pub fn init(freq: u32) {
    let divisor = PIT_FREQ / freq;
    unsafe {
        // Mode 3 (square wave), channel 0, accès lo/hi byte
        asm!("out dx, al", in("dx") PIT_CMD, in("al") 0x36u8, options(nostack));
        // envoie le diviseur (low byte puis high byte)
        asm!("out dx, al", in("dx") PIT_CHANNEL0, in("al") (divisor & 0xFF) as u8, options(nostack));
        asm!("out dx, al", in("dx") PIT_CHANNEL0, in("al") (divisor >> 8) as u8, options(nostack));
    }
}

/// Appelé par l'IRQ timer - incrémente le compteur
pub fn tick() {
    unsafe {
        let ptr = core::ptr::addr_of_mut!(TICKS);
        *ptr += 1;
    }
}

/// Retourne le nombre de ticks depuis le boot
pub fn get_ticks() -> u64 {
    unsafe { *core::ptr::addr_of!(TICKS) }
}