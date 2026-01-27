use rust_decimal::Decimal;

use crate::checks::{Check, CheckCommand, CheckEvent};
use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::Apply;
use pos_types::{CheckID, Money, OrderID, OrderItemID, VenueID};

#[test]
fn can_route_check_command_and_workflow_split() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let from_check_id = CheckID::new();
    let to_check_id = CheckID::new();

    // open FROM check via CoreCommand
    let evs = route_core_command(
        CoreContext {
            check: None,
            order: None,
            table: None,
            to_check: None,
        },
        CoreCommand::Check(CheckCommand::OpenCheck {
            venue_id,
            check_id: from_check_id,
            order_id,
        }),
    )
    .unwrap();

    let mut from = Check::new(venue_id, from_check_id, order_id);
    for e in evs {
        if let CoreEvent::Check(ce) = e {
            from.apply(&ce).unwrap();
        }
    }

    // add 2x item
    let item_id = OrderItemID::new();
    let evs = route_core_command(
        CoreContext {
            check: Some(&from),
            order: None,
            table: None,
            to_check: None,
        },
        CoreCommand::Check(CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id: from_check_id,
            order_item_id: item_id,
            qty: 2,
            unit_price: Money::gbp(Decimal::new(1000, 2)),
            name: "Pasta".into(),
        }),
    )
    .unwrap();
    for e in evs {
        if let CoreEvent::Check(ce) = e {
            from.apply(&ce).unwrap();
        }
    }

    // create split check via workflow
    let evs = route_core_command(
        CoreContext {
            check: Some(&from),
            order: None,
            table: None,
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::SplitCheckCreate {
            venue_id,
            from_check_id,
            new_check_id: to_check_id,
        }),
    )
    .unwrap();

    let mut to = Check::new(venue_id, to_check_id, order_id);
    for e in evs {
        if let CoreEvent::Check(ce) = e {
            // route by check_id
            match &ce {
                CheckEvent::CheckOpened { check_id, .. }
                | CheckEvent::SplitCheckCreated { check_id, .. } => {
                    if *check_id == to_check_id {
                        to.apply(&ce).unwrap();
                    }
                }
                _ => {}
            }
        }
    }

    // move qty=1 via workflow
    let evs = route_core_command(
        CoreContext {
            check: Some(&from),
            order: None,
            table: None,
            to_check: Some(&to),
        },
        CoreCommand::Workflow(WorkflowCommand::SplitCheckMoveLineQty {
            venue_id,
            from_check_id,
            to_check_id,
            order_item_id: item_id,
            qty: 1,
        }),
    )
    .unwrap();

    for e in evs {
        if let CoreEvent::Check(ce) = e {
            match &ce {
                CheckEvent::LineQtyDecreased { check_id, .. } => {
                    if *check_id == from_check_id {
                        from.apply(&ce).unwrap();
                    }
                }
                CheckEvent::LineAdded { check_id, .. } => {
                    if *check_id == to_check_id {
                        to.apply(&ce).unwrap();
                    }
                }
                _ => {}
            }
        }
    }

    assert_eq!(from.lines[0].qty, 1);
    assert_eq!(to.lines[0].qty, 1);
}
