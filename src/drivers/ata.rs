// Ports ATA Primary Bus
const ATA_DATA:				u16 = 0x1F0;	// lecture/ecriture 16-bit
const ATA_SECTOR_COUNT:		u16 = 0x1F2;	// nombre de secteurs
const ATA_LBA_LOW:			u16 = 0x1F3;	// bits 0-7 de l'adresse LBA
const ATA_LBA_MID:			u16 = 0x1F4;	// bits 8-15
const ATA_LBA_HIGH:			u16 = 0x1F5;	// bits 16-23
const ATA_DRIVE:			u16 = 0x1F6;	// selection drive + bits 24-27
const ATA_STATUS:			u16 = 0x1F7;	// lire le statut
const ATA_COMMAND:			u16 = 0x1F7;	// envoyer commande (meme port)

const STATUS_BSY:			u8 = 0x80;		// drive occupe
const STATUS_DRQ:			u8 = 0x08;		// pret a transferer des donnees

use core::arch::asm;

unsafe fn inb(port: u16) -> u8 {
	let v: u8;
	unsafe { asm!("in al, dx", out("al") v, in("dx") port, options(nostack)); }
	v
}

unsafe fn outb(port: u16, val: u8) {
	unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack)); }
}

unsafe fn inw(port: u16) -> u16 {
	let v: u16;
	unsafe { asm!("in ax, dx", out("ax") v, in("dx") port, options(nostack)); }
	v
}

unsafe fn outw(port: u16, val: u16) {
	unsafe { asm!("out dx, ax", in("dx") port, in("ax") val, options(nostack)); }
}

unsafe fn wait_ready() {
	// Attend que BSY soit a 0
	unsafe { while inb(ATA_STATUS) & STATUS_BSY != 0 {} }
}

unsafe fn wait_drq() {
	// Attend aue DRQ soit a 1 (pret a transferer)
	unsafe { while inb(ATA_STATUS) & STATUS_DRQ == 0 {} }
}

/// Lit un secteur de 512 octets depusi le disque
/// lba = adresse logique du secteur (commence à 0)
/// buf = tableau de 256 u16 (512 octets)
pub fn read_sector(lba: u32, buf: &mut [u16; 256]) {
	unsafe {
		wait_ready();

		// Configure le drive : master, LBA mode, bits 24-27
		outb(ATA_DRIVE,			0xE0 | ((lba >> 24) & 0x0F) as u8);
		outb(ATA_SECTOR_COUNT,	1);
		outb(ATA_LBA_LOW,		(lba & 0xff) as u8);
		outb(ATA_LBA_MID,		((lba >> 8) & 0xFF) as u8);
		outb(ATA_LBA_HIGH,		((lba >> 16) & 0xFF) as u8);
		outb(ATA_COMMAND,		0x20); // commande READ SECTORS

		wait_drq();

		// Lit 256 mots de 16 bits = 512 octets
		let mut i = 0;
		while i < 256 {
			buf[i] = inw(ATA_DATA);
			i += 1;
		}
	}
}

/// Ecrit un secteur de 512 octets sur le disque
pub fn write_sector(lba: u32, buf: &[u16; 256]) {
	unsafe {
		wait_ready();

		// Configure le drive : master, LBA mode, bits 24-27
		outb(ATA_DRIVE,			0xE0 | ((lba >> 24) & 0x0F) as u8);
		outb(ATA_SECTOR_COUNT,	1);
		outb(ATA_LBA_LOW,		(lba & 0xff) as u8);
		outb(ATA_LBA_MID,		((lba >> 8) & 0xFF) as u8);
		outb(ATA_LBA_HIGH,		((lba >> 16) & 0xFF) as u8);
		outb(ATA_COMMAND,		0x30); // commande WRITE SECTORS

		wait_drq();

		let mut i = 0;
		while i < 256 {
			outw(ATA_DATA, buf[i]);
			i +=1;
		}
	}
}