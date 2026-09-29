# virtio-rust-lab

A hands-on Rust lab for learning virtio by implementing it.
This starter deliberately contains no virtio implementation.

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

## Learning milestones

1. Model guest memory and validate descriptor addresses and lengths.
2. Implement a split virtqueue: descriptor table, available ring, used ring.
3. Process entropy requests for a minimal virtio-rng backend.
4. Implement device initialization, feature negotiation, and notifications.
5. Connect the backend to a guest using separate VM boot plumbing.
6. Test malformed descriptor chains, bounds, and queue index wraparound.

The learner writes the implementation. Guidance should favor small exercises,
acceptance tests, and hints before complete solutions.