use super::versioning::{ensure_supported_schema, SCHEMA_VERSION};
use crate::{
    checks::{Check, CheckCommand, CheckEvent},
    orders::{Order, OrderCommand, OrderEvent},
    tables::{Table, TableCommand, TableEvent},
};
use pos_types::PosError;
use pos_types::{ActorID, CheckID, OrderID, OrderItemID, TableID, VenueID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_schema_version() -> u16 {
    SCHEMA_VERSION
}

/// Client-provided retry key for deduplication.
///
/// Serde-transparent so it serializes as a plain string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

impl From<String> for IdempotencyKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for IdempotencyKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// A unified command envelope for clients (TS/Swift/Kotlin).
// Serde-tagged so it serializes cleanly as JSON.

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CoreCommand {
    Order(OrderCommand),
    Check(CheckCommand),
    Table(TableCommand),
    Workflow(WorkflowCommand),
}

// Workflow commands are cross-aggregate actions.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum WorkflowCommand {
    SplitCheckCreate {
        venue_id: VenueID,
        from_check_id: CheckID,
        new_check_id: CheckID,
    },

    SplitCheckMoveLineQty {
        venue_id: VenueID,
        from_check_id: CheckID,
        to_check_id: CheckID,
        order_item_id: OrderItemID,
        qty: u32,
    },

    OpenTableSession {
        venue_id: VenueID,
        table_id: pos_types::TableID,
        order_id: pos_types::OrderID,
        check_id: pos_types::CheckID,
    },

    CloseTableSession {
        venue_id: VenueID,
        table_id: TableID,
        order_id: OrderID,
        check_id: CheckID,
        reason: String,
    },
}

// A unified event envelope for clients (TS/Swift/Kotlin).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CoreEvent {
    Order(OrderEvent),
    Check(CheckEvent),
    Table(TableEvent),
}

// Context provided by the host (server/app) when routing commands.
// The host loads whatever aggregates it has, then calls `route_core_command`.
pub struct CoreContext<'a> {
    pub order: Option<&'a Order>,
    pub check: Option<&'a Check>,
    pub table: Option<&'a Table>,

    // Used for cross-check workflows (splits)
    pub to_check: Option<&'a Check>,
}

