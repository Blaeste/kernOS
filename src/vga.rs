/*
	&[u8] → une slice d'octets — c'est le type d'un b"texte" passé en paramètre
	row: usize → usize = entier de la taille d'un pointeur (64-bit sur notre machine)
*/

use core::fmt; // trait Write de core::fmt — donne accès au système de formatage {} sans std

const VGA: *mut u8 = 0xB8000 as *mut u8; 		// adresse physique du buffer VGA text mode
const WIDTH: usize = 80;						// colonnes par ligne
const HEIGHT: usize = 25;						// lignes à l'écran

// Struct qui represente l'etat de l'ecran
pub struct Writer {
	pub row: usize,								// ligne courante du curseur (0 = haut)
	pub col: usize,								// colonne courante du curseur (0 = gauche)
	pub color: u8,								// octet couleur : bits 7-4 = fond, bits 3-0 = texte
}

impl Writer {
	pub fn clear(&mut self) {
		for i in 0..(WIDTH * HEIGHT) {
			unsafe {
				*VGA.add(i * 2)		= b' ';
				*VGA.add(i * 2 + 1)	= 0x00;
			}
		}
		self.row = 0;
		self.col = 0;
	}

	pub fn write_byte(&mut self, byte: u8) {
		match byte {
			b'\n' => { // Si c'est un retour a la ligne
				self.col = 0;
				self.row += 1;
			}
			byte => {
				let offset = (self.row * WIDTH +  self.col) * 2;
				unsafe {
					*VGA.add(offset)		= byte;
					*VGA.add(offset + 1)	= self.color;
				}
				self.col += 1;
				if self.col >= WIDTH { // retour a la ligne auto
					self.col = 0;
					self.row += 1;
				}
			}
		}
	}

	pub fn write_str_bytes(&mut self, s: &[u8]) {
		for &byte in s {
			self.write_byte(byte);
		}
	}
}

// Implemente le trait Write de core::fmt
impl fmt::Write for Writer {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.write_str_bytes(s.as_bytes());
		Ok(())
	}
}

// Writer global (unsafe ca rpas de mutex actuellement)
pub static mut WRITER: Writer = Writer {
	row: 0,
	col: 0,
	color: 0x0F,
};