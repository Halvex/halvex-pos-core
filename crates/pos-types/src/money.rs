use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Money {
    /// Amount in major units (e.g. GBP)
    pub amount: Decimal,
    pub currency: Currency,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    GBP,
    EUR,
    USD,
    AUD,
    CAD,
}

impl Money {
    pub fn gbp(amount: Decimal) -> Self {
        Self {
            amount,
            currency: Currency::GBP,
        }
    }

    pub fn eur(amount: Decimal) -> Self {
        Self {
            amount,
            currency: Currency::EUR,
        }
    }

    pub fn usd(amount: Decimal) -> Self {
        Self {
            amount,
            currency: Currency::USD,
        }
    }

    pub fn aud(amount: Decimal) -> Self {
        Self {
            amount,
            currency: Currency::AUD,
        }
    }

    pub fn cad(amount: Decimal) -> Self {
        Self {
            amount,
            currency: Currency::CAD,
        }
    }
}
