use pos_core::prelude::*;
use pos_types::{CheckID, MenuItemID, Money, OrderID, OrderItemID, TableID, VenueID};
use rust_decimal::Decimal;

#[test]
fn public_api_roundtrip() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();
    let table_id = TableID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let order_cmd = OrderCommand::AddItem {
        venue_id,
        order_id,
        order_item_id,
        menu_item_id,
        name: "Latte".to_string(),
        unit_price: Money::gbp(Decimal::new(450, 2)),
        qty: 1,
        notes: None,
    };

    let env = CommandEnvelope::new(CoreCommand::Order(order_cmd));
    let json = serde_json::to_string(&env).unwrap();
    let de: CommandEnvelope = serde_json::from_str(&json).unwrap();

    assert_eq!(de.schema_version, SCHEMA_VERSION);
    assert!(matches!(de.command, CoreCommand::Order(_)));

    let status_json = serde_json::to_value(OrderStatus::Open).unwrap();
    assert_eq!(status_json["kind"], "Open");

    let tender_json = serde_json::to_value(TenderType::Other {
        label: "custom".to_string(),
    })
    .unwrap();
    assert_eq!(tender_json["kind"], "Other");

    let mut check = Check::new(venue_id, check_id, order_id);
    check.lines.push(CheckLine {
        order_item_id,
        qty: 1,
        unit_price: Money::gbp(Decimal::new(450, 2)),
        name: "Latte".to_string(),
    });

    let totals = totals(&check);
    let totals_json = serde_json::to_string(&totals).unwrap();
    let _: CheckTotals = serde_json::from_str(&totals_json).unwrap();
    assert!(!can_close_check(&check));

    let table = Table::new(venue_id, table_id, "A1", None);
    assert!(table_is_available(&table));

    let mut state = KdsState::new();
    let routing = RoutingTable::default();
    let order_event = OrderEvent::ItemAdded {
        venue_id,
        order_id,
        order_item_id,
        menu_item_id,
        name: "Latte".to_string(),
        unit_price: Money::gbp(Decimal::new(450, 2)),
        qty: 1,
        notes: None,
    };

    let mut event_env =
        EventEnvelope::new(AggregateRef::order(order_id), CoreEvent::Order(order_event));
    event_env.seq = Some(1);

    apply_core_event_to_kds(&mut state, &event_env, &routing, None).unwrap();

    let tickets = kitchen_tickets(&state, None);
    assert_eq!(tickets.len(), 1);
    let ticket_json = serde_json::to_string(&tickets[0]).unwrap();
    let _: KdsTicket = serde_json::from_str(&ticket_json).unwrap();
}
