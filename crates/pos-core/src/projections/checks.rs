use crate::checks::{totals, Check, CheckStatus};
use pos_types::Money;

/// Returns true if the check can be closed with current state.
pub fn can_close_check(check: &Check) -> bool {
    if check.status != CheckStatus::Open {
        return false;
    }

    let t = totals(check);
    t.balance_due.amount <= rust_decimal::Decimal::ZERO
}

/// Returns the current balance due (total - paid).
pub fn check_balance_due(check: &Check) -> Money {
    totals(check).balance_due
}
