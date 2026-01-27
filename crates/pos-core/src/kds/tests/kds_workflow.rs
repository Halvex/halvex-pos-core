use crate::kds::{
    apply_kds_event, apply_order_event, kitchen_tickets, KdsEvent, KdsLineStatus, KdsState,
    KdsTicketStatus, RoutingTable,
};
use crate::orders::OrderEvent;
use pos_types::{MenuItemID, OrderID, OrderItemID, VenueID};

#[test]
fn lifecycle_events_update_ticket_and_line_statuses() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();

    let routing = RoutingTable::default();
    let mut state = KdsState::new();

    apply_order_event(
        &mut state,
        &OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name: "Steak".into(),
            unit_price: pos_types::Money::gbp(rust_decimal::Decimal::new(2500, 2)),
            qty: 1,
            notes: None,
        },
        1,
        &routing,
        None,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))
        .into_iter()
        .next()
        .expect("ticket missing");
    let line_id = ticket.lines[0].line_id;

    apply_kds_event(
        &mut state,
        &KdsEvent::TicketAcknowledged {
            venue_id,
            station: "kitchen".into(),
            order_id,
        },
        2,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))[0].clone();
    assert_eq!(ticket.status, KdsTicketStatus::Acknowledged);

    apply_kds_event(
        &mut state,
        &KdsEvent::LineStarted {
            venue_id,
            station: "kitchen".into(),
            order_id,
            line_id,
        },
        3,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))[0].clone();
    assert_eq!(ticket.status, KdsTicketStatus::InProgress);
    assert_eq!(ticket.lines[0].status, KdsLineStatus::InProgress);

    apply_kds_event(
        &mut state,
        &KdsEvent::LineDone {
            venue_id,
            station: "kitchen".into(),
            order_id,
            line_id,
        },
        4,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))[0].clone();
    assert_eq!(ticket.status, KdsTicketStatus::Ready);
    assert_eq!(ticket.lines[0].status, KdsLineStatus::Done);

    apply_kds_event(
        &mut state,
        &KdsEvent::TicketBumped {
            venue_id,
            station: "kitchen".into(),
            order_id,
        },
        5,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))[0].clone();
    assert_eq!(ticket.status, KdsTicketStatus::Completed);

    apply_kds_event(
        &mut state,
        &KdsEvent::TicketRecalled {
            venue_id,
            station: "kitchen".into(),
            order_id,
        },
        6,
    )
    .unwrap();

    let ticket = kitchen_tickets(&state, Some("kitchen"))[0].clone();
    assert_eq!(ticket.status, KdsTicketStatus::InProgress);
}
