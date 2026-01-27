use proptest::prelude::*;

use crate::checks::{Check, CheckCommand};
use crate::engine::{handle_check_command, handle_order_command, handle_table_command, Apply};
use crate::orders::{Order, OrderCommand};
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{CheckID, MenuItemID, OrderID, OrderItemID, TableID, VenueID};

use super::strategies::{item_name, money_from_cents, price_cents, qty};

#[derive(Clone, Debug)]
enum NoPanicAction {
    OrderAddItem {
        qty: u32,
        price_cents: i64,
        name: String,
    },
    OrderRemoveItem,
    CheckAddLine {
        qty: u32,
        price_cents: i64,
        name: String,
    },
    CheckRemoveLine,
    TableAssign,
    TableUnassign,
}

fn action_strategy() -> impl Strategy<Value = NoPanicAction> {
    prop_oneof![
        (qty(), price_cents(), item_name()).prop_map(|(qty, price_cents, name)| {
            NoPanicAction::OrderAddItem {
                qty,
                price_cents,
                name,
            }
        }),
        Just(NoPanicAction::OrderRemoveItem),
        (qty(), price_cents(), item_name()).prop_map(|(qty, price_cents, name)| {
            NoPanicAction::CheckAddLine {
                qty,
                price_cents,
                name,
            }
        }),
        Just(NoPanicAction::CheckRemoveLine),
        Just(NoPanicAction::TableAssign),
        Just(NoPanicAction::TableUnassign),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 200,
        max_shrink_iters: 500,
        .. ProptestConfig::default()
    })]

    #[test]
    fn handlers_and_apply_do_not_panic(actions in prop::collection::vec(action_strategy(), 1..50)) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let venue_id = VenueID::new();
            let order_id = OrderID::new();
            let check_id = CheckID::new();
            let table_id = TableID::new();

            let mut order = Order::new(venue_id, order_id);
            let mut check = Check::new(venue_id, check_id, order_id);
            let mut table = Table::new(venue_id, table_id, "T1", None);

            let mut order_items: Vec<OrderItemID> = Vec::new();
            let mut check_lines: Vec<OrderItemID> = Vec::new();

            for action in actions {
                match action {
                    NoPanicAction::OrderAddItem { qty, price_cents, name } => {
                        let order_item_id = OrderItemID::new();
                        let cmd = OrderCommand::AddItem {
                            venue_id,
                            order_id,
                            order_item_id,
                            menu_item_id: MenuItemID::new(),
                            name,
                            unit_price: money_from_cents(price_cents),
                            qty,
                            notes: None,
                        };

                        if let Ok(events) = handle_order_command(Some(&order), cmd) {
                            for event in events {
                                order.apply(&event).unwrap();
                            }
                            order_items.push(order_item_id);
                        }
                    }
                    NoPanicAction::OrderRemoveItem => {
                        let order_item_id = match order_items.pop() {
                            Some(id) => id,
                            None => continue,
                        };
                        let cmd = OrderCommand::RemoveItem {
                            venue_id,
                            order_id,
                            order_item_id,
                            reason: "test".into(),
                        };

                        if let Ok(events) = handle_order_command(Some(&order), cmd) {
                            for event in events {
                                order.apply(&event).unwrap();
                            }
                        }
                    }
                    NoPanicAction::CheckAddLine { qty, price_cents, name } => {
                        let order_item_id = OrderItemID::new();
                        let cmd = CheckCommand::AddLineFromOrderItem {
                            venue_id,
                            check_id,
                            order_item_id,
                            qty,
                            unit_price: money_from_cents(price_cents),
                            name,
                        };

                        if let Ok(events) = handle_check_command(Some(&check), cmd) {
                            for event in events {
                                check.apply(&event).unwrap();
                            }
                            check_lines.push(order_item_id);
                        }
                    }
                    NoPanicAction::CheckRemoveLine => {
                        let order_item_id = match check_lines.pop() {
                            Some(id) => id,
                            None => continue,
                        };
                        let cmd = CheckCommand::RemoveLine {
                            venue_id,
                            check_id,
                            order_item_id,
                            reason: "test".into(),
                        };

                        if let Ok(events) = handle_check_command(Some(&check), cmd) {
                            for event in events {
                                check.apply(&event).unwrap();
                            }
                        }
                    }
                    NoPanicAction::TableAssign => {
                        if table.status == TableStatus::Closed || table.active_order_id.is_some() {
                            continue;
                        }
                        let cmd = TableCommand::AssignOrder {
                            venue_id,
                            table_id,
                            order_id,
                        };

                        if let Ok(events) = handle_table_command(Some(&table), cmd) {
                            for event in events {
                                table.apply(&event).unwrap();
                            }
                        }
                    }
                    NoPanicAction::TableUnassign => {
                        if table.active_order_id.is_none() {
                            continue;
                        }
                        let cmd = TableCommand::UnassignOrder {
                            venue_id,
                            table_id,
                            reason: "test".into(),
                        };

                        if let Ok(events) = handle_table_command(Some(&table), cmd) {
                            for event in events {
                                table.apply(&event).unwrap();
                            }
                        }
                    }
                }
            }
        }));

        prop_assert!(result.is_ok());
    }
}
