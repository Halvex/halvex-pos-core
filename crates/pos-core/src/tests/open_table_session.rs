use crate::core::{route_core_command, CoreCommand, CoreContext, CoreEvent, WorkflowCommand};
use crate::engine::Apply;
use crate::tables::{Table, TableCommand, TableStatus};
use pos_types::{CheckID, OrderID, TableID, VenueID};

#[test]
fn open_table_session_emits_order_check_and_table_events() {
    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    // Create a table first
    let mut table = Table::new(venue_id, table_id, "T1", None);
    let created = crate::engine::handle_table_command(
        None,
        TableCommand::CreateTable {
            venue_id,
            table_id,
            label: "T1".into(),
            area_id: None,
        },
    )
    .unwrap();
    for e in &created {
        table.apply(e).unwrap();
    }
    assert_eq!(table.status, TableStatus::Available);
    assert!(table.active_order_id.is_none());

    // Run OpenTableSession workflow
    let events = route_core_command(
        CoreContext {
            table: Some(&table),
            order: None,
            check: None,
            to_check: None,
        },
        CoreCommand::Workflow(WorkflowCommand::OpenTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
        }),
    )
    .unwrap();

    // Validate the 3 events exist
    assert!(events.iter().any(|e| matches!(e, CoreEvent::Order(_))));
    assert!(events.iter().any(|e| matches!(e, CoreEvent::Check(_))));
    assert!(events.iter().any(|e| matches!(e, CoreEvent::Table(_))));

    // Apply only the table event to table, to ensure it becomes occupied
    for e in events {
        if let CoreEvent::Table(te) = e {
            table.apply(&te).unwrap();
        }
    }

    assert_eq!(table.status, TableStatus::Occupied);
    assert_eq!(table.active_order_id, Some(order_id));
}
