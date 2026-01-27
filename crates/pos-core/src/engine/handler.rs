use crate::checks::{
    ensure_check_open, ensure_currency_matches, ensure_payment_can_capture,
    ensure_payment_can_refund, ensure_payment_can_void, ensure_payment_exists,
    ensure_percent_valid, ensure_qty_positive as ensure_line_qty, totals, Check, CheckCommand,
    CheckEvent, CheckStatus,
};

use crate::orders::{
    ensure_order_open, ensure_qty_positive, Order, OrderCommand, OrderEvent, OrderStatus,
};
use crate::tables::{
    ensure_table_has_active_order, ensure_table_has_no_active_order, ensure_table_not_closed,
    Table, TableCommand, TableEvent, TableStatus,
};
use pos_types::PosError;

pub fn handle_order_command(
    current: Option<&Order>,
    cmd: OrderCommand,
) -> Result<Vec<OrderEvent>, PosError> {
    match cmd {
        OrderCommand::OpenOrder { venue_id, order_id } => {
            if let Some(o) = current {
                if o.status != OrderStatus::Cancelled {
                    return Err(PosError::Conflict("order already exists".into()));
                }
            }
            Ok(vec![OrderEvent::OrderOpened { venue_id, order_id }])
        }

        OrderCommand::AddItem {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name,
            unit_price,
            qty,
            notes,
        } => {
            let o = current.ok_or_else(|| PosError::NotFound("order not found".into()))?;
            ensure_order_open(o)?;
            ensure_qty_positive(qty)?;

            Ok(vec![OrderEvent::ItemAdded {
                venue_id,
                order_id,
                order_item_id,
                menu_item_id,
                name,
                unit_price,
                qty,
                notes,
            }])
        }

        OrderCommand::RemoveItem {
            venue_id,
            order_id,
            order_item_id,
            reason,
        } => {
            let o = current.ok_or_else(|| PosError::NotFound("order not found".into()))?;
            ensure_order_open(o)?;

            Ok(vec![OrderEvent::ItemRemoved {
                venue_id,
                order_id,
                order_item_id,
                reason: Some(reason),
            }])
        }

        OrderCommand::CloseOrder { venue_id, order_id } => {
            let o = current.ok_or_else(|| PosError::NotFound("order not found".into()))?;
            ensure_order_open(o)?;

            if o.items.is_empty() {
                return Err(PosError::validation("cannot close an empty order"));
            }

            Ok(vec![OrderEvent::OrderClosed { venue_id, order_id }])
        }
    }
}

