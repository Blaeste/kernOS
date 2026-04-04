/*
	&[u8] → une slice d'octets — c'est le type d'un b"texte" passé en paramètre
	row: usize → usize = entier de la taille d'un pointeur (64-bit sur notre machine)
*/

const VGA: *mut u8 = 0xB8000 as *mut u8;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

pub fn clear() {
	for i in 0..(WIDTH * HEIGHT) {
		unsafe {
			*VGA.add(i * 2)     = b' ';
			*VGA.add(i * 2 + 1) = 0x00;
		}
	}
}

pub fn print_at(text: &[u8], row: usize, col: usize, color: u8) {
	let offset = (row * WIDTH + col) * 2;
	for (i, &byte) in text.iter().enumerate() {
		unsafe {
			*VGA.add(offset + i * 2)    = byte;
			*VGA.add(offset + i * 2 + 1) = color;
		}
	}
}

pub fn center(text: &[u8], row: usize, color: u8) {
	let col = (WIDTH - text.len()) / 2;
	print_at(text, row, col, color);
}