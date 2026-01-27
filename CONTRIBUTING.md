# Contributing to HalvexPOS Core

Thanks for your interest in contributing!

## Development setup

- Install Rust (stable) via rustup
- Install toolchain components:

```bash
rustup component add rustfmt clippy
```

## Coding style

- Format with `cargo fmt`
- Keep changes focused and readable
- Add or update tests when behavior changes

## Commit messages

Use clear, concise commit messages. Conventional commits are encouraged but not
required.

## Running checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Adding features

Please open an issue before starting significant work to align on design.
Small fixes can go straight to a PR.

## CLA

No CLA is required at this time. By contributing, you agree that your
contributions will be licensed under the Apache License 2.0.

## Before opening a PR

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] Updated docs/tests as needed
