use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, CheckEvent, CheckStatus, TenderType};
use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::Apply;
use crate::orders::{Order, OrderCommand};
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{CheckID, MenuItemID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID};

#[test]
fn mvp_flow_end_to_end() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();
    let split_check_id = CheckID::new();

    // create table
    let mut table = Table::new(venue_id, table_id, "T1", None);
    let events = route_core_command(
        CoreContext::empty(),
        CoreCommand::Table(TableCommand::CreateTable {
            venue_id,
            table_id,
            label: "T1".into(),
            area_id: None,
        }),
    )
    .unwrap();
    for e in events {
        if let CoreEvent::Table(te) = e {
            table.apply(&te).unwrap();
        }
    }

    // open table session (order + check + table assignment)
    let events = route_core_command(
        CoreContext {
            table: Some(&table),
            order: None,
            check: None,
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::OpenTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
        }),
    )
    .unwrap();

    let mut order = Order::new(venue_id, order_id);
    let mut check = Check::new(venue_id, check_id, order_id);

    for e in events {
        match e {
            CoreEvent::Order(oe) => order.apply(&oe).unwrap(),
            CoreEvent::Check(ce) => check.apply(&ce).unwrap(),
            CoreEvent::Table(te) => table.apply(&te).unwrap(),
        }
    }

    // add order item
    let order_item_id = OrderItemID::new();
    let menu_item_id = MenuItemID::new();
    let unit_price = Money::gbp(Decimal::new(1000, 2));

    let events = route_core_command(
        CoreContext {
            order: Some(&order),
            check: None,
            table: None,
            to_check: None,
        },
        CoreCommand::Order(OrderCommand::AddItem {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name: "Burger".into(),
            unit_price,
            qty: 2,
            notes: None,
        }),
    )
    .unwrap();
    for e in events {
        if let CoreEvent::Order(oe) = e {
            order.apply(&oe).unwrap();
        }
    }

    // add line to check from order item
    let events = route_core_command(
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        CoreCommand::Check(CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id,
            qty: 2,
            unit_price,
            name: "Burger".into(),
        }),
    )
    .unwrap();
    for e in events {
        if let CoreEvent::Check(ce) = e {
            check.apply(&ce).unwrap();
        }
    }

    // split check
    let events = route_core_command(
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::SplitCheckCreate {
            venue_id,
            from_check_id: check_id,
            new_check_id: split_check_id,
        }),
    )
    .unwrap();

    let mut split_check = Check::new(venue_id, split_check_id, order_id);
    for e in events {
        if let CoreEvent::Check(ce) = e {
            split_check.apply(&ce).unwrap();
        }
    }

    let events = route_core_command(
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: Some(&split_check),
        },
        CoreCommand::Workflow(WorkflowCommand::SplitCheckMoveLineQty {
            venue_id,
            from_check_id: check_id,
            to_check_id: split_check_id,
            order_item_id,
            qty: 2,
        }),
    )
    .unwrap();

    for e in events {
        if let CoreEvent::Check(ce) = e {
            match &ce {
                CheckEvent::LineQtyDecreased {
                    check_id: from_id, ..
                } => {
                    if *from_id == check_id {
                        check.apply(&ce).unwrap();
                    }
                }
                CheckEvent::LineAdded {
                    check_id: to_id, ..
                } => {
                    if *to_id == split_check_id {
                        split_check.apply(&ce).unwrap();
                    }
                }
                _ => {}
            }
        }
    }

    // authorise + capture payment on split check
    let payment_id = PaymentID::new();
    let amount = Money::gbp(Decimal::new(2000, 2));

    let events = route_core_command(
        CoreContext {
            order: None,
            check: Some(&split_check),
            table: None,
            to_check: None,
        },
        CoreCommand::Check(CheckCommand::AuthorisePayment {
            venue_id,
            check_id: split_check_id,
            payment_id,
            tender: TenderType::Card,
            amount,
            tip: None,
            processor: None,
        }),
    )
    .unwrap();
    for e in events {
        if let CoreEvent::Check(ce) = e {
            split_check.apply(&ce).unwrap();
        }
    }

    let events = route_core_command(
        CoreContext {
            order: None,
            check: Some(&split_check),
            table: None,
            to_check: None,
        },
        CoreCommand::Check(CheckCommand::CapturePayment {
            venue_id,
            check_id: split_check_id,
            payment_id,
            amount,
        }),
    )
    .unwrap();
    for e in events {
        if let CoreEvent::Check(ce) = e {
            split_check.apply(&ce).unwrap();
        }
    }

    // close table session using split check
    let events = route_core_command(
        CoreContext {
            order: Some(&order),
            check: Some(&split_check),
            table: Some(&table),
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::CloseTableSession {
            venue_id,
            table_id,
            order_id,
            check_id: split_check_id,
            reason: "settled".into(),
        }),
    )
    .unwrap();

    for e in events {
        match e {
            CoreEvent::Check(ce) => split_check.apply(&ce).unwrap(),
            CoreEvent::Table(te) => table.apply(&te).unwrap(),
            CoreEvent::Order(_) => {}
        }
    }

    let t = totals(&split_check);
    assert_eq!(t.balance_due.amount, Decimal::new(0, 2));
    assert_eq!(split_check.status, CheckStatus::Closed);
    assert_eq!(table.status, TableStatus::Available);
    assert!(table.active_order_id.is_none());
}
