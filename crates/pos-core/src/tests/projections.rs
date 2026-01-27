use rust_decimal::Decimal;

use crate::checks::{Check, CheckCommand};
use crate::engine::{handle_check_command, Apply};
use crate::projections::{can_close_check, check_balance_due, table_is_available};
use crate::tables::{Table, TableEvent};
use pos_types::{CheckID, Money, OrderID, OrderItemID, TableID, VenueID};

#[test]
fn table_is_available_tracks_assignment() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();

    let mut table = Table::new(venue_id, table_id, "T1", None);
    assert!(table_is_available(&table));

    table
        .apply(&TableEvent::OrderAssignedToTable {
            venue_id,
            table_id,
            order_id,
        })
        .unwrap();

    assert!(!table_is_available(&table));
}

#[test]
fn check_balance_due_and_can_close_check() {
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

    let due = check_balance_due(&check);
    assert_eq!(due.amount, Decimal::new(1000, 2));
    assert!(!can_close_check(&check));

    let ev = handle_check_command(
        Some(&check),
        CheckCommand::RecordPayment {
            venue_id,
            check_id,
            payment_id: pos_types::PaymentID::new(),
            tender: crate::checks::TenderType::Card,
            amount: Money::gbp(Decimal::new(1000, 2)),
            tip: None,
            processor: None,
        },
    )
    .unwrap();
    for e in &ev {
        check.apply(e).unwrap();
    }

    let due = check_balance_due(&check);
    assert_eq!(due.amount, Decimal::new(0, 2));
    assert!(can_close_check(&check));
}
