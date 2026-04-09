#!/bin/bash
set -e

cargo build
cp target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos
grub-mkrescue -o target/kernos.iso target/isodir
qemu-system-x86_64 -cdrom target/kernos.iso -boot d
