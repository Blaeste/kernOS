cargo build

Copy-Item target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos

if (-not (Test-Path "target/disk.img")) {
    & "C:\Program Files\qemu\qemu-img.exe" create -f raw target/disk.img 10M
}
if ($IsWindows) {
    wsl grub-mkrescue -o target/kernos.iso target/isodir
    & "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom target/kernos.iso -boot d -vga std -m 256 -drive file=target/disk.img,format=raw,index=0,media=disk -debugcon stdio
} else {
    grub-mkrescue -o target/kernos.iso target/isodir
    qemu-system-x86_64 -cdrom target/kernos.iso -boot d -vga std -m 256 -drive file=target/disk.img,format=raw,index=0,media=disk -debugcon stdio
}
