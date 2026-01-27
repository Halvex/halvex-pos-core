use crate::checks::{ensure_check_open, ensure_qty_positive, Check, CheckEvent};
use pos_types::{CheckID, OrderItemID, PosError, VenueID};

/// Create a split check from an existing check.
/// Returns events for the *new* check ( Open + Metadata )
pub fn split_check_create(
    from: &Check,
    venue_id: VenueID,
    new_check_id: CheckID,
) -> Result<Vec<CheckEvent>, PosError> {
    // Creates a new check with the same order_id
    let events = vec![
        CheckEvent::CheckOpened {
            venue_id,
            check_id: new_check_id,
            order_id: from.order_id,
        },
        CheckEvent::SplitCheckCreated {
            venue_id,
            check_id: new_check_id,
            from_check_id: from.check_id,
        },
    ];

    Ok(events)
}

/// Move a quantity of an order item line from one check to another.
/// Emits:
/// - LineQtyDecreased on FROM check
/// - LineAdded on TO check (reusing existing event)
pub fn split_check_move_line_qty(
    from: &Check,
    to: &Check,
    venue_id: VenueID,
    order_item_id: OrderItemID,
    qty: u32,
) -> Result<Vec<CheckEvent>, PosError> {
    ensure_check_open(from)?;
    ensure_check_open(to)?;
    ensure_qty_positive(qty)?;

    // Ensure that they are the same order (for resturant splits)
    if from.order_id != to.order_id {
        return Err(PosError::validation(
            "cannot move line between checks of different orders",
        ));
    }

    let from_line = from
        .lines
        .iter()
        .find(|l| l.order_item_id == order_item_id)
        .ok_or_else(|| PosError::NotFound("line not found on from_check".into()))?;

    if from_line.qty < qty {
        return Err(PosError::validation(
            "cannot move more qty than exists on from_check",
        ));
    }

    Ok(vec![
        CheckEvent::LineQtyDecreased {
            venue_id,
            check_id: from.check_id,
            order_item_id,
            qty,
            reason: format!("moved to check {}", to.check_id.0),
        },
        CheckEvent::LineAdded {
            venue_id,
            check_id: to.check_id,
            order_item_id,
            qty,
            unit_price: from_line.unit_price,
            name: from_line.name.clone(),
        },
    ])
}
