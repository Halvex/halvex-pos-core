use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pos_types::{OrderID, VenueID};

use super::StationId;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum KdsEvent {
    TicketAcknowledged {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
    },
    TicketBumped {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
    },
    TicketRecalled {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
    },
    LineStarted {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
        line_id: Uuid,
    },
    LineDone {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
        line_id: Uuid,
    },
    TicketNoteAdded {
        venue_id: VenueID,
        station: StationId,
        order_id: OrderID,
        note: String,
    },
}
