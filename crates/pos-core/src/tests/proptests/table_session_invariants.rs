use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, CheckStatus, TenderType};
use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::{handle_check_command, handle_table_command, Apply};
use crate::orders::Order;
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{CheckID, OrderID, OrderItemID, PaymentID, TableID, VenueID};

use super::strategies::{item_name, money_from_cents, price_cents, qty};

#[derive(Clone, Debug)]
enum SessionAction {
    OpenSession,
    AddLine {
        qty: u32,
        price_cents: i64,
        name: String,
    },
    RecordPayment {
        amount_cents: i64,
    },
    CloseSession,
    CloseTable,
    ReopenTable,
}

fn action_strategy() -> impl Strategy<Value = SessionAction> {
    prop_oneof![
        Just(SessionAction::OpenSession),
        (qty(), price_cents(), item_name()).prop_map(|(qty, price_cents, name)| {
            SessionAction::AddLine {
                qty,
                price_cents,
                name,
            }
        }),
        price_cents().prop_map(|amount_cents| SessionAction::RecordPayment { amount_cents }),
        Just(SessionAction::CloseSession),
        Just(SessionAction::CloseTable),
        Just(SessionAction::ReopenTable),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 200,
        max_shrink_iters: 500,
        .. ProptestConfig::default()
    })]

    #[test]
    fn table_session_invariants_hold(actions in prop::collection::vec(action_strategy(), 1..40)) {
        let venue_id = VenueID::new();
        let table_id = TableID::new();

        let mut table = Table::new(venue_id, table_id, "T1", None);
        let create_events = handle_table_command(
            None,
            TableCommand::CreateTable {
                venue_id,
                table_id,
                label: "T1".into(),
                area_id: None,
            },
        )
        .unwrap();
        for event in create_events {
            table.apply(&event).unwrap();
        }

        let mut current_order: Option<Order> = None;
        let mut current_check: Option<Check> = None;

        for action in actions {
            match action {
                SessionAction::OpenSession => {
                    let order_id = OrderID::new();
                    let check_id = CheckID::new();

                    let can_open = table.status != TableStatus::Closed && table.active_order_id.is_none();
                    let result = route_core_command(
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
                    );

                    if can_open {
                        prop_assert!(result.is_ok());
                        let mut order = Order::new(venue_id, order_id);
                        let mut check = Check::new(venue_id, check_id, order_id);

                        for event in result.unwrap() {
                            match event {
                                CoreEvent::Order(oe) => order.apply(&oe).unwrap(),
                                CoreEvent::Check(ce) => check.apply(&ce).unwrap(),
                                CoreEvent::Table(te) => table.apply(&te).unwrap(),
                            }
                        }

                        current_order = Some(order);
                        current_check = Some(check);
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
                SessionAction::AddLine { qty, price_cents, name } => {
                    let check = match current_check.as_mut() {
                        Some(check) if check.status == CheckStatus::Open => check,
                        _ => continue,
                    };

                    let order_item_id = OrderItemID::new();
                    let cmd = CheckCommand::AddLineFromOrderItem {
                        venue_id,
                        check_id: check.check_id,
                        order_item_id,
                        qty,
                        unit_price: money_from_cents(price_cents),
                        name,
                    };

                    let events = handle_check_command(Some(check), cmd).unwrap();
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                }
                SessionAction::RecordPayment { amount_cents } => {
                    let check = match current_check.as_mut() {
                        Some(check) if check.status == CheckStatus::Open => check,
                        _ => continue,
                    };

                    let cmd = CheckCommand::RecordPayment {
                        venue_id,
                        check_id: check.check_id,
                        payment_id: PaymentID::new(),
                        tender: TenderType::Card,
                        amount: money_from_cents(amount_cents),
                        tip: None,
                        processor: None,
                    };

                    let events = handle_check_command(Some(check), cmd).unwrap();
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                }
                SessionAction::CloseSession => {
                    let (order, check) = match (current_order.as_ref(), current_check.as_ref()) {
                        (Some(order), Some(check)) => (order, check),
                        _ => continue,
                    };

                    let totals = totals(check);
                    let can_close = table.active_order_id == Some(order.order_id)
                        && check.order_id == order.order_id
                        && totals.balance_due.amount <= Decimal::ZERO;

                    let result = route_core_command(
                        CoreContext {
                            table: Some(&table),
                            order: None,
                            check: Some(check),
                            to_check: None,
                        },
                        CoreCommand::Workflow(WorkflowCommand::CloseTableSession {
                            venue_id,
                            table_id,
                            order_id: order.order_id,
                            check_id: check.check_id,
                            reason: "paid".into(),
                        }),
                    );

                    if can_close {
                        prop_assert!(result.is_ok());
                        let mut check = current_check.take().unwrap();
                        for event in result.unwrap() {
                            match event {
                                CoreEvent::Check(ce) => check.apply(&ce).unwrap(),
                                CoreEvent::Table(te) => table.apply(&te).unwrap(),
                                _ => {}
                            }
                        }
                        current_order = None;
                        current_check = None;
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
                SessionAction::CloseTable => {
                    let can_close = table.status != TableStatus::Closed && table.active_order_id.is_none();
                    let result = handle_table_command(
                        Some(&table),
                        TableCommand::CloseTable { venue_id, table_id },
                    );

                    if can_close {
                        prop_assert!(result.is_ok());
                        for event in result.unwrap() {
                            table.apply(&event).unwrap();
                        }
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
                SessionAction::ReopenTable => {
                    let can_reopen = table.status == TableStatus::Closed;
                    let result = handle_table_command(
                        Some(&table),
                        TableCommand::ReopenTable { venue_id, table_id },
                    );

                    if can_reopen {
                        prop_assert!(result.is_ok());
                        for event in result.unwrap() {
                            table.apply(&event).unwrap();
                        }
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
            }

            if table.status == TableStatus::Available {
                prop_assert!(table.active_order_id.is_none());
            }
            if table.active_order_id.is_some() {
                prop_assert_eq!(table.status, TableStatus::Occupied);
            }
            if table.status == TableStatus::Closed {
                prop_assert!(table.active_order_id.is_none());
            }
        }
    }
}
