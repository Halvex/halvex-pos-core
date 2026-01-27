use proptest::prelude::*;
use rust_decimal::Decimal;

use pos_types::Money;

pub fn qty() -> impl Strategy<Value = u32> {
    1u32..=10
}

pub fn price_cents() -> impl Strategy<Value = i64> {
    100i64..=5000i64
}

pub fn percent() -> impl Strategy<Value = u8> {
    0u8..=50u8
}

pub fn item_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Burger".to_string()),
        Just("Fries".to_string()),
        Just("Salad".to_string()),
        Just("Pizza".to_string()),
        Just("Soda".to_string()),
    ]
}

pub fn money_from_cents(cents: i64) -> Money {
    Money::gbp(Decimal::new(cents, 2))
}
