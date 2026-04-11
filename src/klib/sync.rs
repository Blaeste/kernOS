use core::sync::atomic::{AtomicBool, Ordering};

/// Spinlock simple basé sur un AtomicBool.
/// Lock() tourne en boucle jusqu'a obtenir le verrou.
/// unlock() libere le verrou.
pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        SpinLock { locked: AtomicBool::new(false) }
    }

    /// Acquiert le verrou (spin jusqu'a disponibilité)
    pub fn lock(&self) {
        unsafe { core::arch::asm!("cli", options(nostack)); }
        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
    }

    /// Libère le verrou
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
        unsafe { core::arch::asm!("sti", options(nostack)); }
    }

     /// Libère le verrou
    pub fn force_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Execute une closure sosu verrou et retourne le résultat
    pub fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        self.lock();
        let r = f();
        self.unlock();
        r
    }
}