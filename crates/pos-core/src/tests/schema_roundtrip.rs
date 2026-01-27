use rust_decimal::Decimal;

use crate::checks::CheckCommand;
use crate::core::{CommandEnvelope, CoreCommand, CoreEvent, EventEnvelope, SCHEMA_VERSION};
use crate::orders::OrderCommand;
use crate::tables::TableCommand;
use pos_types::{CheckID, Money, OrderID, TableID, VenueID};

#[test]
fn schema_roundtrip_command_envelope() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    let env = CommandEnvelope::new(CoreCommand::Order(OrderCommand::OpenOrder {
        venue_id,
        order_id,
    }));

    let json = serde_json::to_string(&env).unwrap();
    let de: CommandEnvelope = serde_json::from_str(&json).unwrap();

    assert_eq!(de.schema_version, SCHEMA_VERSION);
    match de.command {
        CoreCommand::Order(_) => {}
        _ => panic!("expected Order command after round-trip"),
    }
}

#[test]
fn schema_roundtrip_event_envelope() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    let event = CoreEvent::Order(crate::orders::OrderEvent::OrderOpened { venue_id, order_id });
    let agg = event.aggregate_ref();

    let env = EventEnvelope::new(agg, event);

    let json = serde_json::to_string(&env).unwrap();
    let de: EventEnvelope = serde_json::from_str(&json).unwrap();

    assert_eq!(de.schema_version, SCHEMA_VERSION);
    assert_eq!(de.aggregate.ty, env.aggregate.ty);
}

#[test]
fn schema_version_defaults_when_missing() {
    let venue_id = VenueID::new();
    let check_id = CheckID::new();

    let command = CoreCommand::Check(CheckCommand::CloseCheck { venue_id, check_id });
    let env = CommandEnvelope::new(command);

    let mut value = serde_json::to_value(&env).unwrap();
    value.as_object_mut().unwrap().remove("schema_version");

    let json = serde_json::to_string(&value).unwrap();
    let de: CommandEnvelope = serde_json::from_str(&json).unwrap();

    assert_eq!(de.schema_version, SCHEMA_VERSION);
}

#[test]
fn schema_roundtrip_domain_commands() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();
    let table_id = TableID::new();

    let order_cmd = OrderCommand::OpenOrder { venue_id, order_id };
    let check_cmd = CheckCommand::AddLineFromOrderItem {
        venue_id,
        check_id,
        order_item_id: pos_types::OrderItemID::new(),
        qty: 1,
        unit_price: Money::gbp(Decimal::new(450, 2)),
        name: "Latte".into(),
    };
    let table_cmd = TableCommand::CloseTable { venue_id, table_id };

    let order_json = serde_json::to_string(&order_cmd).unwrap();
    let check_json = serde_json::to_string(&check_cmd).unwrap();
    let table_json = serde_json::to_string(&table_cmd).unwrap();

    let _: OrderCommand = serde_json::from_str(&order_json).unwrap();
    let _: CheckCommand = serde_json::from_str(&check_json).unwrap();
    let _: TableCommand = serde_json::from_str(&table_json).unwrap();

    assert!(serde_json::from_str::<serde_json::Value>(&order_json).unwrap()["kind"].is_string());
    assert!(serde_json::from_str::<serde_json::Value>(&check_json).unwrap()["kind"].is_string());
    assert!(serde_json::from_str::<serde_json::Value>(&table_json).unwrap()["kind"].is_string());
}
