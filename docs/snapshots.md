# POS Core Snapshots

This document describes snapshotting strategies for hosts integrating POS Core.

## Why snapshots

Event streams are append-only and can grow large. Snapshots allow faster rebuilds
by storing the latest aggregate state plus its per-aggregate version.

## Snapshot shape

Use `Snapshot<T>`:

```
Snapshot<T> { aggregate, version, state }
```

- `aggregate` is the aggregate stream (`AggregateRef`).
- `version` is the per-aggregate version after applying all events in `state`.
- `state` is the serialized aggregate state.

## Rebuild strategy

1. Load the latest snapshot for an aggregate (if any).
2. Load tail events with `seq > snapshot.version`.
3. Apply tail events in order to the snapshot state.

If no snapshot is available, rebuild from the full event stream.

## Version rules

`EventEnvelope.seq` is the per-aggregate version after applying each event.
Snapshots must store the last applied version, and replay must only apply events
with higher `seq` values.

## KDS snapshots

KDS state is derived from `EventEnvelope`s. If you snapshot KDS, you should
pair it with a cursor (last processed seq) and only replay newer events.

## Host responsibilities

- Store snapshots separately from the append-only event stream.
- Ensure snapshot state and version are consistent.
- Use transactions when saving a snapshot alongside new events.
- Never mutate historical events.
