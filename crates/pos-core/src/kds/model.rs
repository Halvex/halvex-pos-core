use std::collections::{BTreeMap, HashMap};

use pos_types::{OrderID, OrderItemID, TableID, VenueID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type StationId = String;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KdsTicketKey {
    pub order_id: OrderID,
    pub station: StationId,
}

impl KdsTicketKey {
    pub fn new(order_id: OrderID, station: StationId) -> Self {
        Self { order_id, station }
    }
}

impl Ord for KdsTicketKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let order_cmp = self.order_id.0.as_bytes().cmp(other.order_id.0.as_bytes());
        if order_cmp == std::cmp::Ordering::Equal {
            self.station.cmp(&other.station)
        } else {
            order_cmp
        }
    }
}

impl PartialOrd for KdsTicketKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsTicket {
    pub ticket_id: Uuid,
    pub venue_id: VenueID,
    pub station: StationId,

    pub order_id: OrderID,
    pub table_id: Option<TableID>,
    pub table_label: Option<String>,

    pub status: KdsTicketStatus,
    pub created_at_seq: u64,
    pub updated_at_seq: u64,

    pub lines: Vec<KdsLine>,
    pub notes: Vec<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsLine {
    pub line_id: Uuid,
    pub name: String,
    pub qty: u32,

    pub modifiers: Vec<String>,
    pub note: Option<String>,

    pub status: KdsLineStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum KdsTicketStatus {
    New,
    Acknowledged,
    InProgress,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum KdsLineStatus {
    New,
    InProgress,
    Done,
    Voided,
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KdsState {
    pub tickets: BTreeMap<KdsTicketKey, KdsTicket>,
    pub cursor: u64,
    pub item_to_ticket: HashMap<OrderItemID, KdsTicketKey>,
    pub line_to_ticket: HashMap<Uuid, KdsTicketKey>,
    pub changes: Vec<KdsChange>,
}

impl KdsState {
    pub fn new() -> Self {
        Self {
            tickets: BTreeMap::new(),
            cursor: 0,
            item_to_ticket: HashMap::new(),
            line_to_ticket: HashMap::new(),
            changes: Vec::new(),
        }
    }
}

impl Default for KdsState {
    fn default() -> Self {
        Self::new()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsChange {
    pub seq: u64,
    pub station: StationId,
    pub order_id: OrderID,
    pub change: KdsChangeKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum KdsChangeKind {
    LineAdded {
        line_id: Uuid,
        name: String,
        qty: u32,
        modifiers: Vec<String>,
        note: Option<String>,
    },
    LineVoided {
        line_id: Uuid,
        name: String,
        qty: u32,
        modifiers: Vec<String>,
        note: Option<String>,
    },
    LineQtyChanged {
        line_id: Uuid,
        name: String,
        from_qty: u32,
        to_qty: u32,
        modifiers: Vec<String>,
        note: Option<String>,
    },
    TicketNoteAdded {
        note: String,
    },
}
