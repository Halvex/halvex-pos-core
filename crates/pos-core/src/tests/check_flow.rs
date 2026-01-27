use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand};
use crate::engine::{handle_check_command, Apply};
use pos_types::{CheckID, Money, OrderID, OrderItemID, VenueID};

#[test]
fn can_open_add_discount_service_and_total() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    // open
    let ev = handle_check_command(
        None,
        CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id,
        },
    )
    .unwrap();
    let mut check = Check::new(venue_id, check_id, order_id);
    for e in &ev {
        check.apply(e).unwrap();
    }

    // add line 2x 10.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id: OrderItemID::new(),
            qty: 2,
            unit_price: Money::gbp(Decimal::new(1000, 2)), // 10.00
            name: "Pasta".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    // 10% discount
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::ApplyDiscountPercent {
            venue_id,
            check_id,
            label: "Staff".into(),
            percent: 10,
            reason: "staff discount".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    // 12% service
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::ApplyServiceChargePercent {
            venue_id,
            check_id,
            label: "Service".into(),
            percent: 12,
            reason: "service charge".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let t = totals(&check);
    // subtotal = 20.00, discount = 2.00, after = 18.00, service = 2.16, total = 20.16
    assert_eq!(t.subtotal.amount, Decimal::new(2000, 2));
    assert_eq!(t.discount.amount, Decimal::new(200, 2));
    assert_eq!(t.service_charge.amount, Decimal::new(216, 2));
    assert_eq!(t.total.amount, Decimal::new(2016, 2));
}
