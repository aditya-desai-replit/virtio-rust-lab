#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cache="$root/.cache/boot"
vmm="$cache/vmm-reference"
revision=89e4c8ba56b553eeefe53ab318a67b870a8b8e41
kernel="$cache/vmlinux-hello-busybox"
kernel_url=https://vmm-reference-test-resources.s3.amazonaws.com/v5/kernel/vmlinux-hello-busybox
kernel_sha=603efe3e8fabe5a9084be65f20c6398eb31ddb5a1f2dffd193d4ee3e0590d89c

[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] ||
    { echo "This boot harness requires x86-64 Linux." >&2; exit 1; }
for tool in git curl cargo rustc cc sha256sum python3; do
    command -v "$tool" >/dev/null ||
        { echo "Missing tool: $tool. Run bash dev-shell first." >&2; exit 1; }
done
python3 - <<'PY'
import fcntl
import os
import sys
try:
    kvm = os.open("/dev/kvm", os.O_RDWR | os.O_CLOEXEC)
    try:
        if fcntl.ioctl(kvm, 0xAE00, 0) != 12:
            raise RuntimeError("Unsupported KVM API version")
        vm = fcntl.ioctl(kvm, 0xAE01, 0)
        os.close(vm)
    finally:
        os.close(kvm)
except (OSError, RuntimeError) as error:
    sys.exit(f"KVM preflight failed: {error}. Use a workspace with working /dev/kvm access.")
print("KVM preflight passed.")
PY
mkdir -p "$cache"
if [[ ! -d "$vmm" ]]; then
    git init "$vmm"
    git -C "$vmm" remote add origin https://github.com/rust-vmm/vmm-reference.git
fi
if ! git -C "$vmm" rev-parse --verify HEAD >/dev/null 2>&1; then
    git -C "$vmm" fetch --depth 1 origin "$revision"
    git -C "$vmm" checkout --detach "$revision"
fi
[[ "$(git -C "$vmm" rev-parse HEAD)" == "$revision" ]] ||
    { echo "Unexpected VMM revision; refusing to replace your checkout." >&2; exit 1; }
[[ -z "$(git -C "$vmm" status --porcelain)" ]] ||
    { echo "VMM checkout has changes; refusing to build a modified baseline." >&2; exit 1; }

if [[ ! -f "$kernel" ]]; then
    tmp="$(mktemp "$cache/kernel-download.XXXXXX")"
    trap 'rm -f "$tmp"' EXIT
    curl -fL --retry 2 --max-time 180 "$kernel_url" -o "$tmp"
    printf '%s  %s\n' "$kernel_sha" "$tmp" | sha256sum -c -
    mv "$tmp" "$kernel"
    trap - EXIT
fi
printf '%s  %s\n' "$kernel_sha" "$kernel" | sha256sum -c -
(cd "$vmm" && cargo build --locked)
echo "Ready. Boot with: bash boot/run.sh"