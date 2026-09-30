#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cache="$root/.cache/boot"
vmm="$cache/vmm-reference"
revision=89e4c8ba56b553eeefe53ab318a67b870a8b8e41
kernel="$cache/vmlinux-hello-busybox"
kernel_url=https://vmm-reference-test-resources.s3.amazonaws.com/v5/kernel/vmlinux-hello-busybox
kernel_sha=603efe3e8fabe5a9084be65f20c6398eb31ddb5a1f2dffd193d4ee3e0590d89c

[[ "$(uname -m)" == x86_64 ]] || { echo "This boot harness requires x86-64." >&2; exit 1; }
for tool in git curl cargo rustc cc sha256sum; do
    command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
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