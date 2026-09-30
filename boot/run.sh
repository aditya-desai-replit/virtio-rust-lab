#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cache="$root/.cache/boot"
vmm="$cache/vmm-reference/target/debug/vmm-reference"
kernel="$cache/vmlinux-hello-busybox"
[[ -x "$vmm" && -f "$kernel" ]] ||
    { echo "Run bash boot/setup.sh first." >&2; exit 1; }
[[ -r /dev/kvm && -w /dev/kvm ]] ||
    { echo "Read/write access to /dev/kvm is required." >&2; exit 1; }
if [[ -t 0 ]]; then
    saved_tty="$(stty -g)"
    trap 'stty "$saved_tty"' EXIT
fi
"$vmm" \
    --memory size_mib=256 \
    --vcpu num=1 \
    --kernel "path=$kernel,cmdline=console=ttyS0 i8042.nokbd reboot=t panic=1 pci=off"