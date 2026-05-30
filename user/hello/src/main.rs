#![no_std]
#![no_main]

// ── Wrappers syscall ──────────────────────────────────────────────────────
// Un seul wrapper générique : rax=num, rdi=a1, rsi=a2, rdx=a3.
// Le stub int 0x80 réarange vers les registres C et écrit la valeur de retour
// dans le slot rax sauvegardé -> inlateout capture le résultat.

#[inline(always)]
unsafe fn sys(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {

    let ret: u64;

    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("rax") num => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            options(nostack),
        );
    }
    ret
}

fn write(s: &[u8]) {
    unsafe { sys(0, 1, s.as_ptr() as u64, s.len() as u64); }
}

fn exit(code: u64) -> ! {
    unsafe { sys(1, code, 0, 0); }
    loop {}
}

fn getpid() -> u64                      { unsafe { sys(2, 0, 0, 0) } }

fn yield_cpu()                          { unsafe { sys(3, 0, 0, 0); } }

fn open(path: &[u8]) -> u64             { unsafe { sys(4, path.as_ptr() as u64, path.len() as u64, 0) } }

fn close(fd: u64)                       { unsafe { sys(5, fd, 0, 0); } }

fn read(fd: u64, buf: &mut [u8]) -> u64 { unsafe { sys(6, fd, buf.as_ptr() as u64, buf.len() as u64) } }

fn print_u64(mut n: u64) {

    if n == 0 { write(b"0"); return; }

    let mut buf = [0u8; 20];
    let mut i = 20usize;

    while n > 0  { i -= 1; buf[i] = b'0' + (n % 10) as u8; n /= 10; }

    write(&buf[i..]);
}

// ── Test ──────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {

    write(b"[hello] write:  ok\n");

    write(b"[hello] pid:    = ");
    print_u64(getpid());
    write(b"\n");

    yield_cpu();
    write(b"[hello] yield:  ok\n");

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
