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

    // ── Test pipe ─────────────────────────────────────────────────────────
    let (fd_r, fd_w) = pipe();
    if fd_r == u64::MAX {
        write(b"[hello] pipe:    erreur\n");
    } else {
        // Ecrit dans le write-end
        let msg = b"pipe ok";
        unsafe { sys(0, fd_w, msg.as_ptr() as u64, msg.len() as u64); }

        // Lit depuis le read-end
        let mut pbuf = [0u8; 32];
        let n = read(fd_r, &mut pbuf);
        write(b"[hello] pipe:    ");
        write(&pbuf[..n as usize]);
        write(b"\n");

        close(fd_w);
        close(fd_r);
    }

    // ── Test lecture > 512 octets ─────────────────────────────────────────
    // Prerequis : `install big` copie l'ELF hello (~19 Ko) dans KFS.
    // Verifie que read() franchit la frontiere des 512 o (taille du tampon
    // de transit kernel) et que fd.pos avance correctement d'un appel a
    // l'autre : on lit jusqu'a EOF et on compte le total.
    let fd = open(b"big");
    if fd == u64::MAX {

        write(b"[hello] big:     absent -> fais 'install big' puis 'run'\n");
    } else {

        let mut buf = [0u8; 512];
        let mut total: u64 = 0;

        loop {
            let n = read(fd, &mut buf);
            if n == 0 || n == u64::MAX { break; }
            total += n;
        }

        write(b"[hello] big:     ");
        print_u64(total);
        write(b" octets lus\n");

        close(fd);
    }

    // ── Test stdin ─────────────────────────────────────────────────────────
    write(b"Ton nom ? ");
    let mut buf = [0u8; 64];
    let name = libkern::readline(&mut buf);
    write(b"Bonjour, ");
    write(name);
    write(b" !\n");

    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
