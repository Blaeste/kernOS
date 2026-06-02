/// Framebuffer - accès direct aux pixels de l'écran.
/// GRUB nous donne l'adresse mémoire, la taille, le pitch.
/// Chaque pixel = 4 octets : Bleu, Vert, Rouge, Alpha (format BGR en mémoire).

use core::fmt;
use core::ptr::{addr_of, addr_of_mut};

use crate::klib::sync::SpinLock;

/// Mutex protect
pub static PRINT_LOCK: SpinLock = SpinLock::new();

/// Infos globales du framebuffer, remplies au boot.
pub static mut FB_ADDR:     u64 = 0;        // adresse physique du framebuffer
pub static mut FB_WIDTH:    u32 = 0;        // largeur en pixels
pub static mut FB_HEIGHT:   u32 = 0;        // hauteur en pixels
pub static mut FB_PITCH:    u32 = 0;        // octets par ligne
pub static mut FB_ACTIVE:   bool = false;   // true si le framebuffer est actif

static mut BACK_BUF: [u32; 2_073_600] = [0; 2_073_600];

/// Dirty rect
static mut DIRTY_X1:    u32 = 0;    // coin haut gauche X
static mut DIRTY_Y1:    u32 = 0;    // coin haut droit Y
static mut DIRTY_X2:    u32 = 0;    // coin bas gauche X
static mut DIRTY_Y2:    u32 = 0;    // coin bas droit Y

/// Couleur du texte par défaut
const FG:                       u32 = 0xFFFFFF; // blanc
pub const TEXT_MAX_COLS:        usize = 100;
pub const TEXT_MAX_ROWS:        usize = 200;

/// Palette ANSI 16 couleurs (0-7 normales, 8-15 vives). Format 0xRRGGBB.
const ANSI_PALETTE: [u32; 16] = [
    0x000000, 0xAA0000, 0x00AA00, 0xAA5500,     // noir,    rouge,      vert,   jaune
    0x0000AA, 0xAA00AA, 0x00AAAA, 0xAAAAAA,     // bleu,    magenta,    cyan,   blanc,
    0x555555, 0xFF5555, 0x55FF55, 0xFFFF55,     // versions vives
    0x5555FF, 0xFF55FF, 0x55FFFF, 0xFFFFFF,
];

