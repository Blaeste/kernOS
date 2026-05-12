#![no_std]
#![no_main]

const MSG: &[u8] = b"Hello from ELF!\n";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        core::arch::asm!(
            "mov rax, 0",
            "mov rdi, 1",
            "mov rdx, {len}",
            "int 0x80",
            len = const 16usize,
            in("rsi") MSG.as_ptr(),
            options(nostack)
        );
        core::arch::asm!(
            "mov rax, 1",
            "xor rdi, rdi",
            "int 0x80",
            options(nostack, noreturn)
        );
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
