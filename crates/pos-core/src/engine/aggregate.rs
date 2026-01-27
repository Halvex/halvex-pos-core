use crate::checks::{Check, CheckEvent, CheckLine, CheckStatus, Discount, ServiceCharge};
use crate::orders::{Order, OrderEvent, OrderItem, OrderStatus};
use crate::tables::{Table, TableEvent, TableStatus};
use pos_types::PosError;

#[allow(dead_code)]
pub trait Apply<E> {
    fn apply(&mut self, event: &E) -> Result<(), PosError>;
}

impl Apply<OrderEvent> for Order {
    fn apply(&mut self, event: &OrderEvent) -> Result<(), PosError> {
        match event {
            OrderEvent::OrderOpened { venue_id, order_id } => {
                self.venue_id = *venue_id;
                self.order_id = *order_id;
                self.status = OrderStatus::Open;
            }
            OrderEvent::ItemAdded {
                order_item_id,
                menu_item_id,
                name,
                unit_price,
                qty,
                notes,
                ..
            } => {
                self.items.push(OrderItem {
                    item_id: *order_item_id,
                    menu_item_id: *menu_item_id,
                    name: name.clone(),
                    unit_price: *unit_price,
                    qty: *qty,
                    notes: notes.clone(),
                });
            }
            OrderEvent::ItemRemoved { order_item_id, .. } => {
                self.items.retain(|i| i.item_id != *order_item_id);
            }
            OrderEvent::OrderClosed { .. } => {
                self.status = OrderStatus::Closed;
            }
        }

        self.version += 1;
        Ok(())
    }
}

impl Apply<CheckEvent> for Check {
    fn apply(&mut self, event: &CheckEvent) -> Result<(), PosError> {
        match event {
            CheckEvent::CheckOpened {
                venue_id,
                check_id,
                order_id,
            } => {
                self.venue_id = *venue_id;
                self.check_id = *check_id;
                self.order_id = *order_id;
                self.status = CheckStatus::Open;
            }
            CheckEvent::LineAdded {
                order_item_id,
                qty,
                unit_price,
                name,
                ..
            } => {
                // If the item already exists on the check,
                // just increase qty (simple behavior)
                if let Some(existing) = self
                    .lines
                    .iter_mut()
                    .find(|l| l.order_item_id == *order_item_id)
                {
                    existing.qty += *qty;
                } else {
                    self.lines.push(CheckLine {
                        order_item_id: *order_item_id,
                        qty: *qty,
                        unit_price: *unit_price,
                        name: name.clone(),
                    });
                }
            }
            CheckEvent::LineRemoved { order_item_id, .. } => {
                self.lines.retain(|l| l.order_item_id != *order_item_id);
            }
            CheckEvent::DiscountPercentApplied { label, percent, .. } => {
                self.discount = Some(Discount::Percent {
                    label: label.clone(),
                    percent: *percent,
                });
            }
            CheckEvent::ServiceChargeApplied { label, percent, .. } => {
                self.service_charge = Some(ServiceCharge::Percent {
                    label: label.clone(),
                    percent: *percent,
                });
            }
            CheckEvent::CheckClosed { .. } => {
                self.status = CheckStatus::Closed;
            }
            CheckEvent::PaymentRecorded {
                payment_id,
                tender,
                amount,
                tip,
                processor,
                ..
            } => {
                self.payments.push(crate::checks::Payment {
                    payment_id: *payment_id,
                    tender: tender.clone(),
                    authorised: *amount,
                    captured: *amount,
                    refunded: pos_types::Money {
                        amount: rust_decimal::Decimal::ZERO,
                        currency: amount.currency,
                    },
                    tip: *tip,
                    status: crate::checks::PaymentStatus::Captured,
                    processor: processor.clone(),
                });
            }

            CheckEvent::PaymentAuthorised {
                payment_id,
                tender,
                amount,
                tip,
                processor,
                ..
            } => {
                self.payments.push(crate::checks::Payment {
                    payment_id: *payment_id,
                    tender: tender.clone(),
                    authorised: *amount,
                    captured: pos_types::Money {
                        amount: rust_decimal::Decimal::ZERO,
                        currency: amount.currency,
                    },
                    refunded: pos_types::Money {
                        amount: rust_decimal::Decimal::ZERO,
                        currency: amount.currency,
                    },
                    tip: *tip,
                    status: crate::checks::PaymentStatus::Authorized,
                    processor: processor.clone(),
                });
            }

            CheckEvent::PaymentCaptured {
                payment_id, amount, ..
            } => {
                if let Some(p) = self
                    .payments
                    .iter_mut()
                    .find(|p| p.payment_id == *payment_id)
                {
                    p.captured.amount += amount.amount;
                    p.status = crate::checks::PaymentStatus::Captured;
                }
            }

            CheckEvent::PaymentVoided { payment_id, .. } => {
                if let Some(p) = self
                    .payments
                    .iter_mut()
                    .find(|p| p.payment_id == *payment_id)
                {
                    p.status = crate::checks::PaymentStatus::Voided;
                }
            }

            CheckEvent::RefundRecorded {
                payment_id, amount, ..
            } => {
                if let Some(p) = self
                    .payments
                    .iter_mut()
                    .find(|p| p.payment_id == *payment_id)
                {
                    p.refunded.amount += amount.amount;
                    if p.refunded.amount >= p.captured.amount {
                        p.status = crate::checks::PaymentStatus::Refunded;
                    }
                }
            }

            CheckEvent::SplitCheckCreated {
                venue_id: _,
                check_id: _,
                from_check_id: _,
            } => {
                // No state change needed (metadata/audit event)
            }

            CheckEvent::LineQtyDecreased {
                venue_id: _,
                check_id: _,
                order_item_id,
                qty,
                reason: _,
            } => {
                if let Some(line) = self
                    .lines
                    .iter_mut()
                    .find(|l| l.order_item_id == *order_item_id)
                {
                    if line.qty <= *qty {
                        // Remove if qty becomes 0 or negative
                        self.lines.retain(|l| l.order_item_id != *order_item_id);
                    } else {
                        line.qty -= *qty;
                    }
                }
            }
        }

        self.version += 1;
        Ok(())
    }
}

impl Apply<TableEvent> for Table {
    fn apply(&mut self, event: &TableEvent) -> Result<(), PosError> {
        match event {
            TableEvent::TableCreated {
                venue_id,
                table_id,
                label,
                area_id,
            } => {
                self.venue_id = *venue_id;
                self.table_id = *table_id;
                self.label = label.clone();
                self.area_id = *area_id;
                self.status = TableStatus::Available;
                self.active_order_id = None;
            }

            TableEvent::OrderAssignedToTable { order_id, .. } => {
                self.active_order_id = Some(*order_id);
                self.status = TableStatus::Occupied;
            }

            TableEvent::OrderUnassingedFromTable { .. } => {
                self.active_order_id = None;
                self.status = TableStatus::Available;
            }

            TableEvent::TableClosed { .. } => {
                self.status = TableStatus::Closed;
            }

            TableEvent::TableReopened { .. } => {
                // Reopening makes it available unless an order is
                // re-assinged by command

                self.status = TableStatus::Available;
            }
        }

        self.version += 1;
        Ok(())
    }
}
