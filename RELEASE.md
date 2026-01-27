# Release Process

This project follows SemVer for Rust API changes and a separate
`SCHEMA_VERSION` for JSON wire compatibility.

## Versioning policy

- Rust API breaking changes => major version bump.
- JSON wire breaking changes => bump `SCHEMA_VERSION` and document in
  `docs/migrations/`.

## Release steps

1. Update version(s) as needed.
2. Update changelog (if present) and migration notes.
3. Run checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

4. Tag the release:

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
```

5. Push tags:

```bash
git push --tags
```

## Publishing (optional)

If publishing to crates.io, ensure all metadata is correct and run:

```bash
cargo publish -p pos-core
```
