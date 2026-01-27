use super::{ProcessorRef, TenderType};
use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CheckEvent {
    CheckOpened {
        venue_id: VenueID,
        check_id: CheckID,
        order_id: OrderID,
    },

    LineAdded {
        venue_id: VenueID,
        check_id: CheckID,
        order_item_id: OrderItemID,
        qty: u32,
        unit_price: Money,
        name: String,
    },

    LineRemoved {
        venue_id: VenueID,
        check_id: CheckID,
        order_item_id: OrderItemID,
        reason: String,
    },

    DiscountPercentApplied {
        venue_id: VenueID,
        check_id: CheckID,
        label: String,
        percent: u8,
        reason: String,
    },

    ServiceChargeApplied {
        venue_id: VenueID,
        check_id: CheckID,
        label: String,
        percent: u8,
        reason: String,
    },

    CheckClosed {
        venue_id: VenueID,
        check_id: CheckID,
    },

    PaymentRecorded {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        tender: TenderType,
        amount: Money,
        tip: Option<Money>,
        processor: Option<ProcessorRef>,
    },

    PaymentVoided {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        reason: String,
    },

    RefundRecorded {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        amount: Money,
        reason: String,
    },

    SplitCheckCreated {
        venue_id: VenueID,
        check_id: CheckID,
        from_check_id: CheckID,
    },

    LineQtyDecreased {
        venue_id: VenueID,
        check_id: CheckID,
        order_item_id: OrderItemID,
        qty: u32,
        reason: String,
    },

    PaymentAuthorised {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        tender: TenderType,
        amount: Money,
        tip: Option<Money>,
        processor: Option<ProcessorRef>,
    },

    PaymentCaptured {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        amount: Money,
    },
}
