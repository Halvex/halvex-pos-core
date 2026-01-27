# POS Core Storage Contract

This document describes the host-facing event storage expectations for POS Core.

## Append-only event streams

Events are stored as ordered, append-only streams keyed by `AggregateRef`
(`AggregateType + id`). Hosts must never mutate or delete historical events.
Rebuilding state is done by replaying the stream in order.

## Replay and ordering

- `EventEnvelope.seq` is the per-aggregate version *after* applying the event.
- Sequence numbers start at 1 for the first event in a stream.
- Streams are replayed in ascending `seq` order.
- `seq` is **not** a global stream position.

Example: if the current version is 7 and two events are appended, they receive
`seq` 8 and 9, and the new version is 9.

## Optimistic concurrency (`ExpectedVersion`)

When appending events, the host should use optimistic concurrency to prevent
conflicting writes:

- `ExpectedVersion::NoStream` -> stream must be empty (current version 0)
- `ExpectedVersion::Exact(v)` -> stream must be at version `v`
- `ExpectedVersion::Any` -> no version check (not recommended for live writes)

If the expectation fails, the store should return a conflict error and the host
should reload/retry.

## Idempotency

`CommandEnvelope.idempotency_key` is used to dedupe client retries. The host
should store the resulting `EventEnvelope` list under the key
`(aggregate_ref, idempotency_key)`:

- On receive, look up the key first.
- If present, return the stored events (no new append).
- If absent, append events and then store them for the key.

This guarantees repeated commands return the same event list.

## Audit notes

Audit fields (`actor_id`, `reason`, `causation_id`, `correlation_id`) must be
preserved verbatim. Do not drop or rewrite them during storage or replay.

## Multi-aggregate workflows and atomicity

Workflow commands may emit events across multiple aggregates. The host **must**
commit all involved streams atomically (transaction) or roll back the entire
operation. Partial commits will corrupt replay and violate core invariants.

If your storage does not support multi-stream transactions, use an application-
level outbox/transactional write pattern to ensure all streams commit together.
