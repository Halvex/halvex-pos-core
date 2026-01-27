use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand};
use crate::engine::{handle_check_command, Apply};
use pos_types::{CheckID, OrderID, OrderItemID, VenueID};

use super::strategies::{item_name, money_from_cents, percent, price_cents, qty};

#[derive(Clone, Debug)]
enum CheckAction {
    AddLine {
        qty: u32,
        price_cents: i64,
        name: String,
    },
    RemoveLine,
    ApplyDiscount {
        percent: u8,
    },
    ApplyServiceCharge {
        percent: u8,
    },
}

fn action_strategy() -> impl Strategy<Value = CheckAction> {
    prop_oneof![
        (qty(), price_cents(), item_name()).prop_map(|(qty, price_cents, name)| {
            CheckAction::AddLine {
                qty,
                price_cents,
                name,
            }
        }),
        Just(CheckAction::RemoveLine),
        percent().prop_map(|percent| CheckAction::ApplyDiscount { percent }),
        percent().prop_map(|percent| CheckAction::ApplyServiceCharge { percent }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 200,
        max_shrink_iters: 500,
        .. ProptestConfig::default()
    })]

    #[test]
    fn check_totals_invariants_hold(actions in prop::collection::vec(action_strategy(), 1..50)) {
        let venue_id = VenueID::new();
        let check_id = CheckID::new();
        let order_id = OrderID::new();

        let mut check = Check::new(venue_id, check_id, order_id);
        let mut line_ids: Vec<OrderItemID> = Vec::new();

        for action in actions {
            match action {
                CheckAction::AddLine { qty, price_cents, name } => {
                    let order_item_id = OrderItemID::new();
                    let cmd = CheckCommand::AddLineFromOrderItem {
                        venue_id,
                        check_id,
                        order_item_id,
                        qty,
                        unit_price: money_from_cents(price_cents),
                        name,
                    };

                    let events = handle_check_command(Some(&check), cmd).expect("add line failed");
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                    line_ids.push(order_item_id);
                }
                CheckAction::RemoveLine => {
                    let order_item_id = match line_ids.pop() {
                        Some(id) => id,
                        None => continue,
                    };

                    let cmd = CheckCommand::RemoveLine {
                        venue_id,
                        check_id,
                        order_item_id,
                        reason: "test".into(),
                    };

                    let events = handle_check_command(Some(&check), cmd).expect("remove line failed");
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                }
                CheckAction::ApplyDiscount { percent } => {
                    let cmd = CheckCommand::ApplyDiscountPercent {
                        venue_id,
                        check_id,
                        label: "discount".into(),
                        percent,
                        reason: "proptest".into(),
                    };

                    let events = handle_check_command(Some(&check), cmd).expect("discount failed");
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                }
                CheckAction::ApplyServiceCharge { percent } => {
                    let cmd = CheckCommand::ApplyServiceChargePercent {
                        venue_id,
                        check_id,
                        label: "service".into(),
                        percent,
                        reason: "proptest".into(),
                    };

                    let events = handle_check_command(Some(&check), cmd).expect("service failed");
                    for event in events {
                        check.apply(&event).unwrap();
                    }
                }
            }

            let totals = totals(&check);

            for line in &check.lines {
                prop_assert!(line.qty > 0);
                prop_assert_eq!(line.unit_price.currency, totals.total.currency);
            }

            prop_assert_eq!(
                totals.total.amount,
                totals.subtotal.amount - totals.discount.amount + totals.service_charge.amount
            );
            prop_assert_eq!(
                totals.balance_due.amount,
                totals.total.amount - totals.paid.amount
            );
            prop_assert_eq!(totals.paid.amount, Decimal::ZERO);
            prop_assert_eq!(totals.total.currency, totals.balance_due.currency);
        }
    }
}
