use std::collections::HashMap;

use pos_core::prelude::*;
use pos_types::{CheckID, OrderID, VenueID};

#[derive(Clone, Default)]
struct InMemoryEventStore {
    streams: HashMap<(u8, uuid::Uuid), Vec<EventEnvelope>>,
    idempotency: HashMap<(u8, uuid::Uuid, String), Vec<EventEnvelope>>,
}

impl InMemoryEventStore {
    fn new() -> Self {
        Self::default()
    }

    fn agg_key(aggregate: AggregateRef) -> (u8, uuid::Uuid) {
        (aggregate.ty as u8, aggregate.id)
    }
}

impl EventStore for InMemoryEventStore {
    fn load_stream(&self, aggregate: AggregateRef) -> Result<Vec<EventEnvelope>, PosError> {
        let key = Self::agg_key(aggregate);
        Ok(self.streams.get(&key).cloned().unwrap_or_default())
    }

    fn append_to_stream(
        &mut self,
        aggregate: AggregateRef,
        expected: ExpectedVersion,
        events: Vec<EventEnvelope>,
    ) -> Result<u64, PosError> {
        let key = Self::agg_key(aggregate);
        let stream = self.streams.entry(key).or_default();
        let current_version = stream.last().and_then(|e| e.seq).unwrap_or(0);

        match expected {
            ExpectedVersion::Any => {}
            ExpectedVersion::NoStream => {
                if current_version != 0 {
                    return Err(PosError::Conflict("expected empty stream".into()));
                }
            }
            ExpectedVersion::Exact(v) => {
                if current_version != v {
                    return Err(PosError::Conflict("expected version mismatch".into()));
                }
            }
        }

        if events.is_empty() {
            return Ok(current_version);
        }

        let mut next_version = current_version;
        for mut event in events {
            if event.aggregate != aggregate {
                return Err(PosError::validation("event aggregate mismatch"));
            }
            next_version += 1;
            event.seq = Some(next_version);
            stream.push(event);
        }

        Ok(next_version)
    }

    fn get_idempotency(
        &self,
        aggregate: AggregateRef,
        key: &IdempotencyKey,
    ) -> Result<Option<Vec<EventEnvelope>>, PosError> {
        let agg_key = Self::agg_key(aggregate);
        let lookup = (agg_key.0, agg_key.1, key.0.clone());
        Ok(self.idempotency.get(&lookup).cloned())
    }

    fn put_idempotency(
        &mut self,
        aggregate: AggregateRef,
        key: IdempotencyKey,
        events: Vec<EventEnvelope>,
    ) -> Result<(), PosError> {
        let agg_key = Self::agg_key(aggregate);
        let lookup = (agg_key.0, agg_key.1, key.0);
        self.idempotency.insert(lookup, events);
        Ok(())
    }
}

fn append_batch_atomic(
    store: &mut InMemoryEventStore,
    batches: Vec<(AggregateRef, ExpectedVersion, Vec<EventEnvelope>)>,
) -> Result<(), PosError> {
    let mut shadow = store.clone();
    for (aggregate, expected, events) in batches {
        shadow.append_to_stream(aggregate, expected, events)?;
    }
    *store = shadow;
    Ok(())
}

fn order_open_event(venue_id: VenueID, order_id: OrderID) -> EventEnvelope {
    let event = CoreEvent::Order(OrderEvent::OrderOpened { venue_id, order_id });
    EventEnvelope::new(AggregateRef::order(order_id), event)
}

fn check_open_event(venue_id: VenueID, check_id: CheckID, order_id: OrderID) -> EventEnvelope {
    let event = CoreEvent::Check(CheckEvent::CheckOpened {
        venue_id,
        check_id,
        order_id,
    });
    EventEnvelope::new(AggregateRef::check(check_id), event)
}

#[test]
fn assigns_seq_and_replays() {
    let mut store = InMemoryEventStore::new();
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    let aggregate = AggregateRef::order(order_id);
    let events = vec![
        order_open_event(venue_id, order_id),
        order_open_event(venue_id, order_id),
    ];

    let new_version = store
        .append_to_stream(aggregate, ExpectedVersion::NoStream, events)
        .unwrap();
    assert_eq!(new_version, 2);

    let replayed = store.load_stream(aggregate).unwrap();
    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].seq, Some(1));
    assert_eq!(replayed[1].seq, Some(2));
}

#[test]
fn idempotency_get_put_roundtrip() {
    let mut store = InMemoryEventStore::new();
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    let aggregate = AggregateRef::order(order_id);
    let key = IdempotencyKey::new("retry-key");

    assert!(store.get_idempotency(aggregate, &key).unwrap().is_none());

    let events = vec![order_open_event(venue_id, order_id)];
    store
        .put_idempotency(aggregate, key.clone(), events.clone())
        .unwrap();

    let stored = store.get_idempotency(aggregate, &key).unwrap().unwrap();
    assert_eq!(stored.len(), events.len());
}

#[test]
fn atomic_batch_commit_or_rollback() {
    let mut store = InMemoryEventStore::new();
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    let order_agg = AggregateRef::order(order_id);
    let check_agg = AggregateRef::check(check_id);

    let order_events = vec![order_open_event(venue_id, order_id)];
    let check_events = vec![check_open_event(venue_id, check_id, order_id)];

    let err = append_batch_atomic(
        &mut store,
        vec![
            (order_agg, ExpectedVersion::NoStream, order_events.clone()),
            (check_agg, ExpectedVersion::Exact(1), check_events.clone()),
        ],
    )
    .unwrap_err();
    match err {
        PosError::Conflict(_) => {}
        _ => panic!("expected conflict on version mismatch"),
    }

    assert!(store.load_stream(order_agg).unwrap().is_empty());
    assert!(store.load_stream(check_agg).unwrap().is_empty());

    append_batch_atomic(
        &mut store,
        vec![
            (order_agg, ExpectedVersion::NoStream, order_events),
            (check_agg, ExpectedVersion::NoStream, check_events),
        ],
    )
    .unwrap();

    assert_eq!(store.load_stream(order_agg).unwrap().len(), 1);
    assert_eq!(store.load_stream(check_agg).unwrap().len(), 1);
}
