use pos_types::{AreaID, OrderID, TableID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TableCommand {
    CreateTable {
        venue_id: VenueID,
        table_id: TableID,
        label: String,
        area_id: Option<AreaID>,
    },

    AssignOrder {
        venue_id: VenueID,
        table_id: TableID,
        order_id: OrderID,
    },

    UnassignOrder {
        venue_id: VenueID,
        table_id: TableID,
        reason: String,
    },

    CloseTable {
        venue_id: VenueID,
        table_id: TableID,
    },

    ReopenTable {
        venue_id: VenueID,
        table_id: TableID,
    },
}
