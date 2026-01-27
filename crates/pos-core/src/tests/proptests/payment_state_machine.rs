use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand, PaymentStatus, TenderType};
use crate::engine::{handle_check_command, Apply};
use pos_types::{CheckID, OrderID, OrderItemID, PaymentID, VenueID};

use super::strategies::{money_from_cents, price_cents, qty};

#[derive(Clone, Debug)]
enum PaymentAction {
    Capture { amount_cents: i64 },
    Refund { amount_cents: i64 },
    Void,
}

fn action_strategy() -> impl Strategy<Value = PaymentAction> {
    prop_oneof![
        price_cents().prop_map(|amount_cents| PaymentAction::Capture { amount_cents }),
        price_cents().prop_map(|amount_cents| PaymentAction::Refund { amount_cents }),
        Just(PaymentAction::Void),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 200,
        max_shrink_iters: 500,
        .. ProptestConfig::default()
    })]

    #[test]
    fn payment_state_machine_boundaries(
        line_qty in qty(),
        line_price_cents in price_cents(),
        authorised_cents in price_cents(),
        actions in prop::collection::vec(action_strategy(), 1..40),
    ) {
        let venue_id = VenueID::new();
        let check_id = CheckID::new();
        let order_id = OrderID::new();
        let mut check = Check::new(venue_id, check_id, order_id);

        let order_item_id = OrderItemID::new();
        let add_cmd = CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id,
            qty: line_qty,
            unit_price: money_from_cents(line_price_cents),
            name: "Item".into(),
        };

        let events = handle_check_command(Some(&check), add_cmd).unwrap();
        for event in events {
            check.apply(&event).unwrap();
        }

        let payment_id = PaymentID::new();
        let auth_cmd = CheckCommand::AuthorisePayment {
            venue_id,
            check_id,
            payment_id,
            tender: TenderType::Card,
            amount: money_from_cents(authorised_cents),
            tip: None,
            processor: None,
        };

        let events = handle_check_command(Some(&check), auth_cmd).unwrap();
        for event in events {
            check.apply(&event).unwrap();
        }

        for action in actions {
            let payment = check
                .payments
                .iter()
                .find(|p| p.payment_id == payment_id)
                .expect("payment missing");

            match action {
                PaymentAction::Capture { amount_cents } => {
                    let amount = money_from_cents(amount_cents);
                    let remaining = payment.authorised.amount - payment.captured.amount;
                    let can_capture = !matches!(payment.status, PaymentStatus::Voided | PaymentStatus::Refunded)
                        && amount.amount <= remaining;

                    let cmd = CheckCommand::CapturePayment {
                        venue_id,
                        check_id,
                        payment_id,
                        amount,
                    };

                    let result = handle_check_command(Some(&check), cmd);
                    if can_capture {
                        prop_assert!(result.is_ok());
                        for event in result.unwrap() {
                            check.apply(&event).unwrap();
                        }
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
                PaymentAction::Refund { amount_cents } => {
                    let amount = money_from_cents(amount_cents);
                    let refundable = payment.captured.amount - payment.refunded.amount;
                    let can_refund = !matches!(payment.status, PaymentStatus::Voided)
                        && amount.amount <= refundable;

                    let cmd = CheckCommand::RecordRefund {
                        venue_id,
                        check_id,
                        payment_id,
                        amount,
                        reason: "test".into(),
                    };

                    let result = handle_check_command(Some(&check), cmd);
                    if can_refund {
                        prop_assert!(result.is_ok());
                        for event in result.unwrap() {
                            check.apply(&event).unwrap();
                        }
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
                PaymentAction::Void => {
                    let can_void = payment.captured.amount == Decimal::ZERO;
                    let cmd = CheckCommand::VoidPayment {
                        venue_id,
                        check_id,
                        payment_id,
                        reason: "test".into(),
                    };

                    let result = handle_check_command(Some(&check), cmd);
                    if can_void {
                        prop_assert!(result.is_ok());
                        for event in result.unwrap() {
                            check.apply(&event).unwrap();
                        }
                    } else {
                        prop_assert!(result.is_err());
                    }
                }
            }

            for payment in &check.payments {
                prop_assert!(payment.captured.amount <= payment.authorised.amount);
                prop_assert!(payment.refunded.amount <= payment.captured.amount);
                if payment.status == PaymentStatus::Voided {
                    prop_assert_eq!(payment.captured.amount, Decimal::ZERO);
                }
            }

            let totals = totals(&check);
            let paid_sum: Decimal = check
                .payments
                .iter()
                .filter_map(|p| match p.status {
                    PaymentStatus::Captured | PaymentStatus::Refunded => {
                        Some(p.captured.amount - p.refunded.amount)
                    }
                    _ => None,
                })
                .sum();

            prop_assert_eq!(totals.paid.amount, paid_sum);
            prop_assert_eq!(totals.balance_due.amount, totals.total.amount - totals.paid.amount);
        }
    }
}
