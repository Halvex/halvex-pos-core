use pos_types::{MenuItemID, Money, OrderID, OrderItemID, VenueID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub venue_id: VenueID,
    pub order_id: OrderID,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrderStatus {
    Open,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub item_id: OrderItemID,
    pub menu_item_id: MenuItemID,
    pub name: String,
    pub unit_price: Money,
    pub qty: u32,
    pub notes: Option<String>,
}

impl Order {
    pub fn new(venue_id: VenueID, order_id: OrderID) -> Self {
        Self {
            venue_id,
            order_id,
            status: OrderStatus::Open,
            items: vec![],
            version: 0,
        }
    }
}
