use rand::Rng;
use rust_decimal::Decimal;

use crate::checks::{CheckCommand, TenderType};
use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::Apply;
use crate::orders::OrderCommand;
use crate::tables::TableCommand;
use pos_types::{CheckID, MenuItemID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID};

use super::random_state::{gen_state, RandomState};

pub fn gen_core_command(rng: &mut impl Rng, state: &RandomState) -> CoreCommand {
    match rng.gen_range(0..=3) {
        0 => CoreCommand::Order(gen_order_command(rng, state)),
        1 => CoreCommand::Check(gen_check_command(rng, state)),
        2 => CoreCommand::Table(gen_table_command(rng, state)),
        _ => CoreCommand::Workflow(gen_workflow_command(rng, state)),
    }
}

fn gen_order_command(rng: &mut impl Rng, state: &RandomState) -> OrderCommand {
    let venue_id = VenueID::new();
    let order_id = pick_order_id(rng, state);
    match rng.gen_range(0..=3) {
        0 => OrderCommand::OpenOrder { venue_id, order_id },
        1 => {
            let cents = rng.gen_range(100..=5000);
            OrderCommand::AddItem {
                venue_id,
                order_id,
                order_item_id: OrderItemID::new(),
                menu_item_id: MenuItemID::new(),
                name: random_name(rng),
                unit_price: money_from_cents(rng, cents),
                qty: rng.gen_range(1..=5),
                notes: None,
            }
        }
        2 => OrderCommand::RemoveItem {
            venue_id,
            order_id,
            order_item_id: pick_order_item_id(rng, state),
            reason: "abuse".into(),
        },
        _ => OrderCommand::CloseOrder { venue_id, order_id },
    }
}

fn gen_check_command(rng: &mut impl Rng, state: &RandomState) -> CheckCommand {
    let venue_id = VenueID::new();
    let check_id = pick_check_id(rng, state);
    match rng.gen_range(0..=9) {
        0 => CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id: pick_order_id(rng, state),
        },
        1 => {
            let cents = rng.gen_range(100..=5000);
            CheckCommand::AddLineFromOrderItem {
                venue_id,
                check_id,
                order_item_id: OrderItemID::new(),
                qty: rng.gen_range(1..=5),
                unit_price: money_from_cents(rng, cents),
                name: random_name(rng),
            }
        }
        2 => CheckCommand::RemoveLine {
            venue_id,
            check_id,
            order_item_id: pick_check_line_id(rng, state),
            reason: "abuse".into(),
        },
        3 => CheckCommand::ApplyDiscountPercent {
            venue_id,
            check_id,
            label: "disc".into(),
            percent: rng.gen_range(0u8..=100u8),
            reason: "abuse".into(),
        },
        4 => CheckCommand::ApplyServiceChargePercent {
            venue_id,
            check_id,
            label: "svc".into(),
            percent: rng.gen_range(0u8..=100u8),
            reason: "abuse".into(),
        },
        5 => CheckCommand::CloseCheck { venue_id, check_id },
        6 => {
            let cents = rng.gen_range(100..=10000);
            CheckCommand::RecordPayment {
                venue_id,
                check_id,
                payment_id: PaymentID::new(),
                tender: TenderType::Card,
                amount: money_from_cents(rng, cents),
                tip: None,
                processor: None,
            }
        }
        7 => CheckCommand::VoidPayment {
            venue_id,
            check_id,
            payment_id: pick_payment_id(rng, state),
            reason: "abuse".into(),
        },
        8 => {
            let cents = rng.gen_range(100..=10000);
            CheckCommand::RecordRefund {
                venue_id,
                check_id,
                payment_id: pick_payment_id(rng, state),
                amount: money_from_cents(rng, cents),
                reason: "abuse".into(),
            }
        }
        _ => {
            let cents = rng.gen_range(100..=10000);
            CheckCommand::CapturePayment {
                venue_id,
                check_id,
                payment_id: pick_payment_id(rng, state),
                amount: money_from_cents(rng, cents),
            }
        }
    }
}

fn gen_table_command(rng: &mut impl Rng, state: &RandomState) -> TableCommand {
    let venue_id = VenueID::new();
    let table_id = pick_table_id(rng, state);
    match rng.gen_range(0..=4) {
        0 => TableCommand::CreateTable {
            venue_id,
            table_id,
            label: random_label(rng),
            area_id: None,
        },
        1 => TableCommand::AssignOrder {
            venue_id,
            table_id,
            order_id: pick_order_id(rng, state),
        },
        2 => TableCommand::UnassignOrder {
            venue_id,
            table_id,
            reason: "abuse".into(),
        },
        3 => TableCommand::CloseTable { venue_id, table_id },
        _ => TableCommand::ReopenTable { venue_id, table_id },
    }
}

