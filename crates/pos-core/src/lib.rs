mod checks;
mod core;
mod engine;
mod kds;
mod orders;
mod projections;
mod tables;

pub mod prelude;

pub use pos_types::PosError;

pub use core::envelope::{
    AggregateRef, AggregateType, CommandEnvelope, CoreCommand, CoreContext, CoreEvent,
    DedupeDecision, EventEnvelope, IdempotencyKey, WorkflowCommand,
};

pub use core::router::{
    route_command_envelope_enveloped, route_core_command, route_core_command_enveloped,
};

pub use core::snapshot::{
    rebuild_check_from_events, rebuild_check_from_snapshot_and_events, rebuild_kds_from_events,
    rebuild_order_from_events, rebuild_order_from_snapshot_and_events, rebuild_table_from_events,
    rebuild_table_from_snapshot_and_events, snapshot_check, snapshot_order, snapshot_table,
    Snapshot,
};
pub use core::storage_contract::{EventStore, ExpectedVersion};
pub use core::versioning::{
    ensure_supported_schema, MAX_SUPPORTED_SCHEMA_VERSION, MIN_SUPPORTED_SCHEMA_VERSION,
    SCHEMA_VERSION,
};

pub use orders::{Order, OrderCommand, OrderEvent, OrderItem, OrderStatus};

pub use checks::{
    totals, Check, CheckCommand, CheckEvent, CheckLine, CheckStatus, CheckTotals, Discount,
    Payment, PaymentStatus, ProcessorRef, ServiceCharge, TenderType,
};

pub use tables::{Table, TableCommand, TableEvent, TableStatus};

pub use projections::{can_close_check, check_balance_due, table_is_available};

pub use kds::{
    apply_core_event_to_kds, apply_kds_event, apply_order_event, delta_tickets_since, expo_view,
    kitchen_tickets, printable_lines_from_ticket, station_for_item, ticket_has_active_lines,
    ticket_is_ready, tickets_for_printing, KdsChange, KdsChangeKind, KdsCursorStore, KdsDeltaKind,
    KdsDeltaLine, KdsDeltaTicket, KdsEvent, KdsLine, KdsLineStatus, KdsPrintCursor, KdsState,
    KdsStateStore, KdsTicket, KdsTicketKey, KdsTicketStatus, PrintableLine, PrintableTicket,
    RoutingRule, RoutingTable, StationId,
};

#[cfg(test)]
mod tests;
