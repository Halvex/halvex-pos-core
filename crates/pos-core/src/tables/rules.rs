use super::{Table, TableStatus};
use pos_types::PosError;

pub fn ensure_table_not_closed(t: &Table) -> Result<(), PosError> {
    if t.status == TableStatus::Closed {
        return Err(PosError::validation("table is closed"));
    }

    Ok(())
}

pub fn ensure_table_has_no_active_order(t: &Table) -> Result<(), PosError> {
    if t.active_order_id.is_some() {
        return Err(PosError::validation("table already has an active order"));
    }

    Ok(())
}

pub fn ensure_table_has_active_order(t: &Table) -> Result<(), PosError> {
    if t.active_order_id.is_none() {
        return Err(PosError::validation("table has no active order"));
    }

    Ok(())
}
