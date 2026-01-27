use crate::engine::{handle_order_command, Apply};
use crate::orders::{Order, OrderCommand};
use pos_types::{MenuItemID, Money, OrderID, OrderItemID, VenueID};
use rust_decimal::Decimal;

#[test]
fn can_open_add_and_close_order() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    // open
    let open_events =
        handle_order_command(None, OrderCommand::OpenOrder { venue_id, order_id }).unwrap();
    let mut order = Order::new(venue_id, order_id);
    for e in &open_events {
        order.apply(e).unwrap();
    }

    // add item
    let add_events = handle_order_command(
        Some(&order),
        OrderCommand::AddItem {
            venue_id,
            order_id,
            order_item_id: OrderItemID::new(),
            menu_item_id: MenuItemID::new(),
            name: "Burger".to_string(),
            unit_price: Money::gbp(Decimal::new(1299, 2)), // 12.99 GBP
            qty: 1,
            notes: Some("No onions".into()),
        },
    )
    .unwrap();

    for e in &add_events {
        order.apply(e).unwrap();
    }
    assert_eq!(order.items.len(), 1);

    // close
    let close_events = handle_order_command(
        Some(&order),
        OrderCommand::CloseOrder { venue_id, order_id },
    )
    .unwrap();
    for e in &close_events {
        order.apply(e).unwrap();
    }
}
