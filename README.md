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

See `docs/overview.md`, `docs/storage_contract.md`, and `docs/schema.md` for
the detailed flows, storage contract, and JSON schema rules.

## Host integration skeleton (in-memory)

Copy-paste starting point for an in-memory event store + replay + routing loop.
This is a trimmed version of `crates/pos-core/examples/golden_flow.rs`:

```rust
use std::collections::HashMap;

use pos_core::{
    rebuild_check_from_events, rebuild_order_from_events, rebuild_table_from_events,
    route_command_envelope_enveloped, AggregateRef, AggregateType, Check, CommandEnvelope,
    CoreCommand, CoreContext, CoreEvent, EventEnvelope, EventStore, ExpectedVersion, Order, Table,
};
use pos_types::{CheckID, OrderID, TableID};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct AggregateKey {
    ty: u8,
    id: Uuid,
}

fn aggregate_type_key(ty: AggregateType) -> u8 {
    match ty {
        AggregateType::Order => 1,
        AggregateType::Check => 2,
        AggregateType::Table => 3,
    }
}

impl From<AggregateRef> for AggregateKey {
    fn from(value: AggregateRef) -> Self {
        Self {
            ty: aggregate_type_key(value.ty),
            id: value.id,
        }
    }
}

struct InMemoryEventStore {
    streams: HashMap<AggregateKey, Vec<EventEnvelope>>,
    idempotency: HashMap<(AggregateKey, String), Vec<EventEnvelope>>,
}

impl InMemoryEventStore {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            idempotency: HashMap::new(),
        }
    }

    fn current_version(&self, aggregate: AggregateRef) -> u64 {
        self.streams
            .get(&AggregateKey::from(aggregate))
            .and_then(|stream| stream.last())
            .and_then(|env| env.seq)
            .unwrap_or(0)
    }

    fn append_envelopes_in_order(
        &mut self,
        envelopes: Vec<EventEnvelope>,
    ) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
        let mut out = Vec::with_capacity(envelopes.len());
        for mut env in envelopes {
            let aggregate = env.aggregate;
            let key = AggregateKey::from(aggregate);
            let next_version = self.current_version(aggregate) + 1;
            env.seq = Some(next_version);
            self.streams.entry(key).or_default().push(env.clone());
            out.push(env);
        }
        Ok(out)
    }
}

impl EventStore for InMemoryEventStore {
    fn load_stream(
        &self,
        aggregate: AggregateRef,
    ) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
        Ok(self
            .streams
            .get(&AggregateKey::from(aggregate))
            .cloned()
            .unwrap_or_default())
    }

    fn append_to_stream(
        &mut self,
        aggregate: AggregateRef,
        expected: ExpectedVersion,
        mut events: Vec<EventEnvelope>,
    ) -> Result<u64, pos_core::PosError> {
        let current_version = self.current_version(aggregate);
        match expected {
            ExpectedVersion::Any => {}
            ExpectedVersion::NoStream => {
                if current_version != 0 {
                    return Err(pos_core::PosError::Conflict(
                        "stream is not empty".to_string(),
                    ));
                }
            }
            ExpectedVersion::Exact(version) => {
                if current_version != version {
                    return Err(pos_core::PosError::Conflict(format!(
                        "expected version {version}, got {current_version}"
                    )));
                }
            }
        }

        if events.is_empty() {
            return Ok(current_version);
        }

        let key = AggregateKey::from(aggregate);
        let stream = self.streams.entry(key).or_default();
        let mut next_version = current_version;
        for env in events.iter_mut() {
            if env.aggregate != aggregate {
                return Err(pos_core::PosError::Validation(
                    "event aggregate does not match stream".to_string(),
                ));
            }
            next_version += 1;
            env.seq = Some(next_version);
            stream.push(env.clone());
        }

        Ok(next_version)
    }

    fn get_idempotency(
        &self,
        aggregate: AggregateRef,
        key: &pos_core::IdempotencyKey,
    ) -> Result<Option<Vec<EventEnvelope>>, pos_core::PosError> {
        Ok(self
            .idempotency
            .get(&(AggregateKey::from(aggregate), key.0.clone()))
            .cloned())
    }

    fn put_idempotency(
        &mut self,
        aggregate: AggregateRef,
        key: pos_core::IdempotencyKey,
        events: Vec<EventEnvelope>,
    ) -> Result<(), pos_core::PosError> {
        self.idempotency
            .insert((AggregateKey::from(aggregate), key.0), events);
        Ok(())
    }
}

fn load_order(store: &InMemoryEventStore, order_id: OrderID) -> Result<Order, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::order(order_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Order(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-order event in order stream".to_string(),
                ))
            }
        }
    }
    rebuild_order_from_events(&events)
}

fn load_check(store: &InMemoryEventStore, check_id: CheckID) -> Result<Check, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::check(check_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Check(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-check event in check stream".to_string(),
                ))
            }
        }
    }
    rebuild_check_from_events(&events)
}

fn load_table(store: &InMemoryEventStore, table_id: TableID) -> Result<Table, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::table(table_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Table(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-table event in table stream".to_string(),
                ))
            }
        }
    }
    rebuild_table_from_events(&events)
}

fn route_and_store(
    store: &mut InMemoryEventStore,
    ctx: CoreContext<'_>,
    env: CommandEnvelope,
) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
    let routed = route_command_envelope_enveloped(ctx, env)?;
    store.append_envelopes_in_order(routed)
}

fn handle_command(
    store: &mut InMemoryEventStore,
    ctx: CoreContext<'_>,
    env: CommandEnvelope,
    aggregate_ref: AggregateRef,
) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
    let idempotency_key = env.idempotency_key.clone();
    if let Some(key) = idempotency_key.as_ref() {
        if let Some(saved) = store.get_idempotency(aggregate_ref, key)? {
            return Ok(saved);
        }
    }

    let events = route_and_store(store, ctx, env)?;

    if let Some(key) = idempotency_key {
        store.put_idempotency(aggregate_ref, key, events.clone())?;
    }

    Ok(events)
}
```

