# POS Core Overview

This document describes the full POS Core application: its domain model,
command/event flow, workflows, KDS subsystem, and host integration
responsibilities.

## Purpose and scope

POS Core is a pure domain library. It focuses on:

- deterministic command validation and event emission
- explicit aggregates and workflows
- stable JSON schema for cross-platform clients
- KDS ticket projection and printing deltas

It intentionally does **not** provide persistence, transport, auth, or UI.

## Aggregates and domain model

POS Core models three primary aggregates:

- **Order** (`crates/pos-core/src/orders`): itemized order data
- **Check** (`crates/pos-core/src/checks`): guest-facing bill with payments
- **Table** (`crates/pos-core/src/tables`): table occupancy and assignment

Each aggregate has:

- `commands.rs`: API surface for intent
- `events.rs`: immutable changes
- `model.rs`: state
- `rules.rs`: validation helpers

State changes are applied via `engine::Apply` in
`crates/pos-core/src/engine/aggregate.rs`.

## Command routing and envelopes

### Core routing

`core::route_core_command` takes a `CoreContext` containing the aggregates
loaded by the host. It returns a list of `CoreEvent`s:

- `OrderCommand` -> `OrderEvent`
- `CheckCommand` -> `CheckEvent`
- `TableCommand` -> `TableEvent`
- `WorkflowCommand` -> multiple events across aggregates

### Envelopes

`CommandEnvelope` and `EventEnvelope` wrap commands/events for transport and
sync. They include:

- `schema_version` for JSON compatibility
- `idempotency_key` for dedupe by the host
- `correlation_id` and `causation_id` for tracing
- `actor_id` for audit trail

`EventEnvelope.seq` is **per-aggregate** and should be set by the host after
persisting events. See `docs/schema.md` for full rules.

## Workflows (cross-aggregate)

Workflows are modeled explicitly as `WorkflowCommand`:

- **Split check create**: opens a new check for the same order and emits
  `SplitCheckCreated`.
- **Split check move line qty**: decrements quantity on the source check and
  adds a line to the destination check.
- **Open table session**: opens order + check and assigns the order to a table.
- **Close table session**: validates payment, closes the check if open, and
  unassigns the order from the table.

Workflows require multiple aggregates in `CoreContext`.

## Checks, payments, and totals

Checks are responsible for pricing and payments:

- `checks::totals` computes subtotal, discounts, service charge, total, paid,
  tip, and balance due.
- Payments track `authorised`, `captured`, and `refunded` amounts.
- Validation rules in `checks::rules` enforce currency consistency and payment
  lifecycle invariants.

## Projections (read helpers)

`crates/pos-core/src/projections` provides lightweight read helpers:

- `can_close_check` and `check_balance_due`
- `table_is_available`

These are pure helpers built on top of the domain model.

## KDS subsystem

The KDS pipeline converts order events into station-specific tickets:

1. The host feeds `EventEnvelope`s into `apply_core_event_to_kds`.
2. `OrderEvent::ItemAdded` creates or updates a ticket line at a station.
3. `OrderEvent::ItemRemoved` voids a line.
4. KDS UI actions emit `KdsEvent`s that update ticket/line statuses.

Routing is defined by `RoutingTable` rules (by menu item or name prefix). Each
ticket is keyed by `(order_id, station)` and uses deterministic IDs to remain
stable across replays.

Printing support:

- `tickets_for_printing` returns full tickets for a station.
- `delta_tickets_since` returns deltas since a cursor for incremental printing.

See `crates/pos-core/src/kds` for details.

## Host responsibilities

The host application is expected to:

- load aggregates into `CoreContext`
- enforce idempotency using `(aggregate_ref, idempotency_key)`
- persist emitted events and assign per-aggregate `seq`
- apply events via `Apply` to advance state
- provide `actor_id` and human-readable `reason` for audit

Audit guidance is in `docs/audit.md`.

## Testing and invariants

Tests in `crates/pos-core/src/tests` cover:

- end-to-end flows (`mvp_flow.rs`, `order_flow.rs`, `check_flow.rs`)
- KDS projections and deltas
- property-based invariants (see `tests/proptests`)

Run all tests with:

```bash
cargo test
```

## Extending the core

When adding commands or events:

1. Update the relevant `commands.rs` / `events.rs`.
2. Add validation rules and apply logic.
3. Update schema compatibility rules (`docs/schema.md`).
4. Add tests for new behavior.
