#!/bin/bash
set -e

# Compile user/hello ELF avant le kernel (requis par include_bytes!)
(cd user/hello && cargo build)

cargo build
cp target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos
if [ ! -f target/disk.img ]; then
    qemu-img create -f raw target/disk.img 10M
fi
grub-mkrescue -o target/kernos.iso target/isodir
qemu-system-x86_64 -cdrom target/kernos.iso -boot d -vga std -m 256 -drive file=target/disk.img,format=raw,index=0,media=disk -debugcon stdio