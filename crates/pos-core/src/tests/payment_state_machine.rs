use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, PaymentStatus, TenderType};
use crate::engine::{handle_check_command, Apply};
use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, VenueID};

#[test]
fn payment_authorise_capture_refund_and_limits() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    let mut check = Check::new(venue_id, check_id, order_id);

    // open check
    let ev = handle_check_command(
        None,
        CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id,
        },
    )
    .unwrap();
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

    let payment_id = PaymentID::new();

    // authorise 10.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::AuthorisePayment {
            venue_id,
            check_id,
            payment_id,
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
    assert_eq!(t.paid.amount, Decimal::new(0, 2));
    assert_eq!(t.balance_due.amount, Decimal::new(1000, 2));

    // capture beyond remaining should fail
    let err = handle_check_command(
        Some(&check),
        CheckCommand::CapturePayment {
            venue_id,
            check_id,
            payment_id,
            amount: Money::gbp(Decimal::new(1200, 2)),
        },
    )
    .unwrap_err();
    assert!(matches!(err, pos_types::PosError::Validation(_)));

    // partial capture 6.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::CapturePayment {
            venue_id,
            check_id,
            payment_id,
            amount: Money::gbp(Decimal::new(600, 2)),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let t = totals(&check);
    assert_eq!(t.paid.amount, Decimal::new(600, 2));
    assert_eq!(t.balance_due.amount, Decimal::new(400, 2));

    // refund 2.00
    let ev = handle_check_command(
        Some(&check),
        CheckCommand::RecordRefund {
            venue_id,
            check_id,
            payment_id,
            amount: Money::gbp(Decimal::new(200, 2)),
            reason: "guest complaint".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let t = totals(&check);
    assert_eq!(t.paid.amount, Decimal::new(400, 2));
    assert_eq!(t.balance_due.amount, Decimal::new(600, 2));

    // refund beyond captured should fail
    let err = handle_check_command(
        Some(&check),
        CheckCommand::RecordRefund {
            venue_id,
            check_id,
            payment_id,
            amount: Money::gbp(Decimal::new(500, 2)),
            reason: "too much".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, pos_types::PosError::Validation(_)));

    // void after capture should fail
    let err = handle_check_command(
        Some(&check),
        CheckCommand::VoidPayment {
            venue_id,
            check_id,
            payment_id,
            reason: "too late".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, pos_types::PosError::Validation(_)));
}

#[test]
fn can_void_authorised_payment_before_capture() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    let mut check = Check::new(venue_id, check_id, order_id);

    let ev = handle_check_command(
        None,
        CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id,
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let payment_id = PaymentID::new();

    let ev = handle_check_command(
        Some(&check),
        CheckCommand::AuthorisePayment {
            venue_id,
            check_id,
            payment_id,
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

    let ev = handle_check_command(
        Some(&check),
        CheckCommand::VoidPayment {
            venue_id,
            check_id,
            payment_id,
            reason: "guest canceled".into(),
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let payment = check
        .payments
        .iter()
        .find(|p| p.payment_id == payment_id)
        .unwrap();
    assert_eq!(payment.status, PaymentStatus::Voided);
}
