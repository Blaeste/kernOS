/// Table des appels système kernel

use core::ptr::{addr_of, addr_of_mut};
use crate::klib::string::to_str_or;
use crate::task::scheduler::{CURRENT, TASKS, MAX_FDS};

// ── Numéros de syscall ────────────────────────────────────────────────────

pub const SYS_WRITE:        u64 = 0;    // write(fd, buf, len)
pub const SYS_EXIT:         u64 = 1;    // exit(code)
pub const SYS_GETPID:       u64 = 2;    // getpid() -> pid
pub const SYS_YIELD:        u64 = 3;    // yield()
pub const SYS_OPEN:         u64 = 4;    // open(path_ptr, path_len) -> fd
pub const SYS_CLOSE:        u64 = 5;    // close(fd)
pub const SYS_READ:         u64 = 6;    // read(fd, buf, len) -> bytes lus

// ── Dispatcher ───────────────────────────────────────────────────────────

/// Handler principal - appelé depuis le stub int 0x80
/// rax = numéro, rdi = arg1, rsi = arg2, rdx = arg3
pub extern "C" fn syscall_handler(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    // Re-active les interruptions : les syscalls doivent etre preemptibles
    // pour eviter un deadlock sur PRINT_LOCK si le timer avait switche mid-print.
    unsafe { core::arch::asm!("sti", options(nostack)); }
    match num {
        SYS_WRITE   => sys_write(a1, a2, a3),
        SYS_EXIT    => sys_exit(a1),
        SYS_GETPID  => sys_getpid(),
        SYS_YIELD   => sys_yield(),
        SYS_OPEN    => sys_open(a1, a2),
        SYS_CLOSE   => sys_close(a1),
        SYS_READ    => sys_read(a1, a2, a3),
        _           => u64::MAX,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// syscall 0 : write(fd, buf_ptr, len)
/// fd 1/2 = stdout:stderr (terminal)
fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> u64 {

    // Pour le moment que stdout et stderr
    if fd != 1  && fd != 2 { return u64::MAX; }

    let slice = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len as usize)
    };

    let s = to_str_or(slice, "");
    if !s.is_empty() {
        crate::print!("{}", s);
    }

    // ret len
    len
}

/// syscall 1 : exit(code)
fn sys_exit(_code: u64) -> u64 {
    crate::task::scheduler::task_exit();
    0
}

/// syscall 2: getpid() -> index de la tâche courante
fn sys_getpid() -> u64 {
    unsafe { *addr_of!(CURRENT) as u64 }
}

/// syscall 3: yield() - cède le CPU au prochain processus
fn sys_yield() -> u64 {
    unsafe { core::arch::asm!("int 0x20", options(nostack)); }
    0
}

/// syscall 4: open(path_ptr, path_len) -> fd ou u64::MAX
///
/// Ouvre un ficheir KFS, Réserve un slot fd (3..7) dans la table du processus courant.
/// Retourne u64::MAX si le fichier n'existe pas ou si tous les slots sont pris.
fn sys_open(path_ptr: u64, path_len: u64) -> u64 {

    let len = (path_len as usize).min(28);
    let name_slice = unsafe {
        core::slice::from_raw_parts(path_ptr as *const u8, len)
    };

    // Vérifie que le fichier existe dans KFS avant de réserver un slot
    let mut probe = [0u8; 512];
    if crate::fs::kfs::read(name_slice, &mut probe) == 0 {
        return u64::MAX;
    }

    // Cherche un slot libre (fd (3..7 - 0/1/2 réservé systeme)
    let cur = unsafe { *addr_of!(CURRENT) };
    let fds = unsafe { &mut (*addr_of_mut!(TASKS))[cur].fds };

    for i in 3..MAX_FDS {
        if !fds[i].active {
            fds[i].active   = true;
            fds[i].pos      = 0;
            // Copie le nom
            let dst = &mut fds[i].name;
            for j in 0..len { dst[j] = name_slice[j]; }
            for j in len..28 { dst[j] = 0; }
            return i as u64; 
        }
    }
    u64::MAX // plus de slots
}

/// syscall 5 : close(fd)
fn sys_close(fd: u64) -> u64 {

    let idx = fd as usize;
    if idx < 3 || idx >= MAX_FDS { return u64::MAX; }

    let cur = unsafe { *addr_of!(CURRENT) };
    let fds = unsafe { &mut (*addr_of_mut!(TASKS))[cur].fds };

    if !fds[idx].active { return u64::MAX; }

    fds[idx].active = false;
    0    
}

/// syscall 6: read(fd, buf_ptr, len) -> octet lus
///
/// fd 0 (stdin)    : non implémenté, retourne 0.
/// fd 3..7         : lit depusi KFS à partir de la position courante (fd.pos).
fn sys_read(fd: u64, buf_ptr: u64, len: u64) -> u64 {

    if fd == 0 { return 0 }
    if fd < 3 || fd as usize >= MAX_FDS { return u64::MAX; }

    let idx = fd as usize;
    let cur = unsafe { *addr_of!(CURRENT) };

    // Lecture KFS (fichier entier dans un buffer temporaire)
    let name    = unsafe { (*addr_of!(TASKS))[cur].fds[idx].name };
    let pos     = unsafe { (*addr_of!(TASKS))[cur].fds[idx].pos };

    if !unsafe { (*addr_of!(TASKS))[cur].fds[idx].active } {
        return u64::MAX;
    }

    // Trim les zéros trailing du nom stocké — kfs::read_at fait une comparaison exacte
    let name_len = name.iter().position(|&b| b == 0).unwrap_or(28);

    let mut kbuf = [0u8; 512];
    let file_size = crate::fs::kfs::read_at(&name[..name_len], pos, &mut kbuf);
    if file_size == 0 || pos >= file_size { return 0; }

    let available = file_size - pos;
    let to_copy = (len as usize).min(available);

    // Copie vers le buffer user (accessible : kernel tourne dans CR3 user)
    let dst = unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, to_copy)
    };

    dst.copy_from_slice(&kbuf[pos..pos + to_copy]);

    unsafe { (*addr_of_mut!(TASKS))[cur].fds[idx].pos += to_copy; }

    to_copy as u64
}