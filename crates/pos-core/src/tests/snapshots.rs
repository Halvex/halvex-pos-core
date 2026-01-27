use rust_decimal::Decimal;

use crate::checks::CheckEvent;
use crate::core::{
    rebuild_check_from_events, rebuild_check_from_snapshot_and_events, rebuild_kds_from_events,
    rebuild_order_from_events, rebuild_order_from_snapshot_and_events, rebuild_table_from_events,
    rebuild_table_from_snapshot_and_events, snapshot_check, snapshot_order, snapshot_table,
};
use crate::engine::Apply;
use crate::kds::{kitchen_tickets, RoutingTable};
use crate::orders::OrderEvent;
use crate::tables::TableEvent;
use crate::{AggregateRef, Check, CoreEvent, EventEnvelope, Order, Table};
use pos_types::{AreaID, CheckID, MenuItemID, Money, OrderID, OrderItemID, TableID, VenueID};

fn assert_json_eq<T: serde::Serialize>(left: &T, right: &T) {
    let left_json = serde_json::to_value(left).unwrap();
    let right_json = serde_json::to_value(right).unwrap();
    assert_eq!(left_json, right_json);
}

fn envelope_with_seq(aggregate: AggregateRef, event: CoreEvent, seq: u64) -> EventEnvelope {
    let mut env = EventEnvelope::new(aggregate, event);
    env.seq = Some(seq);
    env
}

#[test]
fn rebuild_order_matches_apply_and_snapshot_tail() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let events = vec![
        OrderEvent::OrderOpened { venue_id, order_id },
        OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name: "Burger".into(),
            unit_price: Money::gbp(Decimal::new(1200, 2)),
            qty: 1,
            notes: None,
        },
    ];

    let mut direct = Order::new(venue_id, order_id);
    for event in &events {
        direct.apply(event).unwrap();
    }

    let rebuilt = rebuild_order_from_events(&events).unwrap();
    assert_json_eq(&direct, &rebuilt);

    let snapshot = snapshot_order(&rebuilt);
    let envs = vec![
        envelope_with_seq(
            AggregateRef::order(order_id),
            CoreEvent::Order(events[0].clone()),
            1,
        ),
        envelope_with_seq(
            AggregateRef::order(order_id),
            CoreEvent::Order(events[1].clone()),
            2,
        ),
    ];

    let rebuilt_tail = rebuild_order_from_snapshot_and_events(snapshot, &envs).unwrap();
    assert_json_eq(&direct, &rebuilt_tail);
}

#[test]
fn rebuild_check_matches_apply_and_snapshot_tail() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();
    let order_item_id = OrderItemID::new();

    let events = vec![
        CheckEvent::CheckOpened {
            venue_id,
            check_id,
            order_id,
        },
        CheckEvent::LineAdded {
            venue_id,
            check_id,
            order_item_id,
            qty: 2,
            unit_price: Money::gbp(Decimal::new(450, 2)),
            name: "Latte".into(),
        },
    ];

    let mut direct = Check::new(venue_id, check_id, order_id);
    for event in &events {
        direct.apply(event).unwrap();
    }

    let rebuilt = rebuild_check_from_events(&events).unwrap();
    assert_json_eq(&direct, &rebuilt);

    let snapshot = snapshot_check(&rebuilt);
    let envs = vec![
        envelope_with_seq(
            AggregateRef::check(check_id),
            CoreEvent::Check(events[0].clone()),
            1,
        ),
        envelope_with_seq(
            AggregateRef::check(check_id),
            CoreEvent::Check(events[1].clone()),
            2,
        ),
    ];

    let rebuilt_tail = rebuild_check_from_snapshot_and_events(snapshot, &envs).unwrap();
    assert_json_eq(&direct, &rebuilt_tail);
}

#[test]
fn rebuild_table_matches_apply_and_snapshot_tail() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();
    let area_id = AreaID::new();

    let events = vec![
        TableEvent::TableCreated {
            venue_id,
            table_id,
            label: "A1".into(),
            area_id: Some(area_id),
        },
        TableEvent::OrderAssignedToTable {
            venue_id,
            table_id,
            order_id,
        },
    ];

    let mut direct = Table::new(venue_id, table_id, "A1", Some(area_id));
    for event in &events {
        direct.apply(event).unwrap();
    }

    let rebuilt = rebuild_table_from_events(&events).unwrap();
    assert_json_eq(&direct, &rebuilt);

    let snapshot = snapshot_table(&rebuilt);
    let envs = vec![
        envelope_with_seq(
            AggregateRef::table(table_id),
            CoreEvent::Table(events[0].clone()),
            1,
        ),
        envelope_with_seq(
            AggregateRef::table(table_id),
            CoreEvent::Table(events[1].clone()),
            2,
        ),
    ];

    let rebuilt_tail = rebuild_table_from_snapshot_and_events(snapshot, &envs).unwrap();
    assert_json_eq(&direct, &rebuilt_tail);
}

#[test]
fn rebuild_kds_matches_apply() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let event = CoreEvent::Order(OrderEvent::ItemAdded {
        venue_id,
        order_id,
        order_item_id,
        menu_item_id,
        name: "Soup".into(),
        unit_price: Money::gbp(Decimal::new(350, 2)),
        qty: 1,
        notes: None,
    });

    let env = envelope_with_seq(AggregateRef::order(order_id), event, 1);
    let routing = RoutingTable::default();

    let direct = {
        let mut state = crate::kds::KdsState::new();
        crate::kds::apply_core_event_to_kds(&mut state, &env, &routing, None).unwrap();
        state
    };

    let rebuilt = rebuild_kds_from_events(&[env], &routing, None).unwrap();

    assert_eq!(direct.cursor, rebuilt.cursor);
    assert_eq!(direct.changes, rebuilt.changes);
    assert_eq!(
        kitchen_tickets(&direct, None),
        kitchen_tickets(&rebuilt, None)
    );
    assert_eq!(direct.item_to_ticket.len(), rebuilt.item_to_ticket.len());
    assert_eq!(direct.line_to_ticket.len(), rebuilt.line_to_ticket.len());

    for (key, value) in &direct.item_to_ticket {
        assert_eq!(rebuilt.item_to_ticket.get(key), Some(value));
    }

    for (key, value) in &direct.line_to_ticket {
        assert_eq!(rebuilt.line_to_ticket.get(key), Some(value));
    }
}
