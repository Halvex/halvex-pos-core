use super::{CommandEnvelope, CoreCommand, CoreContext, CoreEvent, EventEnvelope, WorkflowCommand};
use crate::{
    checks::CheckEvent,
    engine::{
        handle_check_command, handle_order_command, handle_table_command, split_check_create,
        split_check_move_line_qty,
    },
};
use pos_types::PosError;
use uuid::Uuid;

pub fn route_core_command(
    ctx: CoreContext<'_>,
    cmd: CoreCommand,
) -> Result<Vec<CoreEvent>, PosError> {
    match cmd {
        CoreCommand::Order(c) => {
            let evs = handle_order_command(ctx.order, c)?;
            Ok(evs.into_iter().map(CoreEvent::Order).collect())
        }

        CoreCommand::Check(c) => {
            let evs = handle_check_command(ctx.check, c)?;
            Ok(evs.into_iter().map(CoreEvent::Check).collect())
        }

        CoreCommand::Table(c) => {
            let evs = handle_table_command(ctx.table, c)?;
            Ok(evs.into_iter().map(CoreEvent::Table).collect())
        }

        CoreCommand::Workflow(w) => route_workflow(ctx, w),
    }
}

fn route_workflow(ctx: CoreContext<'_>, cmd: WorkflowCommand) -> Result<Vec<CoreEvent>, PosError> {
    match cmd {
        WorkflowCommand::SplitCheckCreate {
            venue_id,
            from_check_id,
            new_check_id,
        } => {
            let from = ctx.check.ok_or_else(|| {
                PosError::NotFound("from_check not provided in CoreContext".into())
            })?;
            if from.check_id != from_check_id {
                return Err(PosError::validation(
                    "from_check_id does not match provided check aggregate",
                ));
            }

            let evs = split_check_create(from, venue_id, new_check_id)?;
            Ok(evs.into_iter().map(CoreEvent::Check).collect())
        }

        WorkflowCommand::SplitCheckMoveLineQty {
            venue_id,
            from_check_id,
            to_check_id,
            order_item_id,
            qty,
        } => {
            let from = ctx.check.ok_or_else(|| {
                PosError::NotFound("from_check not provided in CoreContext".into())
            })?;
            let to = ctx
                .to_check
                .ok_or_else(|| PosError::NotFound("to_check not provided in CoreContext".into()))?;

            if from.check_id != from_check_id {
                return Err(PosError::validation(
                    "from_check_id does not match provided check aggregate",
                ));
            }

            if to.check_id != to_check_id {
                return Err(PosError::validation(
                    "to_check_id does not match provided check aggregate",
                ));
            }

            let evs: Vec<CheckEvent> =
                split_check_move_line_qty(from, to, venue_id, order_item_id, qty)?;
            Ok(evs.into_iter().map(CoreEvent::Check).collect())
        }

        WorkflowCommand::OpenTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
        } => {
            let table = ctx
                .table
                .ok_or_else(|| PosError::NotFound("table not provided in CoreContext".into()))?;

            if table.table_id != table_id {
                return Err(PosError::validation(
                    "table_id does not match provided table aggregate",
                ));
            }

            if table.status == crate::tables::TableStatus::Closed {
                return Err(PosError::validation("table is closed"));
            }

            if table.active_order_id.is_some() {
                return Err(PosError::validation("table already has an active order"));
            }

            if let Some(o) = ctx.order {
                if o.order_id == order_id {
                    return Err(PosError::Conflict("order_id already exists".into()));
                }
            }
            if let Some(c) = ctx.check {
                if c.check_id == check_id {
                    return Err(PosError::Conflict("check_id already exists".into()));
                }
            }

            Ok(vec![
                CoreEvent::Order(crate::orders::OrderEvent::OrderOpened { venue_id, order_id }),
                CoreEvent::Check(crate::checks::CheckEvent::CheckOpened {
                    venue_id,
                    check_id,
                    order_id,
                }),
                CoreEvent::Table(crate::tables::TableEvent::OrderAssignedToTable {
                    venue_id,
                    table_id,
                    order_id,
                }),
            ])
        }

        WorkflowCommand::CloseTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
            reason,
        } => {
            let table = ctx
                .table
                .ok_or_else(|| PosError::NotFound("table not provided in CoreContext".into()))?;
            let check = ctx
                .check
                .ok_or_else(|| PosError::NotFound("check not provided in CoreContext".into()))?;

            if table.table_id != table_id {
                return Err(PosError::validation(
                    "table_id does not match provided table aggregate",
                ));
            }
            if check.check_id != check_id {
                return Err(PosError::validation(
                    "check_id does not match provided check aggregate",
                ));
            }

            if table.active_order_id != Some(order_id) {
                return Err(PosError::validation(
                    "table active_order_id does not match order_id",
                ));
            }

            if check.order_id != order_id {
                return Err(PosError::validation("check does not belong to order_id"));
            }

            // Check must be paid ( Core Rule )
            let t = crate::checks::totals(check);
            if t.balance_due.amount > rust_decimal::Decimal::ZERO {
                return Err(PosError::validation(
                    "cannot close table session with unpaid check",
                ));
            }

            // Emit events:
            // - close check (if not already closed; we can safely emit and apply rules decide)
            // - unassign order from table (sets table available in apply)
            let mut out = Vec::new();

            if check.status == crate::checks::CheckStatus::Open {
                out.push(CoreEvent::Check(crate::checks::CheckEvent::CheckClosed {
                    venue_id,
                    check_id,
                }));
            }

            out.push(CoreEvent::Table(
                crate::tables::TableEvent::OrderUnassingedFromTable {
                    venue_id,
                    table_id,
                    reason,
                },
            ));

            Ok(out)
        }
    }
}

/// Route a `CommandEnvelope` and return `EventEnvelope`s with tracing propagated.
///
/// - `correlation_id` is copied onto every emitted event envelope.
/// - `causation_id` defaults to `command_id` (falls back to `env.causation_id`).
/// - `idempotency_key` is not used inside the core; your host/storage layer should dedupe on it.
pub fn route_command_envelope_enveloped(
    ctx: CoreContext<'_>,
    env: CommandEnvelope,
) -> Result<Vec<EventEnvelope>, PosError> {
    let events = route_core_command(ctx, env.command)?;

    let mut out = Vec::with_capacity(events.len());
    for ev in events {
        let aggregate = ev.aggregate_ref();
        let mut eenv = EventEnvelope::new(aggregate, ev);

        // Propagate tracing
        eenv.correlation_id = env.correlation_id;
        eenv.causation_id = Some(env.command_id).or(env.causation_id);
        eenv.actor_id = env.actor_id;

        out.push(eenv);
    }

    Ok(out)
}

pub fn route_core_command_enveloped(
    ctx: CoreContext<'_>,
    cmd: CoreCommand,
    causation_id: Option<Uuid>,
    correlation_id: Option<Uuid>,
) -> Result<Vec<EventEnvelope>, PosError> {
    let events = route_core_command(ctx, cmd)?;

    let mut out = Vec::with_capacity(events.len());
    for ev in events {
        let aggregate = ev.aggregate_ref();
        let mut env = EventEnvelope::new(aggregate, ev);

        env.causation_id = causation_id;
        env.correlation_id = correlation_id;

        out.push(env);
    }

    Ok(out)
}
