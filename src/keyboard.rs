// Table scancode Set 1 -> Caractère ASCII (index = scancode)
// 0 = pas de caractère (touche spéciales)
static NORMAL: [u8; 58] = [
    0,  27,                                                         // 0x00=rien, 0x01=Escape
    b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',     // 0x02-0x0B
    b'-', b'=', 8,                                                  // 0x0C=-, 0x0D==, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p',     // 0x10-0x19
    b'[', b']', b'\n',                                              // 0x1A=[, 0x1B=], 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l',           // 0x1E-0x26
    b';', b'\'', b'`',                                              // 0x27=;, 0x28=', 0x29=`
    0,                                                              // 0x2A=Left Shift
    b'\\',                                                          // 0x2B=backslash
    b'z', b'x', b'c', b'v', b'b', b'n', b'm',                       // 0x2C-0x32
    b',', b'.', b'/',                                               // 0x33=,  0x34=.  0x35=/
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

static SHIFT: [u8; 58] = [
    0,  27,                                                         // 0x00=rien, 0x01=Escape
    b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',     // 0x02-0x0B
    b'_', b'+', 8,                                                  // 0x0C=_, 0x0D=+, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P',     // 0x10-0x19
    b'{', b'}', b'\n',                                              // 0x1A={, 0x1B=}, 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L',           // 0x1E-0x26
    b':', b'"', b'~',                                               // 0x27=:, 0x28=", 0x29=~
    0,                                                              // 0x2A=Left Shift
    b'|',                                                           // 0x2B=|
    b'Z', b'X', b'C', b'V', b'B', b'N', b'M',                       // 0x2C-0x32
    b'<', b'>', b'?',                                               // 0x33=<, 0x34=>, 0x35=?
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

static mut SHIFT_HELD: bool = false;

pub fn on_key(scancode: u8) {
    // Shift appuyé (0x2A=gauche, 0x36=droite)
    if scancode == 0x2A || scancode == 0x36 {
        unsafe { SHIFT_HELD = true; }
        return;
    }
    // Shift relâché
    if scancode == 0xAA || scancode == 0xB6 {
        unsafe { SHIFT_HELD = false; }
        return;
    }

    // Ignorer les relâchements
    if scancode >= 0x80 {
        return;
    }

    // Convertir en caractère
    if (scancode as usize) < NORMAL.len() {
        let ch = if unsafe { SHIFT_HELD } {
            SHIFT[scancode as usize]
        } else {
            NORMAL[scancode as usize]
        };

        if ch == 8 {
            // Backspace -> effacer dernier caractere
            unsafe { (*core::ptr::addr_of_mut!(crate::vga::WRITER)).backspace(); }
            return;
        }

        if ch == 27 || ch == 0 {
            return; // Escape et touches speciales -> ignorer
        }

        crate::print!("{}", ch as char);
    }
}