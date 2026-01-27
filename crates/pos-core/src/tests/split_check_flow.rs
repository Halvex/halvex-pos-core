use rust_decimal::Decimal;

use crate::checks::{totals, Check, CheckCommand};
use crate::engine::{handle_check_command, split_check_create, split_check_move_line_qty, Apply};
use pos_types::{CheckID, Money, OrderID, OrderItemID, VenueID};

#[test]
fn can_split_check_and_move_qty_between_checks() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let from_check_id = CheckID::new();
    let to_check_id = CheckID::new();

    // open FROM check
    let ev = handle_check_command(
        None,
        CheckCommand::OpenCheck {
            venue_id,
            check_id: from_check_id,
            order_id,
        },
    )
    .unwrap();
    let mut from = Check::new(venue_id, from_check_id, order_id);
    for e in &ev {
        from.apply(e).unwrap();
    }

    // add line: 2x 10.00
    let item_id = OrderItemID::new();
    let ev = handle_check_command(
        Some(&from),
        CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id: from_check_id,
            order_item_id: item_id,
            qty: 2,
            unit_price: Money::gbp(Decimal::new(1000, 2)),
            name: "Pasta".into(),
        },
    )
    .unwrap();
    for e in &ev {
        from.apply(e).unwrap();
    }

    assert_eq!(totals(&from).total.amount, Decimal::new(2000, 2));

    // create split check (TO)
    let evs = split_check_create(&from, venue_id, to_check_id).unwrap();
    let mut to = Check::new(venue_id, to_check_id, order_id);
    for e in &evs {
        // route by check_id
        match e {
            crate::checks::CheckEvent::CheckOpened { check_id, .. }
            | crate::checks::CheckEvent::SplitCheckCreated { check_id, .. } => {
                if *check_id == to_check_id {
                    to.apply(e).unwrap();
                }
            }
            _ => {}
        }
    }

    // move qty=1 from FROM -> TO
    let evs = split_check_move_line_qty(&from, &to, venue_id, item_id, 1).unwrap();

    for e in &evs {
        // route events to correct aggregate
        match e {
            crate::checks::CheckEvent::LineQtyDecreased { check_id, .. } => {
                if *check_id == from_check_id {
                    from.apply(e).unwrap();
                }
            }
            crate::checks::CheckEvent::LineAdded { check_id, .. } => {
                if *check_id == to_check_id {
                    to.apply(e).unwrap();
                }
            }
            _ => {}
        }
    }

    assert_eq!(from.lines[0].qty, 1);
    assert_eq!(to.lines[0].qty, 1);

    assert_eq!(totals(&from).total.amount, Decimal::new(1000, 2));
    assert_eq!(totals(&to).total.amount, Decimal::new(1000, 2));
}