pub fn handle_check_command(
    current: Option<&Check>,
    cmd: CheckCommand,
) -> Result<Vec<CheckEvent>, PosError> {
    match cmd {
        CheckCommand::OpenCheck {
            venue_id,
            check_id,
            order_id,
        } => {
            if let Some(c) = current {
                if c.status != CheckStatus::Voided {
                    return Err(PosError::Conflict("check already exists".into()));
                }
            }

            Ok(vec![CheckEvent::CheckOpened {
                venue_id,
                check_id,
                order_id,
            }])
        }

        CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id,
            qty,
            unit_price,
            name,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_line_qty(qty)?;
            Ok(vec![CheckEvent::LineAdded {
                venue_id,
                check_id,
                order_item_id,
                qty,
                unit_price,
                name,
            }])
        }

        CheckCommand::RemoveLine {
            venue_id,
            check_id,
            order_item_id,
            reason,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            Ok(vec![CheckEvent::LineRemoved {
                venue_id,
                check_id,
                order_item_id,
                reason,
            }])
        }

        CheckCommand::ApplyDiscountPercent {
            venue_id,
            check_id,
            label,
            percent,
            reason,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            Ok(vec![CheckEvent::DiscountPercentApplied {
                venue_id,
                check_id,
                label,
                percent,
                reason,
            }])
        }

        CheckCommand::ApplyServiceChargePercent {
            venue_id,
            check_id,
            label,
            percent,
            reason,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_percent_valid(percent)?;
            Ok(vec![CheckEvent::ServiceChargeApplied {
                venue_id,
                check_id,
                label,
                percent,
                reason,
            }])
        }

        CheckCommand::CloseCheck { venue_id, check_id } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            if c.lines.is_empty() {
                return Err(PosError::validation("cannot close an empty check"));
            }

            let t = totals(c);
            if t.balance_due.amount > rust_decimal::Decimal::ZERO {
                return Err(PosError::validation("cannot close unpaid check"));
            }

            Ok(vec![CheckEvent::CheckClosed { venue_id, check_id }])
        }

        CheckCommand::RecordPayment {
            venue_id,
            check_id,
            payment_id,
            tender,
            amount,
            tip,
            processor,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_currency_matches(c, &amount)?;
            if let Some(t) = &tip {
                ensure_currency_matches(c, t)?;
            }

            if c.payments.iter().any(|p| p.payment_id == payment_id) {
                return Err(PosError::Conflict("payment_id already exists".into()));
            }

            // RecordPayment is a convenience that emits Authorised + Captured.
            Ok(vec![
                CheckEvent::PaymentAuthorised {
                    venue_id,
                    check_id,
                    payment_id,
                    tender: tender.clone(),
                    amount,
                    tip,
                    processor,
                },
                CheckEvent::PaymentCaptured {
                    venue_id,
                    check_id,
                    payment_id,
                    amount,
                },
            ])
        }

        CheckCommand::VoidPayment {
            venue_id,
            check_id,
            payment_id,
            reason,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_payment_exists(c, payment_id)?;
            ensure_payment_can_void(c, payment_id)?;

            Ok(vec![CheckEvent::PaymentVoided {
                venue_id,
                check_id,
                payment_id,
                reason,
            }])
        }

        CheckCommand::RecordRefund {
            venue_id,
            check_id,
            payment_id,
            amount,
            reason,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_currency_matches(c, &amount)?;
            ensure_payment_exists(c, payment_id)?;
            ensure_payment_can_refund(c, payment_id, &amount)?;

            Ok(vec![CheckEvent::RefundRecorded {
                venue_id,
                check_id,
                payment_id,
                amount,
                reason,
            }])
        }

        CheckCommand::SplitChecksCreate { .. } => Err(PosError::validation(
            "SplitChecksCreate must be handled by workflow (requires both checks)",
        )),

        CheckCommand::SplitCheckMoveLineQty { .. } => Err(PosError::validation(
            "SplitCheckMoveLineQty must be handled by workflow (requires both checks)",
        )),

        CheckCommand::AuthorisePayment {
            venue_id,
            check_id,
            payment_id,
            tender,
            amount,
            tip,
            processor,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_currency_matches(c, &amount)?;
            if let Some(t) = &tip {
                ensure_currency_matches(c, t)?;
            }

            // prevent duplicates
            if c.payments.iter().any(|p| p.payment_id == payment_id) {
                return Err(PosError::Conflict("payment_id already exists".into()));
            }

            Ok(vec![CheckEvent::PaymentAuthorised {
                venue_id,
                check_id,
                payment_id,
                tender,
                amount,
                tip,
                processor,
            }])
        }

        CheckCommand::CapturePayment {
            venue_id,
            check_id,
            payment_id,
            amount,
        } => {
            let c = current.ok_or_else(|| PosError::NotFound("check not found".into()))?;
            ensure_check_open(c)?;
            ensure_currency_matches(c, &amount)?;
            ensure_payment_exists(c, payment_id)?;
            ensure_payment_can_capture(c, payment_id, &amount)?;

            Ok(vec![CheckEvent::PaymentCaptured {
                venue_id,
                check_id,
                payment_id,
                amount,
            }])
        }
    }
}

pub fn handle_table_command(
    current: Option<&Table>,
    cmd: TableCommand,
) -> Result<Vec<TableEvent>, PosError> {
    match cmd {
        TableCommand::CreateTable {
            venue_id,
            table_id,
            label,
            area_id,
        } => {
            if current.is_some() {
                return Err(PosError::Conflict("table already exists".into()));
            }
            Ok(vec![TableEvent::TableCreated {
                venue_id,
                table_id,
                label,
                area_id,
            }])
        }

        TableCommand::AssignOrder {
            venue_id,
            table_id,
            order_id,
        } => {
            let t = current.ok_or_else(|| PosError::NotFound("table not found".into()))?;
            ensure_table_not_closed(t)?;
            ensure_table_has_no_active_order(t)?;
            Ok(vec![TableEvent::OrderAssignedToTable {
                venue_id,
                table_id,
                order_id,
            }])
        }

        TableCommand::UnassignOrder {
            venue_id,
            table_id,
            reason,
        } => {
            let t = current.ok_or_else(|| PosError::NotFound("table not found".into()))?;
            ensure_table_not_closed(t)?;
            ensure_table_has_active_order(t)?;
            Ok(vec![TableEvent::OrderUnassingedFromTable {
                venue_id,
                table_id,
                reason,
            }])
        }

        TableCommand::CloseTable { venue_id, table_id } => {
            let t = current.ok_or_else(|| PosError::NotFound("table not found".into()))?;
            if t.status == TableStatus::Closed {
                return Err(PosError::Conflict("table already closed".into()));
            }
            if t.active_order_id.is_some() {
                return Err(PosError::validation("cannot close table with active order"));
            }
            Ok(vec![TableEvent::TableClosed { venue_id, table_id }])
        }

        TableCommand::ReopenTable { venue_id, table_id } => {
            let t = current.ok_or_else(|| PosError::NotFound("table not found".into()))?;
            if t.status != TableStatus::Closed {
                return Err(PosError::Conflict("table is not closed".into()));
            }
            Ok(vec![TableEvent::TableReopened { venue_id, table_id }])
        }
    }
}
