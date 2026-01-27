use serde::{Deserialize, Serialize};

use pos_types::PosError;

use crate::engine::Apply;
use crate::kds::{apply_core_event_to_kds, KdsState, RoutingTable};
use crate::{Check, CheckEvent, Order, OrderEvent, Table, TableEvent};

use super::envelope::{AggregateRef, CoreEvent, EventEnvelope};

/// Snapshot of an aggregate at a specific per-aggregate version.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot<T> {
    pub aggregate: AggregateRef,
    pub version: u64,
    pub state: T,
}

impl<T> Snapshot<T> {
    pub fn new(aggregate: AggregateRef, version: u64, state: T) -> Self {
        Self {
            aggregate,
            version,
            state,
        }
    }
}

pub fn snapshot_order(order: &Order) -> Snapshot<Order> {
    Snapshot::new(
        AggregateRef::order(order.order_id),
        order.version,
        order.clone(),
    )
}

pub fn snapshot_check(check: &Check) -> Snapshot<Check> {
    Snapshot::new(
        AggregateRef::check(check.check_id),
        check.version,
        check.clone(),
    )
}

pub fn snapshot_table(table: &Table) -> Snapshot<Table> {
    Snapshot::new(
        AggregateRef::table(table.table_id),
        table.version,
        table.clone(),
    )
}

pub fn rebuild_order_from_events(events: &[OrderEvent]) -> Result<Order, PosError> {
    let first = events
        .first()
        .ok_or_else(|| PosError::NotFound("order events empty".into()))?;

    let (venue_id, order_id) = match first {
        OrderEvent::OrderOpened { venue_id, order_id }
        | OrderEvent::ItemAdded {
            venue_id, order_id, ..
        }
        | OrderEvent::ItemRemoved {
            venue_id, order_id, ..
        }
        | OrderEvent::OrderClosed { venue_id, order_id } => (*venue_id, *order_id),
    };

    let mut order = Order::new(venue_id, order_id);
    for event in events {
        order.apply(event)?;
    }

    Ok(order)
}

pub fn rebuild_check_from_events(events: &[CheckEvent]) -> Result<Check, PosError> {
    let first = events
        .first()
        .ok_or_else(|| PosError::NotFound("check events empty".into()))?;

    let (venue_id, check_id, order_id) = match first {
        CheckEvent::CheckOpened {
            venue_id,
            check_id,
            order_id,
        } => (*venue_id, *check_id, *order_id),
        _ => {
            return Err(PosError::validation(
                "check rebuild requires first event to be CheckOpened",
            ))
        }
    };

    let mut check = Check::new(venue_id, check_id, order_id);
    for event in events {
        check.apply(event)?;
    }

    Ok(check)
}

pub fn rebuild_table_from_events(events: &[TableEvent]) -> Result<Table, PosError> {
    let first = events
        .first()
        .ok_or_else(|| PosError::NotFound("table events empty".into()))?;

    let (venue_id, table_id, label, area_id) = match first {
        TableEvent::TableCreated {
            venue_id,
            table_id,
            label,
            area_id,
        } => (*venue_id, *table_id, label.clone(), *area_id),
        _ => {
            return Err(PosError::validation(
                "table rebuild requires first event to be TableCreated",
            ))
        }
    };

    let mut table = Table::new(venue_id, table_id, label, area_id);
    for event in events {
        table.apply(event)?;
    }

    Ok(table)
}

pub fn rebuild_kds_from_events(
    events: &[EventEnvelope],
    routing: &RoutingTable,
    table: Option<&Table>,
) -> Result<KdsState, PosError> {
    let mut state = KdsState::new();
    for env in events {
        apply_core_event_to_kds(&mut state, env, routing, table)?;
    }

    Ok(state)
}

pub fn rebuild_order_from_snapshot_and_events(
    snapshot: Snapshot<Order>,
    events: &[EventEnvelope],
) -> Result<Order, PosError> {
    if snapshot.aggregate.ty != super::envelope::AggregateType::Order {
        return Err(PosError::validation("snapshot aggregate is not Order"));
    }

    let mut order = snapshot.state;
    for env in events {
        if env.aggregate != snapshot.aggregate {
            continue;
        }
        let seq = env
            .seq
            .ok_or_else(|| PosError::validation("event seq missing"))?;
        if seq <= snapshot.version {
            continue;
        }

        match &env.event {
            CoreEvent::Order(event) => order.apply(event)?,
            _ => return Err(PosError::validation("non-order event in order stream")),
        }
    }

    Ok(order)
}

pub fn rebuild_check_from_snapshot_and_events(
    snapshot: Snapshot<Check>,
    events: &[EventEnvelope],
) -> Result<Check, PosError> {
    if snapshot.aggregate.ty != super::envelope::AggregateType::Check {
        return Err(PosError::validation("snapshot aggregate is not Check"));
    }

    let mut check = snapshot.state;
    for env in events {
        if env.aggregate != snapshot.aggregate {
            continue;
        }
        let seq = env
            .seq
            .ok_or_else(|| PosError::validation("event seq missing"))?;
        if seq <= snapshot.version {
            continue;
        }

        match &env.event {
            CoreEvent::Check(event) => check.apply(event)?,
            _ => return Err(PosError::validation("non-check event in check stream")),
        }
    }

    Ok(check)
}

pub fn rebuild_table_from_snapshot_and_events(
    snapshot: Snapshot<Table>,
    events: &[EventEnvelope],
) -> Result<Table, PosError> {
    if snapshot.aggregate.ty != super::envelope::AggregateType::Table {
        return Err(PosError::validation("snapshot aggregate is not Table"));
    }

    let mut table = snapshot.state;
    for env in events {
        if env.aggregate != snapshot.aggregate {
            continue;
        }
        let seq = env
            .seq
            .ok_or_else(|| PosError::validation("event seq missing"))?;
        if seq <= snapshot.version {
            continue;
        }

        match &env.event {
            CoreEvent::Table(event) => table.apply(event)?,
            _ => return Err(PosError::validation("non-table event in table stream")),
        }
    }

    Ok(table)
}
