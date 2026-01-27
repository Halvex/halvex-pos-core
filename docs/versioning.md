# POS Core Versioning

This document defines JSON schema versioning and migration rules for POS Core.

## Schema constants

- `SCHEMA_VERSION`: current wire format version.
- `MIN_SUPPORTED_SCHEMA_VERSION` and `MAX_SUPPORTED_SCHEMA_VERSION` define the
  accepted decode range. Default policy supports only the current version.

Use `ensure_supported_schema(version)` at ingress/egress boundaries to reject
payloads that are too old or too new.

## Breaking vs non-breaking changes

Non-breaking (compatible):
- Add optional fields with serde defaults.
- Add new enum variants only if all clients are updated together or the schema
  version is bumped.

Breaking (requires bump):
- Rename or remove enum variants.
- Remove or rename fields.
- Change field types or semantics.
- Change enum tagging or envelope structure.

## When to bump `SCHEMA_VERSION`

Bump `SCHEMA_VERSION` for any breaking wire change. Record the change in
`docs/migrations/` and update snapshot tests.

## Supported versions

Default policy: only the current schema version is supported. If you need to
support multiple versions, widen `MIN_SUPPORTED_SCHEMA_VERSION` and implement
migration adapters at your boundary.
