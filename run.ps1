cargo build
cp target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos
wsl grub-mkrescue -o target/kernos.iso target/isodir
& "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom target/kernos.iso -boot d