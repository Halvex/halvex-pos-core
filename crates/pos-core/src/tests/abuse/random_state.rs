use rand::Rng;
use rust_decimal::Decimal;

use crate::checks::{
    Check, CheckLine, CheckStatus, Discount, Payment, PaymentStatus, ServiceCharge, TenderType,
};
use crate::orders::{Order, OrderItem, OrderStatus};
use crate::tables::{Table, TableStatus};
use pos_types::{
    CheckID, Currency, MenuItemID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID,
};

#[derive(Clone, Debug)]
pub struct RandomState {
    pub order: Option<Order>,
    pub check: Option<Check>,
    pub table: Option<Table>,
    pub to_check: Option<Check>,
}

pub fn gen_state(rng: &mut impl Rng) -> RandomState {
    let order = if rng.gen_bool(0.7) {
        Some(gen_order(rng))
    } else {
        None
    };

    let check = if rng.gen_bool(0.7) {
        Some(gen_check(rng))
    } else {
        None
    };

    let to_check = if rng.gen_bool(0.3) {
        Some(gen_check(rng))
    } else {
        None
    };

    let table = if rng.gen_bool(0.7) {
        Some(gen_table(rng))
    } else {
        None
    };

    RandomState {
        order,
        check,
        table,
        to_check,
    }
}

fn gen_order(rng: &mut impl Rng) -> Order {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let mut order = Order::new(venue_id, order_id);

    let item_count = rng.gen_range(0..=5);
    for _ in 0..item_count {
        let qty = rng.gen_range(1..=5);
        let cents = rng.gen_range(100..=5000);
        order.items.push(OrderItem {
            item_id: OrderItemID::new(),
            menu_item_id: MenuItemID::new(),
            name: random_name(rng),
            unit_price: money_from_cents(rng, cents),
            qty,
            notes: if rng.gen_bool(0.3) {
                Some("note".into())
            } else {
                None
            },
        });
    }

    order.status = match rng.gen_range(0..=2) {
        0 => OrderStatus::Open,
        1 => OrderStatus::Closed,
        _ => OrderStatus::Cancelled,
    };
    order.version = rng.gen_range(0..=100);
    order
}

fn gen_check(rng: &mut impl Rng) -> Check {
    let venue_id = VenueID::new();
    let check_id = CheckID::new();
    let order_id = OrderID::new();
    let mut check = Check::new(venue_id, check_id, order_id);

    let line_count = rng.gen_range(0..=5);
    for _ in 0..line_count {
        let qty = rng.gen_range(1..=5);
        let cents = rng.gen_range(100..=5000);
        check.lines.push(CheckLine {
            order_item_id: OrderItemID::new(),
            qty,
            unit_price: money_from_cents(rng, cents),
            name: random_name(rng),
        });
    }

    if rng.gen_bool(0.4) {
        check.discount = Some(Discount::Percent {
            label: "disc".into(),
            percent: rng.gen_range(0u8..=100u8),
        });
    }

    if rng.gen_bool(0.4) {
        check.service_charge = Some(ServiceCharge::Percent {
            label: "svc".into(),
            percent: rng.gen_range(0u8..=100u8),
        });
    }

    let payment_count = rng.gen_range(0..=3);
    for _ in 0..payment_count {
        let authorised_cents = rng.gen_range(100..=8000);
        let captured_cents = rng.gen_range(0..=authorised_cents);
        let refunded_cents = rng.gen_range(0..=captured_cents);
        let currency = random_currency(rng);

        let authorised = Money {
            amount: Decimal::new(authorised_cents, 2),
            currency,
        };
        let captured = Money {
            amount: Decimal::new(captured_cents, 2),
            currency,
        };
        let refunded = Money {
            amount: Decimal::new(refunded_cents, 2),
            currency,
        };

        let status = if refunded.amount >= captured.amount && captured.amount > Decimal::ZERO {
            PaymentStatus::Refunded
        } else if captured.amount > Decimal::ZERO {
            PaymentStatus::Captured
        } else if rng.gen_bool(0.2) {
            PaymentStatus::Voided
        } else {
            PaymentStatus::Authorized
        };

        check.payments.push(Payment {
            payment_id: PaymentID::new(),
            tender: TenderType::Card,
            authorised,
            captured,
            refunded,
            tip: None,
            status,
            processor: None,
        });
    }

    check.status = match rng.gen_range(0..=2) {
        0 => CheckStatus::Open,
        1 => CheckStatus::Closed,
        _ => CheckStatus::Voided,
    };
    check.version = rng.gen_range(0..=100);
    check
}

fn gen_table(rng: &mut impl Rng) -> Table {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let mut table = Table::new(venue_id, table_id, random_label(rng), None);

    let status = match rng.gen_range(0..=2) {
        0 => TableStatus::Available,
        1 => TableStatus::Occupied,
        _ => TableStatus::Closed,
    };

    table.status = status;
    table.active_order_id = match status {
        TableStatus::Occupied => Some(OrderID::new()),
        _ => None,
    };
    table.version = rng.gen_range(0..=100);
    table
}

fn random_name(rng: &mut impl Rng) -> String {
    let idx = rng.gen_range(0..=4);
    match idx {
        0 => "Burger",
        1 => "Fries",
        2 => "Salad",
        3 => "Pizza",
        _ => "Soda",
    }
    .to_string()
}

fn random_label(rng: &mut impl Rng) -> String {
    format!("T{}", rng.gen_range(1..=20))
}

fn random_currency(rng: &mut impl Rng) -> Currency {
    match rng.gen_range(0..=4) {
        0 => Currency::GBP,
        1 => Currency::EUR,
        2 => Currency::USD,
        3 => Currency::AUD,
        _ => Currency::CAD,
    }
}

fn money_from_cents(rng: &mut impl Rng, cents: i64) -> Money {
    Money {
        amount: Decimal::new(cents, 2),
        currency: random_currency(rng),
    }
}
