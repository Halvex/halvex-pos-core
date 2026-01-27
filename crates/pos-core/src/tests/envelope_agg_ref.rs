use crate::core::CoreEvent;
use crate::orders::OrderEvent;
use pos_types::{OrderID, VenueID};

#[test]
fn core_event_can_produce_aggregate_ref() {
    let venue_id = VenueID::new();
    let order_id = OrderID::new();

    let ev = CoreEvent::Order(OrderEvent::OrderOpened { venue_id, order_id });
    let agg = ev.aggregate_ref();

    assert_eq!(agg.ty as u8, crate::core::AggregateType::Order as u8);
    assert_eq!(agg.id, order_id.0);
}