impl<'a> CoreContext<'a> {
    pub fn empty() -> Self {
        Self {
            order: None,
            check: None,
            table: None,
            to_check: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum AggregateType {
    Order,
    Check,
    Table,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregateRef {
    pub ty: AggregateType,
    pub id: Uuid,
}

impl AggregateRef {
    pub fn order(id: OrderID) -> Self {
        Self {
            ty: AggregateType::Order,
            id: id.0,
        }
    }

    pub fn check(id: CheckID) -> Self {
        Self {
            ty: AggregateType::Check,
            id: id.0,
        }
    }

    pub fn table(id: TableID) -> Self {
        Self {
            ty: AggregateType::Table,
            id: id.0,
        }
    }
}

// Stable event wrapper for transport/sync.
// `seq` is the *per-aggregate* version after applying this event:
// - It MUST reflect the aggregate version *after* applying the event.
// - It is NOT a global stream position.
// - The host/storage layer assigns it when appending to the stream.
//   If current_version is 7 and two events are appended, they receive seq 8 and 9.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    pub event_id: Uuid,
    pub aggregate: AggregateRef,
    pub seq: Option<u64>,

    #[serde(default)]
    pub actor_id: Option<ActorID>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,

    pub event: CoreEvent,
}

impl EventEnvelope {
    pub fn new(aggregate: AggregateRef, event: CoreEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            event_id: Uuid::new_v4(),
            aggregate,
            seq: None,
            actor_id: None,
            causation_id: None,
            correlation_id: None,
            event,
        }
    }

    /// Validate that this envelope's schema_version is supported by this build.
    pub fn ensure_supported_schema(&self) -> Result<(), PosError> {
        ensure_supported_schema(self.schema_version)
    }
}

impl CoreEvent {
    pub fn aggregate_ref(&self) -> AggregateRef {
        match self {
            CoreEvent::Order(e) => match e {
                crate::orders::OrderEvent::OrderOpened { order_id, .. }
                | crate::orders::OrderEvent::ItemAdded { order_id, .. }
                | crate::orders::OrderEvent::ItemRemoved { order_id, .. }
                | crate::orders::OrderEvent::OrderClosed { order_id, .. } => {
                    AggregateRef::order(*order_id)
                }
            },

            CoreEvent::Check(e) => match e {
                crate::checks::CheckEvent::CheckOpened { check_id, .. }
                | crate::checks::CheckEvent::LineAdded { check_id, .. }
                | crate::checks::CheckEvent::LineRemoved { check_id, .. }
                | crate::checks::CheckEvent::DiscountPercentApplied { check_id, .. }
                | crate::checks::CheckEvent::ServiceChargeApplied { check_id, .. }
                | crate::checks::CheckEvent::PaymentRecorded { check_id, .. }
                | crate::checks::CheckEvent::PaymentVoided { check_id, .. }
                | crate::checks::CheckEvent::RefundRecorded { check_id, .. }
                | crate::checks::CheckEvent::SplitCheckCreated { check_id, .. }
                | crate::checks::CheckEvent::LineQtyDecreased { check_id, .. }
                | crate::checks::CheckEvent::CheckClosed { check_id, .. }
                | crate::checks::CheckEvent::PaymentAuthorised { check_id, .. }
                | crate::checks::CheckEvent::PaymentCaptured { check_id, .. } => {
                    AggregateRef::check(*check_id)
                }
            },

            CoreEvent::Table(e) => match e {
                crate::tables::TableEvent::TableCreated { table_id, .. }
                | crate::tables::TableEvent::OrderAssignedToTable { table_id, .. }
                | crate::tables::TableEvent::OrderUnassingedFromTable { table_id, .. }
                | crate::tables::TableEvent::TableClosed { table_id, .. }
                | crate::tables::TableEvent::TableReopened { table_id, .. } => {
                    AggregateRef::table(*table_id)
                }
            },
        }
    }
}

/// A unified command wrapper for transport/sync.
///
/// Idempotency contract (host-facing):
/// - If `idempotency_key` is set, the host/storage layer MUST ensure that the
///   same `(aggregate_ref, idempotency_key)` returns the same event list.
/// - Recommended `aggregate_ref`:
///   - Order command → order aggregate
///   - Check command → check aggregate
///   - Table command → table aggregate
///   - Workflow command → choose a stable primary aggregate (e.g. `from_check_id`
///     for splits, or `table_id` for table sessions).
///
/// The core does not enforce idempotency; it only carries the key.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    pub command_id: Uuid,

    // Client-provided retry key (same logical command sent again).
    // The host/storage layer should dedupe using this.
    pub idempotency_key: Option<IdempotencyKey>,

    // Trace a request across many commands/events
    pub correlation_id: Option<Uuid>,

    // Optional link to a prior event/command that caused this
    pub causation_id: Option<Uuid>,

    #[serde(default)]
    pub actor_id: Option<ActorID>,

    pub command: CoreCommand,
}

impl CommandEnvelope {
    pub fn new(command: CoreCommand) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            command_id: Uuid::new_v4(),
            idempotency_key: None,
            correlation_id: None,
            causation_id: None,
            actor_id: None,
            command,
        }
    }

    /// Validate that this envelope's schema_version is supported by this build.
    pub fn ensure_supported_schema(&self) -> Result<(), PosError> {
        ensure_supported_schema(self.schema_version)
    }
}

/// Outcome of a host-side idempotency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum DedupeDecision {
    /// Treat as a new command and generate events.
    New,
    /// Replay previously stored events for this idempotency key.
    Replay(Vec<EventEnvelope>),
}
