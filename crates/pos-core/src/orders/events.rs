use pos_types::{MenuItemID, Money, OrderID, OrderItemID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrderEvent {
    OrderOpened {
        venue_id: VenueID,
        order_id: OrderID,
    },
    ItemAdded {
        venue_id: VenueID,
        order_id: OrderID,
        order_item_id: OrderItemID,
        menu_item_id: MenuItemID,
        name: String,
        unit_price: Money,
        qty: u32,
        notes: Option<String>,
    },
    ItemRemoved {
        venue_id: VenueID,
        order_id: OrderID,
        order_item_id: OrderItemID,
        reason: Option<String>,
    },
    OrderClosed {
        venue_id: VenueID,
        order_id: OrderID,
    },
}