/// Writer framebuffer - gère un curseur texte et dessine avec la police bitmap.
pub struct FbWriter {
    pub row:         u32,
    pub col:         u32,
    pub x_origin:    u32,
    pub y_origin:    u32,
    pub max_cols:    u32,
    pub max_rows:    u32,
    pub text_buf:    [[u8; TEXT_MAX_COLS]; TEXT_MAX_ROWS],
    pub bg:          u32,
    pub bound_win:   usize,
    pub view_offset: u32,
    pub fg:          u32,    // couleur de texte courante (modifiable via ANSI)
    pub ansi_state:  u8,     // 0=normal, 1=ESC vu, 2=dans CSI \e[
    pub ansi_acc:    u32,    // paramètre numérique en cours d'accumulation
    pub ansi_bold:   bool,   // gras → couleurs vives (30-37 → 90-97)
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

/// Marque la zone sale a redraw
pub fn mark_dirty(x: u32, y: u32, w: u32, h: u32) {
    unsafe {
        // Agrendit le dirty rect poru englober la nouvelle zone
        if DIRTY_X2 == 0 && DIRTY_Y2 == 0 {
            DIRTY_X1 = x; DIRTY_Y1 = y;
            DIRTY_X2 = x + w; DIRTY_Y2 = y + h;
        } else {
            if x < DIRTY_X1 { DIRTY_X1 = x; }
            if y < DIRTY_Y1 { DIRTY_Y1 = y; }
            if x + w > DIRTY_X2 { DIRTY_X2 = x + w; }
            if y + h > DIRTY_Y2 { DIRTY_Y2 = y + h; }
        }
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
        let h = *addr_of!(FB_HEIGHT);
        if x >= w || y >= h { return 0; }
        BACK_BUF[(y * w + x) as usize]
    }
}

/// Copie le back buffer vers le framebuffer réel (affiche tout d'un coup)
pub fn swap_buffers() {
    unsafe {
        // Rien de modifie -> on copie rien
        if DIRTY_X2 == 0 && DIRTY_Y2 == 0 { return; }

        let addr = *addr_of!(FB_ADDR) as *mut u32;
        let w = *addr_of!(FB_WIDTH) as usize;
        let pitch = *addr_of!(FB_PITCH) as usize;

        let h = *addr_of!(FB_HEIGHT) as usize;
        let w_max = *addr_of!(FB_WIDTH) as usize;
        let x1 = DIRTY_X1 as usize;
        let y1 = DIRTY_Y1 as usize;
        let x2 = if (DIRTY_X2 as usize) > w_max { w_max } else { DIRTY_X2 as usize };
        let y2 = if (DIRTY_Y2 as usize) > h { h } else { DIRTY_Y2 as usize };

        let copy_w = x2 - x1;

        let mut y = y1;
        while y < y2 {
            let src = addr_of!(BACK_BUF).cast::<u32>().add(y * w + x1);
            let dst = addr.add(y * pitch / 4 + x1);
            core::ptr::copy_nonoverlapping(src, dst, copy_w);
            y += 1;
        }

        // Reset dirty rect
        DIRTY_X1 = 0; DIRTY_Y1 = 0;
        DIRTY_X2 = 0; DIRTY_Y2 = 0;
    }
}

/// Dessine un rectangle rempli.
/// (x, y) = coin haut-gauche, w = largeur, h = hauteur
pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    mark_dirty(x, y, w, h);
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
        mark_dirty(0, 0, w as u32, h as u32);
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
    mark_dirty(x, y, 8, 16);

    let idx = if c < 128 { c as usize } else { b'?' as usize }; // attention on gere aps les accent pr le moment
    let glyph = crate::io::font::FONT[idx];      // recupère les 16 lignes du caractère

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

        // ── Séquences ANSI \e[...m ────────────────────────────
        if self.ansi_state != 0 {

            self.ansi_byte(byte);
            return;
        }

        if byte == 0x1b {    // ESC : début de séquence

            self.ansi_state = 1;
            return;
        }

        if self.view_offset > 0 {
            self.view_offset = 0;
            self.redraw_from_buf();
        }
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
                draw_char(x, y, byte, self.fg, self.bg);
                let buf_row = (TEXT_MAX_ROWS - self.max_rows as usize) + self.row as usize;
                self.text_buf[buf_row][self.col as usize] = byte;
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
    pub fn scroll(&mut self) {
        unsafe {
            let sw = *addr_of!(FB_WIDTH) as usize;
            let x0 = self.x_origin as usize;
            let y0 = self.y_origin as usize;
            let zone_w = self.max_cols as usize * 8;
            let zone_h = self.max_rows as usize * 16;
            mark_dirty(self.x_origin, self.y_origin, zone_w as u32, zone_h as u32);

            // Décale text_buf d'une ligne vers le haut
            let mut i = 0;
            while i < TEXT_MAX_ROWS - 1 {
                self.text_buf[i] = self.text_buf[i + 1];
                i += 1;
            }
            self.text_buf[TEXT_MAX_ROWS - 1] = [b' '; TEXT_MAX_COLS];

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
            let buf_row = (TEXT_MAX_ROWS - self.max_rows as usize) + self.row as usize;
            self.text_buf[buf_row][self.col as usize] = b' ';
        }
    }

    /// Efface l'écran et remet le curseur en haut à gauche.
    pub fn clear_screen(&mut self) {
        fill_rect(self.x_origin, self.y_origin, self.max_cols * 8, self.max_rows * 16, self.bg);
        self.row = 0;
        self.col = 0;
        self.text_buf = [[b' '; TEXT_MAX_COLS]; TEXT_MAX_ROWS];
    }

