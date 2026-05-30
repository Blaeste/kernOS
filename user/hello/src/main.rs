#![no_std]
#![no_main]

extern crate alloc;
use libkern::*;

// ── Test de libkern ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {

    write(b"[hello] write:  ok\n");

    write(b"[hello] pid:    = ");
    print_u64(getpid());
    write(b"\n");

    yield_cpu();
    write(b"[hello] yield:  ok\n");

    // Teste l'allocateur bump : Vec<u8> depuis le heap userspace
    let mut v: Vec<u8> = Vec::new();

    for b in b"[hello] alloc:   ok (Vec depuis libkern bump)\n" {
        v.push(*b);
    }
    write(&v);

    let fd = open(b"test");
    if fd == u64::MAX {

        write(b"[hello] open:  fichier 'test' introuvable\n");
        write(b"        -> kfsWrite test <contenu>\n");
    } else {

        write(b"[hello] open:  fd = ");
        print_u64(fd);
        write(b"\n");

        let mut buf = [0u8; 128];
        let n = read(fd, &mut buf);

        write(b"[hello] read:   ");
        print_u64(n);
        write(b" octets\n");

        if n > 0 {

            write(b"[hello] contenu: ");
            write(&buf[..n as usize]);
            write(b"\n");
        }

        close(fd);
        write(b"[hello] close:  ok\n");
    }

    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
