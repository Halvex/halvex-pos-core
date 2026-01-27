use crate::kds::{
    apply_order_event, kitchen_tickets, KdsLineStatus, KdsState, RoutingRule, RoutingTable,
};
use crate::orders::OrderEvent;
use crate::tables::Table;
use pos_types::{MenuItemID, OrderID, OrderItemID, VenueID};

#[test]
fn projects_items_into_station_tickets() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let item_grill = OrderItemID::new();
    let item_bar = OrderItemID::new();
    let menu_grill = MenuItemID::new();
    let menu_bar = MenuItemID::new();

    let routing = RoutingTable::new(
        vec![
            RoutingRule::for_menu_item(menu_grill, "grill"),
            RoutingRule::for_menu_item(menu_bar, "bar"),
        ],
        "kitchen",
    );

    let table = Table::new(venue_id, pos_types::TableID::new(), "T1", None);
    let mut state = KdsState::new();

    apply_order_event(
        &mut state,
        &OrderEvent::OrderOpened { venue_id, order_id },
        1,
        &routing,
        Some(&table),
    )
    .unwrap();

    apply_order_event(
        &mut state,
        &OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id: item_grill,
            menu_item_id: menu_grill,
            name: "Burger".into(),
            unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(1000, 2)),
            qty: 1,
            notes: None,
        },
        2,
        &routing,
        Some(&table),
    )
    .unwrap();

    apply_order_event(
        &mut state,
        &OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id: item_bar,
            menu_item_id: menu_bar,
            name: "Beer".into(),
            unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(450, 2)),
            qty: 2,
            notes: None,
        },
        3,
        &routing,
        Some(&table),
    )
    .unwrap();

    let mut tickets = kitchen_tickets(&state, None);
    tickets.sort_by(|a, b| a.station.cmp(&b.station));

    assert_eq!(tickets.len(), 2);
    assert_eq!(tickets[0].station, "bar");
    assert_eq!(tickets[0].lines.len(), 1);
    assert_eq!(tickets[0].lines[0].name, "Beer");
    assert_eq!(tickets[0].lines[0].qty, 2);
    assert_eq!(tickets[0].lines[0].status, KdsLineStatus::New);

    assert_eq!(tickets[1].station, "grill");
    assert_eq!(tickets[1].lines.len(), 1);
    assert_eq!(tickets[1].lines[0].name, "Burger");
    assert_eq!(tickets[1].lines[0].qty, 1);
    assert_eq!(tickets[1].table_label.as_deref(), Some("T1"));
}

#[test]
fn projection_is_deterministic() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let routing = RoutingTable::default();

    let events = vec![
        (1, OrderEvent::OrderOpened { venue_id, order_id }),
        (
            2,
            OrderEvent::ItemAdded {
                venue_id,
                order_id,
                order_item_id,
                menu_item_id,
                name: "Fries".into(),
                unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(350, 2)),
                qty: 1,
                notes: None,
            },
        ),
    ];

    let mut state_a = KdsState::new();
    let mut state_b = KdsState::new();

    for (seq, event) in events.clone() {
        apply_order_event(&mut state_a, &event, seq, &routing, None).unwrap();
    }
    for (seq, event) in events {
        apply_order_event(&mut state_b, &event, seq, &routing, None).unwrap();
    }

    let tickets_a = kitchen_tickets(&state_a, None);
    let tickets_b = kitchen_tickets(&state_b, None);

    assert_eq!(tickets_a, tickets_b);
}
