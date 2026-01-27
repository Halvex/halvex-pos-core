use super::{ProcessorRef, TenderType};
use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CheckCommand {
    OpenCheck {
        venue_id: VenueID,
        check_id: CheckID,
        order_id: OrderID,
    },
    AddLineFromOrderItem {
        venue_id: VenueID,
        check_id: CheckID,
        order_item_id: OrderItemID,
        qty: u32,
        unit_price: Money,
        name: String,
    },
    RemoveLine {
        venue_id: VenueID,
        check_id: CheckID,
        order_item_id: OrderItemID,
        reason: String,
    },
    ApplyDiscountPercent {
        venue_id: VenueID,
        check_id: CheckID,
        label: String,
        percent: u8,
        reason: String,
    },
    ApplyServiceChargePercent {
        venue_id: VenueID,
        check_id: CheckID,
        label: String,
        percent: u8,
        reason: String,
    },
    CloseCheck {
        venue_id: VenueID,
        check_id: CheckID,
    },
    RecordPayment {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        tender: TenderType,
        amount: Money,
        tip: Option<Money>,
        processor: Option<ProcessorRef>,
    },
    VoidPayment {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        reason: String,
    },
    RecordRefund {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        amount: Money,
        reason: String,
    },
    SplitChecksCreate {
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
    AuthorisePayment {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        tender: TenderType,
        amount: Money,
        tip: Option<Money>,
        processor: Option<ProcessorRef>,
    },
    CapturePayment {
        venue_id: VenueID,
        check_id: CheckID,
        payment_id: PaymentID,
        amount: Money,
    },
}
