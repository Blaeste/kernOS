/*
    Port I/O → communication avec le matériel via des adresses spéciales (pas de la RAM). in/out en ASM
    io_wait() → le PIC est un vieux chip lent, il faut attendre entre chaque commande
    Masques → chaque bit = un IRQ activé/désactivé. On sauvegarde et restaure pour ne pas changer la config existante
    EOI → obligatoire après chaque IRQ — sinon le PIC croit qu'on traite encore l'interruption et n'en envoie plus
*/

use core::arch::asm;

// PIC — Programmable Interrupt Controller
// IRQ - Interruptions matérielles (Interrupt ReQuest) :

// Ports des deux PIC 8259
const PIC1_CMD:     u16 = 0x20;     // port commande du PIC maître
const PIC1_DATA:    u16 = 0x21;     // port données du PIC maître
const PIC2_CMD:     u16 = 0xA0;     // port commande du PIC esclave
const PIC2_DATA:    u16 = 0xA1;     // port données du PIC esclave

// Commandes PIC
const CMD_INIT:     u8 = 0x11;      // initialise le PIC (mode cascade)
const CMD_EOI:      u8 = 0x20;      // End Of Interrupt - dit au PIC "J'ai traité l'interruption"

// Offset dans l'IDT : IRQ0 -> entrée 32, IRQ1 -> 33, etc
const PIC1_OFFSET:  u8 = 32;
const PIC2_OFFSET:  u8 = 40;

// Lit un octet depuis un port I/O
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe { asm!("in al, dx", out("al") value, in("dx") port, options(nostack)); }
    value
}

// Ecrit un octet vers port I/O
unsafe fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nostack)); }
}

// Petite attente- necessaire entre les commandes PIC (matériel lent)
unsafe fn io_wait() {
    unsafe { outb(0x80, 0); } // port 0x80 = port de debug, écrire dedans prend du temps
}

pub fn init() {
    unsafe {
        // Sauvegarde ignorée — on définit nos propres masques explicitement

        // Initialisation en mode cascade
        outb(PIC1_CMD, CMD_INIT); io_wait();        // démarre l'initialisation du PIC maître
        outb(PIC2_CMD, CMD_INIT); io_wait();        // démarre l'initialisation du PIC esclave

        // Définit les offset IDT
        outb(PIC1_DATA, PIC1_OFFSET); io_wait();    // IRQ0 -> IDT[32]
        outb(PIC2_DATA, PIC2_OFFSET); io_wait();    // IRQ8 -> IDT[40]

        // Configure la cascade
        outb(PIC1_DATA, 0x04); io_wait();           // PIC1 : esclave sur IRQ2
        outb(PIC2_DATA, 0x02); io_wait();           // PIC2 : identité cascade = 2

        // Mode 8086
        outb(PIC1_DATA, 0x01); io_wait();           // mode 8086/88 (pas mode MCS-80/85)
        outb(PIC2_DATA, 0x01); io_wait();           // fin de la séquence d'initialisation

        // Démasque seulement les IRQ qu'on gère (0 = actif, 1 = masqué)
        // PIC1 : IRQ0 (timer) + IRQ1 (clavier) actifs, reste masqué
        outb(PIC1_DATA, 0b11111000);                // bits 0,1,2 = timer + clavier + cascade
        // PIC2 : tout masqué pour l'instant
        outb(PIC2_DATA, 0b11101111);                // bit 4 = IRQ12 souris
    }
}

// Envoie EOI au(x) PIC après traitement d'un IRQ
pub unsafe fn eoi(irq: u8) {
    if irq >= 8 {
        unsafe { outb(PIC2_CMD, CMD_EOI); }   // IRQ 8-15 : prévenir aussi le PIC esclave
    }
    unsafe { outb(PIC1_CMD, CMD_EOI); }
}