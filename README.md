# virtio-rust-lab

A hands-on Rust lab for learning virtio by implementing it.
The exercises implement a split virtqueue incrementally, with a separate Linux
boot harness for later device integration.

## Development

With Rust installed:

```sh
cargo run
cargo test
```

In the Nix sandbox:

```sh
nix-shell -I nixpkgs=flake:nixpkgs
cargo run
cargo test
```

KVM exercises require Linux and read/write access to `/dev/kvm`.
The initial standalone virtqueue exercises do not require KVM.

## Boot Linux

On x86-64 Linux with `/dev/kvm` access, Git, curl, a Rust toolchain, a C linker,
and Python 3:

```sh
bash boot/setup.sh
bash boot/run.sh
```

At the guest's `/ #` prompt, try `uname -r` or `cat /proc/cmdline`.
Type `reboot -f` to exit back to the host terminal.
Type commands normally: large pastes can overrun the test VMM's serial FIFO.

Run the automated boot, guest-command, and shutdown check:

```sh
python3 boot/smoke_test.py
```

The harness boots an unmodified, pinned `rust-vmm/vmm-reference` with one vCPU,
256 MiB RAM, and the upstream Linux 5.4.81/BusyBox test image. Downloads, build
artifacts, and the latest `boot.log` live under `.cache/boot/`, excluded from Git.
Guest filesystem changes are ephemeral; no network or disk is attached.
**Our virtio-rng implementation is not connected to this guest yet.**

This old kernel and experimental VMM are only for an isolated learning lab,
not production or untrusted workloads. Guest virtio-mmio/virtio-rng support
must be checked separately before device integration.

VMM source: <https://github.com/rust-vmm/vmm-reference>

The commit and kernel URL/checksum are pinned in `boot/setup.sh`. The kernel
checksum was recorded from the initial download; it detects subsequent changes
but is not an independently authenticated signature.

## Learning milestones

1. Model guest memory and validate descriptor addresses and lengths.
2. Implement a split virtqueue: descriptor table, available ring, used ring.
3. Process entropy requests for a minimal virtio-rng backend.
4. Implement device initialization, feature negotiation, and notifications.
5. Connect the backend to a guest using separate VM boot plumbing.
6. Test malformed descriptor chains, bounds, and queue index wraparound.

The learner writes the implementation. Guidance should favor small exercises,
acceptance tests, and hints before complete solutions.