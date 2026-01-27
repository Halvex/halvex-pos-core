# POS Core Schema Stability

This document defines the wire format and compatibility rules for JSON
serialization of POS Core commands/events and envelopes.

## Enum encoding

All public enums are tagged with the same scheme:

```
{"kind": "VariantName", "data": { ... }}
```

This includes (non-exhaustive):
- CoreCommand / WorkflowCommand / CoreEvent
- OrderCommand / OrderEvent
- CheckCommand / CheckEvent
- TableCommand / TableEvent
- AggregateType and public status/payment/KDS enums

## Schema version

`CommandEnvelope` and `EventEnvelope` include a top-level `schema_version`.

- Current version: `SCHEMA_VERSION = 2`
- If `schema_version` is missing during deserialization, it defaults to the
  current `SCHEMA_VERSION` (backwards compatibility for older payloads).

## Event sequencing

`EventEnvelope.seq` is a per-aggregate sequence number:
- It MUST equal the aggregate version AFTER applying the event.
- It is NOT a global stream position.
- See `docs/storage_contract.md` for append and versioning expectations.

## Compatibility rules

Strict decoding is the default. Unknown enum variants cause a deserialization
error. When making breaking changes, bump `SCHEMA_VERSION`.

Non-breaking changes:
- Add optional fields with defaults.
- Add new event/command variants ONLY if all clients are updated or the schema
  version is bumped.

Breaking changes (bump schema_version):
- Rename or remove variants.
- Remove or rename fields.
- Change field meanings or types.

See `docs/versioning.md` for versioning policy and supported ranges.
