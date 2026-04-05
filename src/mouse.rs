use core::arch::asm;

// Envoie une commande au contrôleur PS/2
unsafe fn ps2_write_cmd(cmd: u8) {
    // Attendre que le buffer d'entrée soit vide
    while unsafe { read_status() & 0x02 } != 0 {}
    unsafe { outb(0x64, cmd); }
}

// Envoie une donnée à la souris
unsafe fn ps2_write_data(data: u8) {
    while unsafe { read_status() & 0x02 } != 0 {}
    unsafe { outb(0x60, data); }
}

unsafe fn read_status() -> u8 {
    unsafe { inb(0x64) }
}

pub unsafe fn init() {
    unsafe { ps2_write_cmd(0xA8); }                     // activer la souris

    unsafe { ps2_write_cmd(0x20); }                     // lire octet de config
    while unsafe { read_status() & 0x01 } == 0 {}       // attendre que les données soient prêtes
    let config = unsafe { inb(0x60) } | 0x02;           // activer IRQ12
    unsafe { ps2_write_cmd(0x60); }                     // ecrire config
    unsafe { ps2_write_data(config); }                  // envoyer la config modifiée à la souris

    unsafe { ps2_write_cmd(0xD4); }                     // prochain octet -> soruis
    unsafe { ps2_write_data(0xF4); }                    // enable data reporting
}

// Compteur global poru savoir quel octet du paquet on reçoit
static mut PACKET: [u8; 3] = [0; 3];
static mut PACKET_IDX: usize = 0;

pub fn on_irq(byte: u8) {
    let result = unsafe {
        // Si on attend le 1er octet et que le bit n'est aps et -> octet invalide, on ignore
        if PACKET_IDX == 0 && (byte & 0x08) == 0 {
            return;
        }

        PACKET[PACKET_IDX] = byte;
        PACKET_IDX += 1;

        if PACKET_IDX == 3 {
            PACKET_IDX = 0;
            let flags = PACKET[0];
            let dx = PACKET[1] as i8;
            let dy = PACKET[2] as i8;
            Some((dx, dy, (flags & 0x01) != 0, (flags & 0x02) != 0))
        } else {
            None
        }
    };

    if let Some((dx, dy, btn_left, btn_right)) = result {
        //crate::println!("[SOURIS] dx:{} dy:{} G:{} D:{}", dx, dy, btn_left as u8, btn_right as u8);
    }
}

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { asm!("in al, dx", out("al") v, in("dx") port, options(nostack)); }
    v
}

unsafe fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nostack)); }
}