use pos_types::{CheckID, Money, OrderID, OrderItemID, PaymentID, VenueID};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub venue_id: VenueID,
    pub check_id: CheckID,
    pub order_id: OrderID,
    pub status: CheckStatus,

    /// Items on this check (references to order item ids + qty)
    pub lines: Vec<CheckLine>,

    pub discount: Option<Discount>,
    pub service_charge: Option<ServiceCharge>,

    pub payments: Vec<Payment>,

    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CheckStatus {
    Open,
    Closed,
    Voided,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckLine {
    pub order_item_id: OrderItemID,
    pub qty: u32,
    pub unit_price: Money,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Discount {
    Percent { label: String, percent: u8 }, // 0..=100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ServiceCharge {
    Percent { label: String, percent: u8 }, // 0..=100
}

impl Check {
    pub fn new(venue_id: VenueID, check_id: CheckID, order_id: OrderID) -> Self {
        Self {
            venue_id,
            check_id,
            order_id,
            status: CheckStatus::Open,
            lines: vec![],
            discount: None,
            service_charge: None,
            payments: vec![],
            version: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: PaymentID,
    pub tender: TenderType,

    /// Processor-style amounts:
    /// - authorised: amount approved/held
    /// - captured: amount settled (counts as paid)
    /// - refunded: amount refunded (reduces paid)
    pub authorised: Money,
    pub captured: Money,
    pub refunded: Money,

    pub tip: Option<Money>,
    pub status: PaymentStatus,

    pub processor: Option<ProcessorRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum PaymentStatus {
    Authorized,
    Captured,
    Voided,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TenderType {
    Cash,
    Card,
    Other { label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorRef {
    pub name: String,      // "stripe_terminal", "adyen", "worldpay", ect.
    pub reference: String, // Payment intent / transaction id
    #[serde(default)]
    pub meta: Option<Value>, // arbitrary fields from the host app
}
