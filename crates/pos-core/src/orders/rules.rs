use super::{Order, OrderStatus};
use pos_types::PosError;

pub fn ensure_order_open(order: &Order) -> Result<(), PosError> {
    if order.status != OrderStatus::Open {
        return Err(PosError::validation("order is not open"));
    }

    Ok(())
}

pub fn ensure_qty_positive(qty: u32) -> Result<(), PosError> {
    if qty == 0 {
        return Err(PosError::validation("qty must be >= 1"));
    }

    Ok(())
}
