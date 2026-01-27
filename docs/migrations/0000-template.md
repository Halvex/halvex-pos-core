# Schema migration template

## From version: X -> Y

## Summary

- Describe the breaking wire change.

## Envelope changes

- List added/removed/renamed fields.
- List enum variant changes.

## Migration strategy

- If supporting old versions, describe adapter/translation steps.
- If not supporting old versions, describe rollout plan.

## Snapshot updates

- Update `insta` snapshots in `tests/schema_snapshots.rs`.
- Update any generated client schema if applicable.
