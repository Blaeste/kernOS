cargo build

Copy-Item target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos

if ($IsWindows) {
    wsl grub-mkrescue -o target/kernos.iso target/isodir
    & "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom target/kernos.iso -boot d
} else {
    grub-mkrescue -o target/kernos.iso target/isodir
    qemu-system-x86_64 -cdrom target/kernos.iso -boot d
}
