/// Framebuffer - accès direct aux pixels de l'écran.
/// GRUB nous donne l'adresse mémoire, la taille, le pitch.
/// Chaque pixel = 4 octets : Bleu, Vert, Rouge, Alpha (format BGR en mémoire).

use core::fmt;
use core::ptr::{addr_of, addr_of_mut};

/// Infos globales du framebuffer, remplies au boot.
pub static mut FB_ADDR:     u64 = 0;        // adresse physique du framebuffer
pub static mut FB_WIDTH:    u32 = 0;        // largeur en pixels
pub static mut FB_HEIGHT:   u32 = 0;        // hauteur en pixels
pub static mut FB_PITCH:    u32 = 0;        // octets par ligne
pub static mut FB_ACTIVE:   bool = false;   // true si le framebuffer est actif

/// Couleurs par défault du texte
const FG:                   u32 = 0xFFFFFF; // blanc
const BG:                   u32 = 0x003366; // bleu foncé (fond)

/// Writer framebuffer - gère un curseur texte et dessine avec la police bitmap.
pub struct FbWriter {
    pub row: u32,                           // ligne courante (en caractères, aps en pixels)
    pub col: u32,                           // colonne courante
}

/// Nombre de colonnes texte (dépend de la résolution)
fn cols() -> u32 {
    unsafe { *addr_of!(FB_WIDTH) / 8 }
}

/// Nombre de lignes texte (dépend de la résolution)
fn rows() -> u32 {
    unsafe { *addr_of!(FB_HEIGHT) / 16 }
}

/// Initialise le framebuffer avec le sinfos du tag Multiboot2.
pub fn init(fb: &crate::multiboot::FramebufferTag) {
    unsafe {
        *addr_of_mut!(FB_ADDR) = fb.addr;
        *addr_of_mut!(FB_WIDTH) = fb.width;
        *addr_of_mut!(FB_HEIGHT) = fb.height;
        *addr_of_mut!(FB_PITCH) = fb.pitch;
        *addr_of_mut!(FB_ACTIVE) = true;
    }
}

/// Dessine un pixel à la position (x, y) avec la couleur donnée.
/// color = 0xRRGGBB (ex: 0xFF0000 = rouge)
pub fn put_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        let addr = *addr_of!(FB_ADDR);
        let pitch = *addr_of!(FB_PITCH);

        // calcule l'adresse du pixel : base + (y * pitch) + (x * 4)
        let pixel = (addr + (y as u64 * pitch as u64) + (x as u64 * 4)) as *mut u32;
        *pixel = color;
    }
}

/// Lit la couleur du pixel à la position (x, y).
pub fn get_pixel(x: u32, y: u32) -> u32 {
    unsafe {
        let addr = *addr_of!(FB_ADDR);
        let pitch = *addr_of!(FB_PITCH);

        let pixel = (addr + (y as u64 * pitch as u64) + (x as u64 * 4)) as *const u32;
        *pixel
    }
}

/// Dessine un rectangle rempli.
/// (x, y) = coin haut-gauche, w = largeur, h = hauteur
pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    let mut dy = 0;
    while dy < h {
        let mut dx = 0;
        while dx < w {
            put_pixel(x + dx, y + dy, color);
            dx += 1;
        }
        dy += 1;
    }
}

/// Efface tout l'écran avec la couleur donnée.
pub fn clear(color: u32) {
    unsafe {
        let w = *addr_of!(FB_WIDTH);
        let h = *addr_of!(FB_HEIGHT);
        fill_rect(0, 0, w, h, color);
    }
}

/// Dessine un caractère à la position (x, y) avec la police bitmap 8x16.
/// fg = couleur du text, ng = couleur du fond.
pub fn draw_char(x: u32, y: u32, c: u8, fg: u32, bg: u32) {
    let glyph = crate::io::font::FONT[c as usize];      // recupère les 16 lignes du caractère

    let mut row = 0;
    while row < 16 {
        let line = glyph[row];

        let mut col = 0;
        while col < 8 {
            // teste chaque bit de gauche a droite (bit 7 = pixel de gauche)
            if line & (0x80 >> col) != 0 {
                put_pixel(x + col, y + row as u32, fg); // bit = 1 -> couleur du texte
            } else {
                put_pixel(x + col, y + row as u32, bg); // bit = 0 -> couleur du fond
            }
            col += 1;
        }
        row += 1;
    }
}

/// Dessine une chaine de charactères à partir de la poisition (x, y).
/// Chaque caractère fait 8 pixels de large.
pub fn draw_string(x: u32, y: u32, s: &str, fg: u32, bg: u32) {
    let mut cx = x;
    let bytes = s.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        draw_char(cx, y, bytes[i], fg, bg);
        cx += 8;
        i += 1;
    }
}

impl FbWriter {
    /// Ecrit un octet (caractère) à la position courante du curseur.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.col = 0;
                self.row += 1;
                if self.row >= rows() {
                    self.scroll();
                }
            }
            byte => {
                let x = self.col * 8;       // position pixel X
                let y = self.row * 16;      // position pixel Y
                draw_char(x, y, byte, FG, BG);
                self.col += 1;
                if self.col >= cols() {
                    self.col = 0;
                    self.row += 1;
                    if self.row >= rows() {
                        self.scroll();
                    }
                }
            }
        }
    }

    /// Scroll : redessine tout le texte une ligne plsu haut.
    /// On efface simplement l'écran et on remet le curseur à la dernière ligne.
    /// (Version simple - a améliorer plsu atrd avec un vrai buffer texte)
    pub fn scroll(&mut self) {
        clear(BG);
        self.row = rows() - 1;
        self.col = 0;
    }

    /// Efface le dernier caractère tapé.
    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            let x = self.col * 8;
            let y = self.row * 16;
            draw_char(x, y, b' ', FG, BG); // dessine un espace par-dessus
        }
    }

    /// Efface l'écran et remet le curseur en haut à gauche.
    pub fn clear_screen(&mut self) {
        clear(BG);
        self.row = 0;
        self.col = 0;
    }
}

impl fmt::Write for FbWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Writer global du frambuffer
pub static mut FB_WRITER: FbWriter = FbWriter {
    row: 0,
    col: 0,
};