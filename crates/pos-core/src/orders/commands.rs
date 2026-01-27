use pos_types::{MenuItemID, Money, OrderID, OrderItemID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrderCommand {
    OpenOrder {
        venue_id: VenueID,
        order_id: OrderID,
    },
    AddItem {
        venue_id: VenueID,
        order_id: OrderID,
        order_item_id: OrderItemID,
        menu_item_id: MenuItemID,
        name: String,
        unit_price: Money,
        qty: u32,
        notes: Option<String>,
    },
    RemoveItem {
        venue_id: VenueID,
        order_id: OrderID,
        order_item_id: OrderItemID,
        reason: String,
    },
    CloseOrder {
        venue_id: VenueID,
        order_id: OrderID,
    },
}
