/*
    ══════════════════════════════════════════════════════════
    SYSCALL — Wrappers userspace pour int 0x80
    ══════════════════════════════════════════════════════════

    CONVENTION D'APPEL
    ──────────────────
    Le kernel attend (via syscall_stub dans interrupt.rs) :
      rax = numéro de syscall
      rdi = arg1  (rsi = arg2, rdx = arg3)

    Le stub réorganise en convention SysV avant d'appeler Rust :
      rdi ← rax (num)   rsi ← rdi (a1)
      rdx ← rsi (a2)    rcx ← rdx (a3)

    La valeur de retour arrive dans rax après iretq.
    inlateout("rax") num => ret capture cette valeur correctement.

    TABLE DES SYSCALLS
    ──────────────────
    0  write(fd, buf, len)    → len          fd 1/2 = stdout/stderr
    1  exit(code)             → !
    2  getpid()               → pid
    3  yield()                → 0            cède le CPU
    4  open(path, len)        → fd           fd 3..7 = KFS, MAX = erreur
    5  close(fd)              → 0 / MAX
    6  read(fd, buf, len)     → octets lus   avance fd.pos
    7  kill(pid, sig)          → 0 / MAX
    8  wait(pid)               → 0            bloque jusqu'à exit
    9  pipe()                  → (fd_r<<32)|fd_w
*/

// ── Wrappers syscall ──────────────────────────────────────────────────────
// Un seul wrapper générique : rax=num, rdi=a1, rsi=a2, rdx=a3.
// Le stub int 0x80 réarange vers les registres C et écrit la valeur de retour
// dans le slot rax sauvegardé -> inlateout capture le résultat.

/// Appel système brut : rax=num, rdi=a1, rsi=a2, rdx=a3.
/// Retourne la valeur dans rax après le retour du kernel (iretq).
#[inline(always)]
pub unsafe fn sys(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {

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


// ── Syscalls typés ────────────────────────────────────────────────────────

/// Écrit `buf` sur le terminal (fd=1).
pub fn write(s: &[u8]) {
    unsafe { sys(0, 1, s.as_ptr() as u64, s.len() as u64); }
}

/// Termine le processus courant. Ne revient jamais.
pub fn exit(code: u64) -> ! {
    unsafe { sys(1, code, 0, 0); }
    loop {}
}

/// Retourne l'index de la tâche courante dans le tableau TASKS du kernel.
pub fn getpid() -> u64                      { unsafe { sys(2, 0, 0, 0) } }

/// Cède le CPU au prochain processus actif (round-robin).
pub fn yield_cpu()                          { unsafe { sys(3, 0, 0, 0); } }

/// Ouvre un fichier KFS. Retourne un fd (3..7) ou u64::MAX si introuvable.
pub fn open(path: &[u8]) -> u64             { unsafe { sys(4, path.as_ptr() as u64, path.len() as u64, 0) } }

/// Ferme un fd précédemment ouvert par open().
pub fn close(fd: u64)                       { unsafe { sys(5, fd, 0, 0); } }

/// Lit jusqu'à buf.len() octets depuis fd à partir de la position courante.
/// Retourne le nombre d'octets effectivement lus (0 = fin de fichier).
pub fn read(fd: u64, buf: &mut [u8]) -> u64 { unsafe { sys(6, fd, buf.as_mut_ptr() as u64, buf.len() as u64) } }


// ── Utilitaires d'affichage ───────────────────────────────────────────────

/// Construit le résultat en base 10 en écrivant de droite à gauche dans un
/// buffer sur la pile, puis appelle write() une seule fois.
pub fn print_u64(mut n: u64) {

    if n == 0 { write(b"0"); return; }

    let mut buf = [0u8; 20];
    let mut i = 20usize;

    while n > 0  {

        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    write(&buf[i..]);
}

// ── IPC (Phase F) ─────────────────────────────────────────────────────────

pub const SIGKILL: u64 = 9;

/// Envoie un signal à un processus. SIGKILL(9) = termine immédiatement.
pub fn kill(pid: u64, sig: u64) -> u64  { unsafe { sys(7, pid, sig, 0) } }

/// BLoque jusqu'à ce que le processus 'pid' se termine.
pub fn wait(pid: u64)                    { unsafe { sys(8, pid, 0, 0); } }

/// Crée un pipe. Retourne (fd_read, fd_write) ou (MAX, MAX) si erreur.
pub fn pipe() -> (u64, u64) {

    let p = unsafe { sys(9, 0, 0, 0) };

    if p == u64::MAX { return (u64::MAX, u64::MAX); }

    (p >> 32, p & 0xFFFF_FFFF)
}

/// Lit une ligne depuis stdin (bloquant jusqu'au '\n').
/// Retourne le slice rempli sans le '\n' terminal.
pub fn readline(buf: &mut [u8]) -> &[u8] {

    let n = read(0, buf) as usize;
    // enleve le '\n' final si present
    if n > 0 && buf[n - 1] == b'\n' { &buf[..n - 1] } else { &buf[..n] }
}
