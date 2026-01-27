use crate::engine::{handle_table_command, Apply};
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{AreaID, OrderID, TableID, VenueID};

#[test]
fn can_create_assign_unassign_and_close_table() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let area_id = Some(AreaID::new());

    // Create
    let ev = handle_table_command(
        None,
        TableCommand::CreateTable {
            venue_id,
            table_id,
            label: "T1".into(),
            area_id,
        },
    )
    .unwrap();

    let mut table = Table::new(venue_id, table_id, "T1", area_id);
    for e in &ev {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Available);

    // assign order
    let order_id = OrderID::new();
    let ev = handle_table_command(
        Some(&table),
        TableCommand::AssignOrder {
            venue_id,
            table_id,
            order_id,
        },
    )
    .unwrap();
    for e in &ev {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Occupied);
    assert_eq!(table.active_order_id, Some(order_id));

    // Unassign
    let ev = handle_table_command(
        Some(&table),
        TableCommand::UnassignOrder {
            venue_id,
            table_id,
            reason: "paid".into(),
        },
    )
    .unwrap();
    for e in &ev {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Available);
    assert_eq!(table.active_order_id, None);

    // Close
    let ev = handle_table_command(
        Some(&table),
        TableCommand::CloseTable { venue_id, table_id },
    )
    .unwrap();
    for e in &ev {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Closed);
}
