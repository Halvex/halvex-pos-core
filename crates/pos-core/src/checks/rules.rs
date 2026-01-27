use super::{Check, CheckStatus, Payment, PaymentStatus};
use pos_types::{PaymentID, PosError};

pub fn ensure_check_open(c: &Check) -> Result<(), PosError> {
    if c.status != CheckStatus::Open {
        return Err(PosError::validation("check is not open"));
    }

    Ok(())
}

pub fn ensure_percent_valid(p: u8) -> Result<(), PosError> {
    if p > 100 {
        return Err(PosError::validation("percent must be between 0 and 100"));
    }

    Ok(())
}

pub fn ensure_qty_positive(qty: u32) -> Result<(), PosError> {
    if qty == 0 {
        return Err(PosError::validation("qty must be >= 1"));
    }

    Ok(())
}

pub fn ensure_payment_exists(check: &Check, payment_id: PaymentID) -> Result<(), PosError> {
    if check.payments.iter().all(|p| p.payment_id != payment_id) {
        return Err(PosError::NotFound("payment not found".into()));
    }

    Ok(())
}

#[allow(dead_code)]
pub fn ensure_payment_not_void_or_refunded(
    check: &Check,
    payment_id: PaymentID,
) -> Result<(), PosError> {
    let p = check
        .payments
        .iter()
        .find(|p| p.payment_id == payment_id)
        .ok_or_else(|| PosError::NotFound("payment not found".into()))?;

    if matches!(p.status, PaymentStatus::Voided | PaymentStatus::Refunded) {
        return Err(PosError::validation("payment is not active"));
    }

    Ok(())
}

pub fn ensure_currency_matches(check: &Check, amount: &pos_types::Money) -> Result<(), PosError> {
    // If check has a line, enforce same currecny
    if let Some(line) = check.lines.first() {
        if line.unit_price.currency != amount.currency {
            return Err(PosError::validation("currency mismatch"));
        }
    }

    Ok(())
}

pub fn get_payment(check: &Check, payment_id: PaymentID) -> Result<&Payment, PosError> {
    check
        .payments
        .iter()
        .find(|p| p.payment_id == payment_id)
        .ok_or_else(|| PosError::NotFound("payment not found".into()))
}

pub fn ensure_payment_can_capture(
    check: &Check,
    payment_id: PaymentID,
    amount: &pos_types::Money,
) -> Result<(), PosError> {
    let p = get_payment(check, payment_id)?;

    if matches!(p.status, PaymentStatus::Voided | PaymentStatus::Refunded) {
        return Err(PosError::validation("payment is not capturable"));
    }

    let remaining = p.authorised.amount - p.captured.amount;
    if amount.amount > remaining {
        return Err(PosError::validation(
            "capture amount exceeds authorised remaining",
        ));
    }

    Ok(())
}

pub fn ensure_payment_can_refund(
    check: &Check,
    payment_id: PaymentID,
    amount: &pos_types::Money,
) -> Result<(), PosError> {
    let p = get_payment(check, payment_id)?;

    if matches!(p.status, PaymentStatus::Voided) {
        return Err(PosError::validation("cannot refund a voided payment"));
    }

    let refundable = p.captured.amount - p.refunded.amount;
    if amount.amount > refundable {
        return Err(PosError::validation(
            "refund amount exceeds refundable amount",
        ));
    }

    Ok(())
}

pub fn ensure_payment_can_void(check: &Check, payment_id: PaymentID) -> Result<(), PosError> {
    let p = get_payment(check, payment_id)?;

    if p.captured.amount > rust_decimal::Decimal::ZERO {
        return Err(PosError::validation(
            "cannot void a captured payment; refund it instead",
        ));
    }

    Ok(())
}
