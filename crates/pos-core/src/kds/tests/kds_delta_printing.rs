use crate::kds::{
    apply_order_event, delta_tickets_since, kitchen_tickets, KdsDeltaKind, KdsLineStatus, KdsState,
    RoutingTable,
};
use crate::orders::OrderEvent;
use pos_types::{MenuItemID, OrderID, OrderItemID, VenueID};

#[test]
fn delta_printing_tracks_adds_qty_changes_and_voids() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let routing = RoutingTable::default();
    let mut state = KdsState::new();

    apply_order_event(
        &mut state,
        &OrderEvent::OrderOpened { venue_id, order_id },
        1,
        &routing,
        None,
    )
    .unwrap();

    apply_order_event(
        &mut state,
        &OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name: "Pizza".into(),
            unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(1200, 2)),
            qty: 2,
            notes: None,
        },
        2,
        &routing,
        None,
    )
    .unwrap();

    let delta = delta_tickets_since(&state, "kitchen", 0);
    assert_eq!(delta.len(), 1);
    assert_eq!(delta[0].lines.len(), 1);
    assert!(matches!(delta[0].lines[0].kind, KdsDeltaKind::Added));

    apply_order_event(
        &mut state,
        &OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name: "Pizza".into(),
            unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(1200, 2)),
            qty: 1,
            notes: None,
        },
        3,
        &routing,
        None,
    )
    .unwrap();

    apply_order_event(
        &mut state,
        &OrderEvent::ItemRemoved {
            venue_id,
            order_id,
            order_item_id,
            reason: Some("cancelled".into()),
        },
        4,
        &routing,
        None,
    )
    .unwrap();

    let delta = delta_tickets_since(&state, "kitchen", 2);
    assert_eq!(delta.len(), 1);
    assert_eq!(delta[0].lines.len(), 2);
    assert!(matches!(
        delta[0].lines[0].kind,
        KdsDeltaKind::QtyChanged { .. }
    ));
    assert!(matches!(delta[0].lines[1].kind, KdsDeltaKind::Voided));

    let tickets = kitchen_tickets(&state, Some("kitchen"));
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].lines.len(), 1);
    assert_eq!(tickets[0].lines[0].status, KdsLineStatus::Voided);
}
