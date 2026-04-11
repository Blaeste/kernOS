/// GDT + TSS — nécessaire pour ring 3 et isolation des piles

use core::arch::asm;
use core::ptr::addr_of;

/// TSS (Task State Segment) - le CPU y lit la pile kernel (RSP0)
/// quand il reçoit un interruption depuis ring 3
#[repr(C, packed)]
pub struct Tss {
    _reserved0:         u32,
    pub rsp0:           u64,    // pile kernel - utilisé par le CPU lors d'un int depuis ring 3
    pub rsp1:           u64,
    pub rsp2:           u64,
    _reserved1:         u64,
    pub ist1:           u64,    // Interrupt Stack Table (optionnel)
    pub ist2:           u64,
    pub ist3:           u64,
    pub ist4:           u64,
    pub ist5:           u64,
    pub ist6:           u64,
    pub ist7:           u64,
    _reserved2:         u64,
    _reserved3:         u16,
    pub iomap_base:     u16,
}

/// GDT : 5 segments + 1 TSS (2 entrées car 16 octets en 64-bit)
/// [0] NULL [1] Code kernel [2] Data kernel [3] Code user [4] Data user [5-6] TSS
#[repr(C, packed)]
pub struct Gdt {
    pub null:           u64,
    pub code_kernel:    u64,
    pub data_kernel:    u64,
    pub code_user:      u64,
    pub data_user:      u64,
    pub tss_low:        u64,
    pub tss_high:       u64,
}

/// Sélecteur de segment (index * 8 + RPL)
pub const KERNEL_CODE:  u16 = 0x08;         // GDT[1], RPL=0
pub const KERNEL_DATA:  u16 = 0x10;         // GDT[2], RPL=0
pub const USER_CODE:    u16 = 0x18 | 3;     // GDT[3], RPL=3
pub const USER_DATA:    u16 = 0x20 | 3;     // GDT[4], RPL=3
pub const TSS_SEL:      u16 = 0x28;         // GDT[5]

pub static mut TSS: Tss = Tss {
    _reserved0: 0,
    rsp0: 0, rsp1: 0, rsp2: 0,
    _reserved1: 0,
    ist1: 0, ist2: 0, ist3: 0, ist4: 0,
    ist5: 0, ist6: 0, ist7: 0,
    _reserved2: 0, _reserved3: 0,
    iomap_base: 104,    // taille du TSS
};

pub static mut GDT: Gdt = Gdt {
    null:           0,
    code_kernel:    0,
    data_kernel:    0,
    code_user:      0,
    data_user:      0,
    tss_low:        0,
    tss_high:       0,
};

/// Initialise la GDT + TSS et le schage dans le CPU
pub fn init(kernel_stack_top: u64) {
    unsafe {
        // RSP0 - pile kernel (le CPU switch ici depusi le ring 3)
        TSS.rsp0 = kernel_stack_top;

        let tss_addr = addr_of!(TSS) as u64;
        let tss_size = (core::mem::size_of::<Tss>() - 1) as u64;

        // Segments kernel: code=execute/read, data=read/write
        GDT.code_kernel = (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53);
        GDT.data_kernel = (1 << 41) | (1 << 44) | (1 << 47);

        // Segments user: comme kernel mais DPL-3 (bits 45-46)
        GDT.code_user = (1 << 43) | (1 << 44) | (1 << 45) | (1 << 46) | (1 << 47) | (1 << 53);
        GDT.data_user = (1 << 41) | (1 << 44) | (1 << 45) | (1 << 46) | (1 << 47);
        
        // TSS desciptor (16 octets, split en 2 entrées de 8)
        GDT.tss_low = tss_size & 0xFFFF
            | ((tss_addr & 0xFFFF) << 16)
            | (((tss_addr >> 16) & 0xFF) << 32)
            | (0x89u64 << 40)  // present + type=available 64-bit TSS
            | (((tss_size >> 16) & 0xF) << 48)
            | (((tss_addr >> 24) & 0xFF) << 56);
        GDT.tss_high = tss_addr >> 32;

        // Charge la GDT
        let gdt_ptr = GdtPointer {
            limit: (core::mem::size_of::<Gdt>() - 1) as u16,
            base: addr_of!(GDT) as u64,
        };
        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(nostack));

        // Charge le TSS
        asm!("ltr {0:x}", in(reg) TSS_SEL, options(nostack));

        // Recharge les segments
        asm!(
            "mov ds, {kd:x}",
            "mov es, {kd:x}",
            "mov fs, {kd:x}",
            "mov gs, {kd:x}",
            "mov ss, {kd:x}",
            kd = in(reg) KERNEL_DATA,
            options(nostack)
        );

        // Far return pour recharger CS avec le nouveau code segment
        asm!(
            "push {cs}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            cs = in(reg) KERNEL_CODE as u64,
            out("rax") _,
        );
    }
}

#[repr(C,packed)]
struct GdtPointer {
    limit:  u16,
    base:   u64,
}