Notes:
- For real storage, enforce `ExpectedVersion` and multi-aggregate transactions.
- The in-memory store above assigns `seq` values and supports idempotency.
- See `crates/pos-core/examples/golden_flow.rs` for a full end-to-end flow.
- Typical loop: receive envelope -> load aggregates -> choose primary aggregate
  for idempotency -> `handle_command` -> publish events -> update projections.

## Integrating from TypeScript / Swift / Kotlin

The cross-platform contract is JSON. Clients send `CommandEnvelope` JSON to the
host, and receive `EventEnvelope` JSON back. Enums are tagged:
`{"kind":"Variant","data":{...}}` (see `docs/schema.md`).

Example `CommandEnvelope`:

```json
{
  "schema_version": 2,
  "command_id": "6b38f7a5-2a1b-45f7-ae45-1b7f6a9b1c07",
  "idempotency_key": "client-req-123",
  "correlation_id": "a2e2b9d0-0a02-4b06-9c0a-ef3a1f0a8d8c",
  "causation_id": null,
  "actor_id": "9f41c2d8-21b7-4f43-a4d0-7a32d5a1ed2d",
  "command": {
    "kind": "Order",
    "data": {
      "kind": "AddItem",
      "data": {
        "venue_id": "f9e12e63-9f4a-4b6c-9b71-3e9968e8c79f",
        "order_id": "17a8c1a5-4c3a-4a0c-8f8b-0b5c3b5e6a11",
        "order_item_id": "0d8d9674-01a3-4a57-bd86-5a2a7d2e0e3c",
        "menu_item_id": "c90c3d1e-7d7d-4b1b-9a0f-7d0e1b5f6a2f",
        "name": "Burger",
        "unit_price": {"amount": "12.99", "currency": "USD"},
        "qty": 1,
        "notes": "No onions"
      }
    }
  }
}
```

Example `EventEnvelope`:

```json
{
  "schema_version": 2,
  "event_id": "b0b9b1a3-75d2-4de4-a2cf-2d1e0d0b4b1e",
  "aggregate": {
    "ty": { "kind": "Order" },
    "id": "17a8c1a5-4c3a-4a0c-8f8b-0b5c3b5e6a11"
  },
  "seq": 5,
  "actor_id": "9f41c2d8-21b7-4f43-a4d0-7a32d5a1ed2d",
  "causation_id": "6b38f7a5-2a1b-45f7-ae45-1b7f6a9b1c07",
  "correlation_id": "a2e2b9d0-0a02-4b06-9c0a-ef3a1f0a8d8c",
  "event": {
    "kind": "Order",
    "data": {
      "kind": "ItemAdded",
      "data": {
        "venue_id": "f9e12e63-9f4a-4b6c-9b71-3e9968e8c79f",
        "order_id": "17a8c1a5-4c3a-4a0c-8f8b-0b5c3b5e6a11",
        "order_item_id": "0d8d9674-01a3-4a57-bd86-5a2a7d2e0e3c",
        "menu_item_id": "c90c3d1e-7d7d-4b1b-9a0f-7d0e1b5f6a2f",
        "name": "Burger",
        "unit_price": {"amount": "12.99", "currency": "USD"},
        "qty": 1,
        "notes": "No onions"
      }
    }
  }
}
```

Integration checklist:
- Build envelopes exactly (same field names, UUIDs as strings).
- Store `EventEnvelope` JSON as-is in an append-only stream keyed by
  `{aggregate.ty, aggregate.id}` and assign `seq` as the per-aggregate version.
- Use a stable aggregate for idempotency keys (order/check/table by command
  type, and a primary aggregate for workflows like `table_id` or `from_check_id`).
- Use `(aggregate_ref, idempotency_key)` to dedupe retries (see
  `docs/storage_contract.md`).
- Rebuild projections by replaying each aggregate stream in `seq` order:
  `rebuild_order_from_events`, `rebuild_check_from_events`,
  `rebuild_table_from_events`, and for KDS `rebuild_kds_from_events`.

## KDS

The KDS subsystem projects order events into station-specific tickets. It
supports routing rules, per-line status, and delta printing with cursors. See
`crates/pos-core/src/kds` and `docs/overview.md`.

## Golden flow example

Run the end-to-end in-memory flow and print the resulting commands, events,
KDS tickets, and totals as JSON:

```bash
cargo run -p pos-core --example golden_flow
```

## Tests

Run all tests:

```bash
cargo test
```

Key flows live in `crates/pos-core/src/tests` (e.g. `mvp_flow.rs`).

## Support
If there are any issues, or questions please contact pos@halvex.net, or open a ticket within
the official Halvex discord server.
