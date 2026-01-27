use pos_types::{PosError, VenueID};

use super::{KdsPrintCursor, KdsState};

pub trait KdsStateStore {
    fn load_state(&self, venue_id: VenueID) -> Result<KdsState, PosError>;
    fn save_state(&self, venue_id: VenueID, state: &KdsState) -> Result<(), PosError>;
}

pub trait KdsCursorStore {
    fn load_cursor(&self, venue_id: VenueID, station: &str) -> Result<KdsPrintCursor, PosError>;
    fn save_cursor(&self, venue_id: VenueID, cursor: &KdsPrintCursor) -> Result<(), PosError>;
}
