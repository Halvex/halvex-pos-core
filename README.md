# HalvexPOS Core

HalvexPOS Core is a pure Rust domain core for a point-of-sale system. It models
orders, checks, tables, and kitchen display workflows as commands and events.
The core is storage-agnostic and transport-agnostic: a host application loads
aggregate state, routes commands, persists emitted events, and replays events
to rebuild state.

## Workspace layout

- `crates/pos-types`: ID newtypes, `Money`, and `PosError`.
- `crates/pos-core`: domain logic (orders/checks/tables), command routing,
  workflows, projections, and the KDS subsystem.
- `docs/overview.md`: full architecture and integration guide.
- `docs/schema.md`: JSON wire format and compatibility rules.
- `docs/audit.md`: audit metadata and reasons.

## Core concepts

- **Aggregates**: `Order`, `Check`, `Table` are versioned state machines.
- **Commands -> Events**: `engine::handle_*_command` validates current state and
  emits events.
- **Event application**: `engine::Apply` mutates aggregate state from events.
- **Workflows**: `WorkflowCommand` handles cross-aggregate actions (check
  splits, table sessions).
- **Envelopes**: `CommandEnvelope`/`EventEnvelope` carry schema version,
  idempotency keys, and tracing metadata.

## Integrating in a host app

1. Load current aggregates (from an event store or snapshot).
2. Call `route_core_command` or `route_command_envelope_enveloped`.
3. Persist emitted events, assign per-aggregate `seq` values.
4. Apply events using `Apply` to advance state.

See `docs/overview.md` for detailed flows and responsibilities.

## KDS

The KDS subsystem projects order events into station-specific tickets. It
supports routing rules, per-line status, and delta printing with cursors. See
`crates/pos-core/src/kds` and `docs/overview.md`.

## Tests

Run all tests:

```bash
cargo test
```

Key flows live in `crates/pos-core/src/tests` (e.g. `mvp_flow.rs`).

## Support
If there are any issues, or questions please contact pos@halvex.net, or open a ticket within
the official Halvex discord server.
