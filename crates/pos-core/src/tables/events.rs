use pos_types::{AreaID, OrderID, TableID, VenueID};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TableEvent {
    TableCreated {
        venue_id: VenueID,
        table_id: TableID,
        label: String,
        area_id: Option<AreaID>,
    },

    OrderAssignedToTable {
        venue_id: VenueID,
        table_id: TableID,
        order_id: OrderID,
    },

    OrderUnassingedFromTable {
        venue_id: VenueID,
        table_id: TableID,
        reason: String,
    },

    TableClosed {
        venue_id: VenueID,
        table_id: TableID,
    },

    TableReopened {
        venue_id: VenueID,
        table_id: TableID,
    },
}
