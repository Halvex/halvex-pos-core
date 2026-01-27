use super::{Check, Discount, ServiceCharge};
use pos_types::Money;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckTotals {
    pub subtotal: Money,
    pub discount: Money,
    pub service_charge: Money,
    pub total: Money,

    pub paid: Money,
    pub tip: Money,
    pub balance_due: Money,
}

pub fn totals(check: &Check) -> CheckTotals {
    // Empty checks default to GBP until a line establishes the currency.
    let currency = check
        .lines
        .first()
        .map(|l| l.unit_price.currency)
        .unwrap_or(pos_types::Currency::GBP);

    let subtotal_amount: Decimal = check
        .lines
        .iter()
        .map(|l| l.unit_price.amount * Decimal::from(l.qty))
        .sum();

    let discount_amount = match &check.discount {
        Some(Discount::Percent { percent, .. }) => {
            subtotal_amount * Decimal::from(*percent) / Decimal::from(100u32)
        }
        None => Decimal::ZERO,
    };

    let after_discount = subtotal_amount - discount_amount;

    let service_amount = match &check.service_charge {
        Some(ServiceCharge::Percent { percent, .. }) => {
            after_discount * Decimal::from(*percent) / Decimal::from(100u32)
        }
        None => Decimal::ZERO,
    };

    let total_amount = after_discount + service_amount;

    let paid_amount: Decimal = check
        .payments
        .iter()
        .filter_map(|p| match p.status {
            super::PaymentStatus::Captured | super::PaymentStatus::Refunded => {
                Some(p.captured.amount - p.refunded.amount)
            }
            _ => None,
        })
        .sum();

    let tip_amount: Decimal = check
        .payments
        .iter()
        .filter_map(|p| match p.status {
            super::PaymentStatus::Captured | super::PaymentStatus::Refunded => {
                p.tip.map(|t| t.amount)
            }
            _ => None,
        })
        .sum();

    let balance_due_amount = total_amount - paid_amount;

    CheckTotals {
        subtotal: Money {
            amount: subtotal_amount,
            currency,
        },
        discount: Money {
            amount: discount_amount,
            currency,
        },
        service_charge: Money {
            amount: service_amount,
            currency,
        },
        total: Money {
            amount: total_amount,
            currency,
        },

        paid: Money {
            amount: paid_amount,
            currency,
        },
        tip: Money {
            amount: tip_amount,
            currency,
        },
        balance_due: Money {
            amount: balance_due_amount,
            currency,
        },
    }
}
