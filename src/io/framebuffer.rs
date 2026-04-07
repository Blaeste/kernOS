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

/// Back buffer pour le double buffering (max 1920×1080)
static mut BACK_BUF:        [u32; 2_073_600] = [0; 2_073_600];

/// Couleur du texte par défaut
const FG:                   u32 = 0xFFFFFF; // blanc

/// Writer framebuffer - gère un curseur texte et dessine avec la police bitmap.
pub struct FbWriter {
    pub row:      u32,                      // ligne courante (en caractères, aps en pixels)
    pub col:      u32,                      // colonne courante
    pub x_origin: u32,                      // pixel X du coin haut-gauche de la zone d'écriture
    pub y_origin: u32,                      // pixel Y du coin haut-gauche de la zone d'écriture
    pub max_cols: u32,                      // nombre max de colonnes
    pub max_rows: u32,                      // nombre max de lignes
    pub text_buf: [[u8; 64]; 20],           // buffer texte max 64 cols x 20 lignes
    pub bg: u32,                            // couleur de fond de la zone d'écriture
    pub bound_win: usize,                   // index fenetre liée (usize::MAX = aucune)
}

/// Initialise le framebuffer avec le sinfos du tag Multiboot2.
pub fn init(fb: &crate::multiboot::FramebufferTag) {
    unsafe {
        *addr_of_mut!(FB_ADDR) = fb.addr;
        *addr_of_mut!(FB_WIDTH) = fb.width;
        *addr_of_mut!(FB_HEIGHT) = fb.height;
        *addr_of_mut!(FB_PITCH) = fb.pitch;
        *addr_of_mut!(FB_ACTIVE) = true;

        // Initialise le writer pour plein écran
        (*addr_of_mut!(FB_WRITER)).x_origin = 0;
        (*addr_of_mut!(FB_WRITER)).y_origin = 0;
        (*addr_of_mut!(FB_WRITER)).max_cols = fb.width / 8;
        (*addr_of_mut!(FB_WRITER)).max_rows = fb.height / 16;
    }
}

/// Dessine un pixel à la position (x, y) avec la couleur donnée.
/// color = 0xRRGGBB (ex: 0xFF0000 = rouge)
pub fn put_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        let w = *addr_of!(FB_WIDTH);
        if x >= w || y >= *addr_of!(FB_HEIGHT) { return; }
        BACK_BUF[(y * w + x) as usize] = color;
    }
}

/// Écrit un pixel directement au framebuffer (sans le back buffer)
pub fn put_pixel_direct(x: u32, y: u32, color: u32) {
    unsafe {
        let w = *addr_of!(FB_WIDTH);
        let h = *addr_of!(FB_HEIGHT);
        if x >= w || y >= h { return; }
        let addr = *addr_of!(FB_ADDR) as *mut u32;
        let pitch = *addr_of!(FB_PITCH);
        let dst = addr.add((y * pitch / 4 + x) as usize);
        *dst = color;
    }
}

/// Lit la couleur du pixel à la position (x, y).
pub fn get_pixel(x: u32, y: u32) -> u32 {
    unsafe {
        let w = *addr_of!(FB_WIDTH);
        BACK_BUF[(y * w + x) as usize]
    }
}

/// Copie le back buffer vers le framebuffer réel (affiche tout d'un coup)
pub fn swap_buffers() {
    unsafe {
        let addr = *addr_of!(FB_ADDR) as *mut u32;
        let w = *addr_of!(FB_WIDTH) as usize;
        let h = *addr_of!(FB_HEIGHT) as usize;
        let pitch = *addr_of!(FB_PITCH) as usize;
        let mut y = 0;
        while y < h {
            let src = addr_of!(BACK_BUF).cast::<u32>().add(y * w);
            let dst = addr.add(y * pitch / 4);
            core::ptr::copy_nonoverlapping(src, dst, w);
            y += 1;
        }
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
        let w = *addr_of!(FB_WIDTH) as usize;
        let h = *addr_of!(FB_HEIGHT) as usize;
        let mut i = 0;
        while i < w * h {
            BACK_BUF[i] = color;
            i += 1;
        }
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
                if self.row >= self.max_rows {
                    self.scroll();
                }
            }
            byte => {
                let x = self.x_origin + self.col * 8;       // position pixel X
                let y = self.y_origin + self.row * 16;      // position pixel Y
                draw_char(x, y, byte, FG, self.bg);
                self.text_buf[self.row as usize][self.col as usize] = byte;
                self.col += 1;
                if self.col >= self.max_cols {
                    self.col = 0;
                    self.row += 1;
                    if self.row >= self.max_rows {
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
        unsafe {
            let sw = *addr_of!(FB_WIDTH) as usize;
            let x0 = self.x_origin as usize;
            let y0 = self.y_origin as usize;
            let zone_w = self.max_cols as usize * 8;
            let zone_h = self.max_rows as usize * 16;

            // Copie chaque ligne de pxiels une ligne de texte (16px) vers le haut
            let mut y: usize = y0 + 16;
            while y < y0 + zone_h {
                let mut x: usize = x0;
                while x < x0 + zone_w {
                    BACK_BUF[(y - 16) * sw + x] = BACK_BUF[y * sw + x];
                    x += 1;
                }
                y += 1;
            }

            // Efface la dernière ligne texte (16 lignes de pxiels)
            fill_rect(self.x_origin, (y0 + zone_h - 16) as u32, zone_w as u32, 16, self.bg);
        }

        self.row = self.max_rows - 1;
        self.col = 0;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            let x = self.x_origin + self.col * 8;
            let y = self.y_origin + self.row * 16;
            draw_char(x, y, b' ', FG, self.bg);
            self.text_buf[self.row as usize][self.col as usize] = b' ';
        }
    }

    /// Efface l'écran et remet le curseur en haut à gauche.
    pub fn clear_screen(&mut self) {
        fill_rect(self.x_origin, self.y_origin, self.max_cols * 8, self.max_rows * 16, self.bg);
        self.row = 0;
        self.col = 0;
        self.text_buf = [[b' '; 64]; 20];
    }

    /// Redessine tout le texte depuis le buffer (après un drag de fenêtre)
    pub fn redraw_text(&self) {
        let mut r: u32 = 0;
        while r < self.max_rows.min(20) {
            let mut c: u32 = 0;
            while c < self.max_cols.min(64) {
                let ch = self.text_buf[r as usize][c as usize];
                let x = self.x_origin + c * 8;
                let y = self.y_origin + r * 16;
                draw_char(x, y, ch, FG, self.bg);
                c += 1;
            }
            r += 1;
        }
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
    row: 0, col: 0,
    x_origin: 0, y_origin: 0,
    max_cols: 0, max_rows: 0,
    text_buf: [[b' '; 64]; 20],
    bg: crate::ui::theme::DESKTOP_BG,
    bound_win: usize::MAX,
};

/// Configure le FB_WRITER pour écrire dans la zone de contenu d'une fentre
pub fn bind_writer_to_window(x: u32, y: u32, w: u32, h: u32, win_idx: usize) {
    unsafe {
        let writer = &mut *addr_of_mut!(FB_WRITER);
        writer.x_origin = x;
        writer.y_origin = y;
        writer.max_cols = w / 8;
        writer.max_rows = h / 16;
        writer.row = 0;
        writer.col = 0;
        writer.bg = crate::ui::theme::WIN_CONTENT;
        writer.text_buf = [[b' '; 64]; 20];
        writer.bound_win = win_idx;
    }
}

