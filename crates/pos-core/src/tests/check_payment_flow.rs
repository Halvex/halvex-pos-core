use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, TenderType};
use crate::engine::{handle_check_command, Apply};
use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, VenueID};

#[test]
fn can_record_payment_and_close_check() {
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

    // add line 1x 10.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id: OrderItemID::new(),
            qty: 1,
            unit_price: Money::gbp(Decimal::new(1000, 2)),
            name: "Coffee".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let t = totals(&check);
    assert_eq!(t.total.amount, Decimal::new(1000, 2));
    assert_eq!(t.balance_due.amount, Decimal::new(1000, 2));

    // pay 10.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::RecordPayment {
            venue_id,
            check_id,
            payment_id: PaymentID::new(),
            tender: TenderType::Card,
            amount: Money::gbp(Decimal::new(1000, 2)),
            tip: None,
            processor: None,
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let t = totals(&check);
    assert_eq!(t.paid.amount, Decimal::new(1000, 2));
    assert_eq!(t.balance_due.amount, Decimal::new(0, 2));

    // close
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::CloseCheck { venue_id, check_id },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }
}
