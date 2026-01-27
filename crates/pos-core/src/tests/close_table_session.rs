use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, CheckStatus, TenderType};
use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::{handle_check_command, handle_table_command, Apply};
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID};

#[test]
fn close_table_session_closes_paid_check_and_unassigns_table() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    // Create table
    let mut table = Table::new(venue_id, table_id, "T1", None);
    let evs = handle_table_command(
        None,
        TableCommand::CreateTable {
            venue_id,
            table_id,
            label: "T1".into(),
            area_id: None,
        },
    )
    .unwrap();
    for e in &evs {
        table.apply(e).unwrap();
    }

    // Assign order to table (simulate open session outcome)
    let evs = handle_table_command(
        Some(&table),
        TableCommand::AssignOrder {
            venue_id,
            table_id,
            order_id,
        },
    )
    .unwrap();
    for e in &evs {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Occupied);

    // Open check
    let mut check = Check::new(venue_id, check_id, order_id);
    let evs = handle_check_command(
        None,
        CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id,
        },
    )
    .unwrap();
    for e in &evs {
        check.apply(e).unwrap();
    }

    // Add line £10.00
    let item_id = OrderItemID::new();
    let evs = handle_check_command(
        Some(&check),
        CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id: item_id,
            qty: 1,
            unit_price: Money::gbp(Decimal::new(1000, 2)),
            name: "Coffee".into(),
        },
    )
    .unwrap();
    for e in &evs {
        check.apply(e).unwrap();
    }

    // Pay £10.00
    let evs = handle_check_command(
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
    for e in &evs {
        check.apply(e).unwrap();
    }

    let tt = totals(&check);
    assert_eq!(tt.balance_due.amount, Decimal::new(0, 2));

    // Close table session
    let out = route_core_command(
        CoreContext {
            table: Some(&table),
            check: Some(&check),
            order: None,
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::CloseTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
            reason: "paid".into(),
        }),
    )
    .unwrap();

    // Apply routed events to aggregates
    for e in out {
        match e {
            CoreEvent::Check(ce) => check.apply(&ce).unwrap(),
            CoreEvent::Table(te) => table.apply(&te).unwrap(),
            _ => {}
        }
    }

    assert_eq!(check.status, CheckStatus::Closed);
    assert_eq!(table.status, TableStatus::Available);
    assert_eq!(table.active_order_id, None);
}
