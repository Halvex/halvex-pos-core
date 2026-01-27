use insta::assert_json_snapshot;
use pos_core::prelude::*;
use pos_types::{ActorID, CheckID, Money, OrderID, OrderItemID, TableID, VenueID};
use rust_decimal::Decimal;
use uuid::Uuid;

fn uid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

#[test]
fn schema_snapshot_command_envelope() {
    let venue_id = VenueID(uid(1));
    let order_id = OrderID(uid(2));
    let command = OrderCommand::OpenOrder { venue_id, order_id };

    let mut env = CommandEnvelope::new(CoreCommand::Order(command));
    env.schema_version = SCHEMA_VERSION;
    env.command_id = uid(10);
    env.idempotency_key = Some(IdempotencyKey::new("retry-1"));
    env.correlation_id = Some(uid(11));
    env.causation_id = Some(uid(12));
    env.actor_id = Some(ActorID(uid(13)));

    assert_json_snapshot!("command_envelope", env);
}

#[test]
fn schema_snapshot_event_envelope() {
    let venue_id = VenueID(uid(20));
    let order_id = OrderID(uid(21));
    let event = CoreEvent::Order(OrderEvent::OrderOpened { venue_id, order_id });

    let mut env = EventEnvelope::new(AggregateRef::order(order_id), event);
    env.schema_version = SCHEMA_VERSION;
    env.event_id = uid(30);
    env.seq = Some(7);
    env.causation_id = Some(uid(31));
    env.correlation_id = Some(uid(32));
    env.actor_id = Some(ActorID(uid(33)));

    assert_json_snapshot!("event_envelope", env);
}

#[test]
fn schema_snapshot_domain_commands() {
    let venue_id = VenueID(uid(40));
    let order_id = OrderID(uid(41));
    let check_id = CheckID(uid(42));
    let table_id = TableID(uid(43));
    let order_item_id = OrderItemID(uid(44));

    let order_command = OrderCommand::AddItem {
        venue_id,
        order_id,
        order_item_id,
        menu_item_id: pos_types::MenuItemID(uid(45)),
        name: "Latte".to_string(),
        unit_price: Money::gbp(Decimal::new(450, 2)),
        qty: 1,
        notes: None,
    };

    let check_command = CheckCommand::AddLineFromOrderItem {
        venue_id,
        check_id,
        order_item_id,
        qty: 1,
        unit_price: Money::gbp(Decimal::new(450, 2)),
        name: "Latte".to_string(),
    };

    let table_command = TableCommand::AssignOrder {
        venue_id,
        table_id,
        order_id,
    };

    assert_json_snapshot!("order_command", order_command);
    assert_json_snapshot!("check_command", check_command);
    assert_json_snapshot!("table_command", table_command);
}