fn gen_workflow_command(rng: &mut impl Rng, state: &RandomState) -> WorkflowCommand {
    let venue_id = VenueID::new();
    match rng.gen_range(0..=3) {
        0 => WorkflowCommand::OpenTableSession {
            venue_id,
            table_id: pick_table_id(rng, state),
            order_id: pick_order_id(rng, state),
            check_id: pick_check_id(rng, state),
        },
        1 => WorkflowCommand::CloseTableSession {
            venue_id,
            table_id: pick_table_id(rng, state),
            order_id: pick_order_id(rng, state),
            check_id: pick_check_id(rng, state),
            reason: "abuse".into(),
        },
        2 => WorkflowCommand::SplitCheckCreate {
            venue_id,
            from_check_id: pick_check_id(rng, state),
            new_check_id: CheckID::new(),
        },
        _ => WorkflowCommand::SplitCheckMoveLineQty {
            venue_id,
            from_check_id: pick_check_id(rng, state),
            to_check_id: pick_to_check_id(rng, state),
            order_item_id: pick_check_line_id(rng, state),
            qty: rng.gen_range(1..=5),
        },
    }
}

fn pick_order_id(rng: &mut impl Rng, state: &RandomState) -> OrderID {
    if rng.gen_bool(0.3) {
        if let Some(order) = &state.order {
            return order.order_id;
        }
    }
    OrderID::new()
}

fn pick_order_item_id(rng: &mut impl Rng, state: &RandomState) -> OrderItemID {
    if rng.gen_bool(0.3) {
        if let Some(order) = &state.order {
            if let Some(item) = order.items.get(rng.gen_range(0..order.items.len().max(1))) {
                return item.item_id;
            }
        }
    }
    OrderItemID::new()
}

fn pick_check_id(rng: &mut impl Rng, state: &RandomState) -> CheckID {
    if rng.gen_bool(0.3) {
        if let Some(check) = &state.check {
            return check.check_id;
        }
    }
    CheckID::new()
}

fn pick_to_check_id(rng: &mut impl Rng, state: &RandomState) -> CheckID {
    if rng.gen_bool(0.3) {
        if let Some(check) = &state.to_check {
            return check.check_id;
        }
    }
    CheckID::new()
}

fn pick_check_line_id(rng: &mut impl Rng, state: &RandomState) -> OrderItemID {
    if rng.gen_bool(0.3) {
        if let Some(check) = &state.check {
            if let Some(line) = check.lines.get(rng.gen_range(0..check.lines.len().max(1))) {
                return line.order_item_id;
            }
        }
    }
    OrderItemID::new()
}

fn pick_payment_id(rng: &mut impl Rng, state: &RandomState) -> PaymentID {
    if rng.gen_bool(0.3) {
        if let Some(check) = &state.check {
            if let Some(payment) = check
                .payments
                .get(rng.gen_range(0..check.payments.len().max(1)))
            {
                return payment.payment_id;
            }
        }
    }
    PaymentID::new()
}

fn pick_table_id(rng: &mut impl Rng, state: &RandomState) -> TableID {
    if rng.gen_bool(0.3) {
        if let Some(table) = &state.table {
            return table.table_id;
        }
    }
    TableID::new()
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

fn money_from_cents(rng: &mut impl Rng, cents: i64) -> Money {
    Money {
        amount: Decimal::new(cents, 2),
        currency: match rng.gen_range(0..=4) {
            0 => pos_types::Currency::GBP,
            1 => pos_types::Currency::EUR,
            2 => pos_types::Currency::USD,
            3 => pos_types::Currency::AUD,
            _ => pos_types::Currency::CAD,
        },
    }
}

#[test]
fn abuse_random_commands_never_panic() {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(0xC0FFEE);

    for _ in 0..5000 {
        let state = gen_state(&mut rng);
        let cmd = gen_core_command(&mut rng, &state);

        let ctx = CoreContext {
            table: state.table.as_ref(),
            order: state.order.as_ref(),
            check: state.check.as_ref(),
            to_check: state.to_check.as_ref(),
        };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            route_core_command(ctx, cmd)
        }));
        assert!(result.is_ok(), "panic occurred routing command");
        let _ = result.unwrap();
    }
}

#[test]
fn abuse_apply_never_panic() {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(12345);

    for _ in 0..2500 {
        let state = gen_state(&mut rng);
        let cmd = gen_core_command(&mut rng, &state);

        let ctx = CoreContext {
            table: state.table.as_ref(),
            order: state.order.as_ref(),
            check: state.check.as_ref(),
            to_check: state.to_check.as_ref(),
        };

        let routed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            route_core_command(ctx, cmd)
        }));
        assert!(routed.is_ok(), "panic occurred routing command");

        if let Ok(Ok(events)) = routed {
            let mut order = state.order.clone();
            let mut check = state.check.clone();
            let mut table = state.table.clone();

            let apply_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for ev in events {
                    match ev {
                        CoreEvent::Order(e) => {
                            if let Some(o) = order.as_mut() {
                                let _ = o.apply(&e);
                            }
                        }
                        CoreEvent::Check(e) => {
                            if let Some(c) = check.as_mut() {
                                let _ = c.apply(&e);
                            }
                        }
                        CoreEvent::Table(e) => {
                            if let Some(t) = table.as_mut() {
                                let _ = t.apply(&e);
                            }
                        }
                    }
                }
            }));

            assert!(apply_res.is_ok(), "panic occurred during apply");
        }
    }
}
