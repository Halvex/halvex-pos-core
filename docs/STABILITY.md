# POS Core Stability Policy

This document describes the compatibility rules for the Rust public API and the
JSON wire format used by POS Core.

## Rust API stability

The stable Rust surface is the set of items exported from the crate root and
`prelude`.

Rules:
- Public commands/events, envelopes, and projection structs are marked
  `#[non_exhaustive]`. Consumers must include a wildcard arm when matching
  enums and avoid struct literals in favor of constructors or helpers.
- Adding new fields or enum variants is considered additive, but it may still
  require updating downstream clients.
- Removing, renaming, or changing the meaning of public items is a breaking
  change and requires a major version bump.

## JSON wire format stability

All public enums use an adjacently tagged format:

```
{"kind": "VariantName", "data": { ... }}
```

Rules:
- `CommandEnvelope.schema_version` and `EventEnvelope.schema_version` are
  required when writing. When missing during deserialization, they default to
  the current `SCHEMA_VERSION` for backwards compatibility.
- `actor_id` and metadata-style fields are optional and default to `null` when
  absent.
- Unknown enum variants are rejected during deserialization.

### Additive changes (compatible)
- Add new optional fields with serde defaults.
- Add new enum variants only if all clients are updated together or the schema
  version is bumped.

### Breaking changes (bump `SCHEMA_VERSION`)
- Rename or remove enum variants.
- Remove or rename fields.
- Change field types or semantics.
- Change enum tagging or envelope structure.

## Versioning

- Bump `SCHEMA_VERSION` for JSON breaking changes.
- Follow Rust semver for API breaking changes.
