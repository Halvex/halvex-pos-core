use pos_types::{AreaID, OrderID, TableID, VenueID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub venue_id: VenueID,
    pub table_id: TableID,
    pub label: String,
    pub area_id: Option<AreaID>,
    pub status: TableStatus,
    pub active_order_id: Option<OrderID>,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TableStatus {
    Available,
    Occupied,
    Closed,
}

impl Table {
    pub fn new(
        venue_id: VenueID,
        table_id: TableID,
        label: impl Into<String>,
        area_id: Option<AreaID>,
    ) -> Self {
        Self {
            venue_id,
            table_id,
            label: label.into(),
            area_id,
            status: TableStatus::Available,
            active_order_id: None,
            version: 0,
        }
    }
}
