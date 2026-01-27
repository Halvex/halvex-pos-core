use crate::checks::{Check, CheckCommand, CheckEvent, TenderType};
use crate::core::{
    route_command_envelope_enveloped, CommandEnvelope, CoreCommand, CoreContext, CoreEvent,
    EventEnvelope,
};
use crate::engine::{handle_check_command, handle_order_command, handle_table_command, Apply};
use crate::orders::{Order, OrderCommand, OrderEvent};
use crate::tables::{Table, TableCommand, TableEvent};
use pos_types::{ActorID, CheckID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID};
use rust_decimal::Decimal;

#[test]
fn actor_id_propagates_to_event_envelopes() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let actor_id = ActorID::new();

    let mut env = CommandEnvelope::new(CoreCommand::Order(OrderCommand::OpenOrder {
        venue_id,
        order_id,
    }));
    env.actor_id = Some(actor_id);

    let events = route_command_envelope_enveloped(CoreContext::empty(), env).unwrap();
    assert!(!events.is_empty());
    for ev in events {
        assert_eq!(ev.actor_id, Some(actor_id));
    }
}

#[test]
fn reason_fields_roundtrip_for_sensitive_actions() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();
    let table_id = TableID::new();

    let mut check = Check::new(venue_id, check_id, order_id);
    let mut order = Order::new(venue_id, order_id);
    let mut table = Table::new(venue_id, table_id, "T1", None);

    // Order item removal
    let order_item_id = OrderItemID::new();
    let order_remove_reason = "voided".to_string();
    let order_events = handle_order_command(
        Some(&order),
        OrderCommand::RemoveItem {
            venue_id,
            order_id,
            order_item_id,
            reason: order_remove_reason.clone(),
        },
    )
    .unwrap();
    for event in &order_events {
        order.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(
        CoreEvent::Order(order_events[0].clone()),
        &order_remove_reason,
    );

    // Check line removal
    let check_remove_reason = "mistake".to_string();
    let check_events = handle_check_command(
        Some(&check),
        CheckCommand::RemoveLine {
            venue_id,
            check_id,
            order_item_id: OrderItemID::new(),
            reason: check_remove_reason.clone(),
        },
    )
    .unwrap();
    for event in &check_events {
        check.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(
        CoreEvent::Check(check_events[0].clone()),
        &check_remove_reason,
    );

    // Discount reason
    let discount_reason = "manager override".to_string();
    let discount_events = handle_check_command(
        Some(&check),
        CheckCommand::ApplyDiscountPercent {
            venue_id,
            check_id,
            label: "Comp".into(),
            percent: 10,
            reason: discount_reason.clone(),
        },
    )
    .unwrap();
    for event in &discount_events {
        check.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(
        CoreEvent::Check(discount_events[0].clone()),
        &discount_reason,
    );

    // Service charge reason
    let service_reason = "service policy".to_string();
    let service_events = handle_check_command(
        Some(&check),
        CheckCommand::ApplyServiceChargePercent {
            venue_id,
            check_id,
            label: "Service".into(),
            percent: 12,
            reason: service_reason.clone(),
        },
    )
    .unwrap();
    for event in &service_events {
        check.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(CoreEvent::Check(service_events[0].clone()), &service_reason);

    // Authorise payment then void
    let payment_id = PaymentID::new();
    let auth_events = handle_check_command(
        Some(&check),
        CheckCommand::AuthorisePayment {
            venue_id,
            check_id,
            payment_id,
            tender: TenderType::Card,
            amount: Money::gbp(Decimal::new(1000, 2)),
            tip: None,
            processor: None,
        },
    )
    .unwrap();
    for event in &auth_events {
        check.apply(event).unwrap();
    }

    let void_reason = "duplicate".to_string();
    let void_events = handle_check_command(
        Some(&check),
        CheckCommand::VoidPayment {
            venue_id,
            check_id,
            payment_id,
            reason: void_reason.clone(),
        },
    )
    .unwrap();
    for event in &void_events {
        check.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(CoreEvent::Check(void_events[0].clone()), &void_reason);

    // Record payment then refund
    let payment_id = PaymentID::new();
    let record_events = handle_check_command(
        Some(&check),
        CheckCommand::RecordPayment {
            venue_id,
            check_id,
            payment_id,
            tender: TenderType::Card,
            amount: Money::gbp(Decimal::new(1500, 2)),
            tip: None,
            processor: None,
        },
    )
    .unwrap();
    for event in &record_events {
        check.apply(event).unwrap();
    }

    let refund_reason = "guest request".to_string();
    let refund_events = handle_check_command(
        Some(&check),
        CheckCommand::RecordRefund {
            venue_id,
            check_id,
            payment_id,
            amount: Money::gbp(Decimal::new(500, 2)),
            reason: refund_reason.clone(),
        },
    )
    .unwrap();
    for event in &refund_events {
        check.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(CoreEvent::Check(refund_events[0].clone()), &refund_reason);

    // Table unassign reason
    let assign_events = handle_table_command(
        Some(&table),
        TableCommand::AssignOrder {
            venue_id,
            table_id,
            order_id,
        },
    )
    .unwrap();
    for event in &assign_events {
        table.apply(event).unwrap();
    }

    let unassign_reason = "closed".to_string();
    let unassign_events = handle_table_command(
        Some(&table),
        TableCommand::UnassignOrder {
            venue_id,
            table_id,
            reason: unassign_reason.clone(),
        },
    )
    .unwrap();
    for event in &unassign_events {
        table.apply(event).unwrap();
    }
    assert_event_reason_roundtrip(
        CoreEvent::Table(unassign_events[0].clone()),
        &unassign_reason,
    );
}

fn assert_event_reason_roundtrip(event: CoreEvent, expected: &str) {
    let env = EventEnvelope::new(event.aggregate_ref(), event.clone());
    let json = serde_json::to_string(&env).unwrap();
    let de: EventEnvelope = serde_json::from_str(&json).unwrap();

    match de.event {
        CoreEvent::Order(OrderEvent::ItemRemoved { reason, .. }) => {
            assert_eq!(reason.as_deref(), Some(expected));
        }
        CoreEvent::Check(CheckEvent::LineRemoved { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        CoreEvent::Check(CheckEvent::DiscountPercentApplied { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        CoreEvent::Check(CheckEvent::ServiceChargeApplied { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        CoreEvent::Check(CheckEvent::PaymentVoided { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        CoreEvent::Check(CheckEvent::RefundRecorded { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        CoreEvent::Table(TableEvent::OrderUnassingedFromTable { reason, .. }) => {
            assert_eq!(reason, expected);
        }
        other => panic!("unexpected event for reason roundtrip: {:?}", other),
    }
}
