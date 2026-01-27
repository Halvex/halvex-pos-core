use crate::tables::{Table, TableStatus};

/// Returns true if the table is available for a new session.
pub fn table_is_available(table: &Table) -> bool {
    table.status == TableStatus::Available && table.active_order_id.is_none()
}