    /// Alimente le parser ANSI (appelé quand ansi_state != 0).
    fn ansi_byte(&mut self, byte: u8) {

        match self.ansi_state {

            1 => {
                // On attend '[' juste après ESC, sinon on abandonne.
                if byte == b'[' { self.ansi_state = 2; self.ansi_acc = 0; }
                else            { self.ansi_state = 0; }
            }
            2 => match byte {
                b'0'..=b'9' => self.ansi_acc = self.ansi_acc * 10 + (byte - b'0') as u32,
                b';'        => { self.apply_sgr(self.ansi_acc); self.ansi_acc = 0; }
                b'm'        => { self.apply_sgr(self.ansi_acc); self.ansi_acc = 0; self.ansi_state = 0; }
                _           => self.ansi_state = 0, // H, J, K.. non gérés 
            },
            _ => self.ansi_state = 0,
        }
    }

    /// Applique un code SGR à la couleur de texte courante.
    fn apply_sgr(&mut self, code: u32) {

        match code {

            0       => { self.fg = FG; self.ansi_bold = false; }                        // reset
            1       => self.ansi_bold = true,                                           // gras -> vif
            22      => self.ansi_bold = false,                                          // gras off
            30..=37 => {
                let idx = (code - 30) as usize + if self.ansi_bold { 8 } else { 0 };
                self.fg = ANSI_PALETTE[idx];
            }
            39      => self.fg = FG,                                                    // couleur par défault
            90..=97 => self.fg = ANSI_PALETTE[(code - 90) as usize + 8],
            _       => {}                                                               // bg & co ignorés (v1)
        }
    }

    /// Redessin depusi le buffer text
    pub fn redraw_from_buf(&self) {
        let base = (TEXT_MAX_ROWS - self.max_rows as usize).saturating_sub(self.view_offset as usize);
        fill_rect(self.x_origin, self.y_origin, self.max_cols * 8, self.max_rows * 16, self.bg);
        let mut r: u32 = 0;
        while r < self.max_rows {
            let buf_row = base + r as usize;
            if buf_row >= TEXT_MAX_ROWS { break; }
            let mut c: u32 = 0;
            while c < self.max_cols {
                let ch = self.text_buf[buf_row][c as usize];
                let x = self.x_origin + c * 8;
                let y = self.y_origin + r * 16;
                if ch != 0 && ch != b' ' {
                    draw_char(x, y, ch, FG, self.bg);
                }
                c += 1;
            }
            r += 1;
        }
    }

    /// Scroll vers le haut
    pub fn scroll_up(&mut self) {
        let max_offset = TEXT_MAX_ROWS - self.max_rows as usize;
        if (self.view_offset as usize) < max_offset {
            self.view_offset += 1;
            self.redraw_from_buf();
        }
    }

    /// Scroll vers le bas
    pub fn scroll_down(&mut self) {
        if self.view_offset > 0 {
            self.view_offset -= 1;
            self.redraw_from_buf();
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
    text_buf: [[b' '; TEXT_MAX_COLS]; TEXT_MAX_ROWS],
    bg: crate::ui::theme::DESKTOP_BG,
    bound_win: usize::MAX,
    view_offset: 0,
    fg: FG,
    ansi_state: 0,
    ansi_acc: 0,
    ansi_bold: false,
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
        writer.text_buf = [[b' '; TEXT_MAX_COLS]; TEXT_MAX_ROWS];
        writer.bound_win = win_idx;
    }
}

/// Bind automatique : lit la taille de la fenetre et calcule la sone de contenue
pub fn bind_writer_to_win(win_idx: usize) {
    unsafe {
        let win = &(*addr_of!(crate::ui::window::WINDOWS))[win_idx];
        let border: u32 = 1;
        let title_h: u32 = 20;
        bind_writer_to_window(
            win.x + border,
            win.y + border + title_h,
            win.w - 2 * border,
            win.h - 2 * border - title_h,
            win_idx,
        );
    }
}
