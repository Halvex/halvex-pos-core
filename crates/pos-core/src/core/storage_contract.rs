use pos_types::PosError;

use super::envelope::{AggregateRef, EventEnvelope, IdempotencyKey};

/// Expected per-aggregate version for optimistic concurrency control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedVersion {
    /// No concurrency check (accept the current head).
    Any,
    /// Expect the stream to be empty (version 0).
    NoStream,
    /// Expect the current stream version to equal this value.
    Exact(u64),
}

/// Host-facing event storage contract.
///
/// Implementations must:
/// - preserve append-only ordering per aggregate
/// - assign `EventEnvelope.seq` as the per-aggregate version *after* each event
/// - reject or retry on version conflicts using `ExpectedVersion`
///
/// Multi-aggregate workflows must be committed atomically by the host.
/// This interface does not provide cross-stream transactions; use your
/// storage system's transaction support to append all streams or roll back.
pub trait EventStore {
    /// Load the full event stream for an aggregate in order.
    fn load_stream(&self, aggregate: AggregateRef) -> Result<Vec<EventEnvelope>, PosError>;

    /// Append events to an aggregate stream with optimistic concurrency.
    ///
    /// Implementations should assign `EventEnvelope.seq` starting at
    /// `current_version + 1` and return the new stream version. If `events`
    /// is empty, return the current version without modification.
    fn append_to_stream(
        &mut self,
        aggregate: AggregateRef,
        expected: ExpectedVersion,
        events: Vec<EventEnvelope>,
    ) -> Result<u64, PosError>;

    /// Retrieve an idempotency record for `(aggregate, key)`.
    fn get_idempotency(
        &self,
        aggregate: AggregateRef,
        key: &IdempotencyKey,
    ) -> Result<Option<Vec<EventEnvelope>>, PosError>;

    /// Persist the idempotency outcome for `(aggregate, key)`.
    fn put_idempotency(
        &mut self,
        aggregate: AggregateRef,
        key: IdempotencyKey,
        events: Vec<EventEnvelope>,
    ) -> Result<(), PosError>;
}
