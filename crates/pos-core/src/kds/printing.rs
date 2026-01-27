use std::collections::HashMap;

use pos_types::{OrderID, TableID, VenueID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::{KdsChangeKind, KdsLineStatus, KdsState, KdsTicket, KdsTicketKey};
use super::{kitchen_tickets, StationId};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsPrintCursor {
    pub station: StationId,
    pub last_seq: u64,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsDeltaTicket {
    pub ticket_id: Uuid,
    pub venue_id: VenueID,
    pub station: StationId,
    pub order_id: OrderID,
    pub table_id: Option<TableID>,
    pub table_label: Option<String>,
    pub seq: u64,
    pub lines: Vec<KdsDeltaLine>,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdsDeltaLine {
    pub line_id: Uuid,
    pub name: String,
    pub qty: u32,
    pub modifiers: Vec<String>,
    pub note: Option<String>,
    pub kind: KdsDeltaKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum KdsDeltaKind {
    Added,
    Voided,
    QtyChanged { from_qty: u32, to_qty: u32 },
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintableTicket {
    pub ticket: KdsTicket,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintableLine {
    pub line_id: Uuid,
    pub name: String,
    pub qty: u32,
    pub modifiers: Vec<String>,
    pub note: Option<String>,
    pub status: KdsLineStatus,
}

pub fn tickets_for_printing(state: &KdsState, station: Option<&str>) -> Vec<PrintableTicket> {
    kitchen_tickets(state, station)
        .into_iter()
        .map(|ticket| PrintableTicket { ticket })
        .collect()
}

pub fn delta_tickets_since(state: &KdsState, station: &str, last_seq: u64) -> Vec<KdsDeltaTicket> {
    let mut out: Vec<KdsDeltaTicket> = Vec::new();
    let mut index: HashMap<KdsTicketKey, usize> = HashMap::new();

    for change in state
        .changes
        .iter()
        .filter(|c| c.seq > last_seq && c.station == station)
    {
        let delta_line = match delta_line_from_change(&change.change) {
            Some(line) => line,
            None => continue,
        };
        let key = KdsTicketKey::new(change.order_id, change.station.clone());
        let idx = if let Some(existing) = index.get(&key) {
            *existing
        } else {
            let ticket = match state.tickets.get(&key) {
                Some(ticket) => ticket,
                None => continue,
            };
            out.push(KdsDeltaTicket {
                ticket_id: ticket.ticket_id,
                venue_id: ticket.venue_id,
                station: change.station.clone(),
                order_id: change.order_id,
                table_id: ticket.table_id,
                table_label: ticket.table_label.clone(),
                seq: change.seq,
                lines: Vec::new(),
            });
            let idx = out.len() - 1;
            index.insert(key, idx);
            idx
        };

        out[idx].lines.push(delta_line);
        out[idx].seq = out[idx].seq.max(change.seq);
    }

    out
}

fn delta_line_from_change(change: &KdsChangeKind) -> Option<KdsDeltaLine> {
    match change {
        KdsChangeKind::LineAdded {
            line_id,
            name,
            qty,
            modifiers,
            note,
        } => Some(KdsDeltaLine {
            line_id: *line_id,
            name: name.clone(),
            qty: *qty,
            modifiers: modifiers.clone(),
            note: note.clone(),
            kind: KdsDeltaKind::Added,
        }),
        KdsChangeKind::LineVoided {
            line_id,
            name,
            qty,
            modifiers,
            note,
        } => Some(KdsDeltaLine {
            line_id: *line_id,
            name: name.clone(),
            qty: *qty,
            modifiers: modifiers.clone(),
            note: note.clone(),
            kind: KdsDeltaKind::Voided,
        }),
        KdsChangeKind::LineQtyChanged {
            line_id,
            name,
            from_qty,
            to_qty,
            modifiers,
            note,
        } => Some(KdsDeltaLine {
            line_id: *line_id,
            name: name.clone(),
            qty: *to_qty,
            modifiers: modifiers.clone(),
            note: note.clone(),
            kind: KdsDeltaKind::QtyChanged {
                from_qty: *from_qty,
                to_qty: *to_qty,
            },
        }),
        KdsChangeKind::TicketNoteAdded { .. } => None,
    }
}

pub fn printable_lines_from_ticket(ticket: &KdsTicket) -> Vec<PrintableLine> {
    let mut lines: Vec<PrintableLine> = ticket
        .lines
        .iter()
        .map(|line| PrintableLine {
            line_id: line.line_id,
            name: line.name.clone(),
            qty: line.qty,
            modifiers: line.modifiers.clone(),
            note: line.note.clone(),
            status: line.status,
        })
        .collect();

    lines.sort_by(|a, b| a.line_id.as_bytes().cmp(b.line_id.as_bytes()));
    lines
}

pub fn ticket_has_active_lines(ticket: &KdsTicket) -> bool {
    ticket
        .lines
        .iter()
        .any(|line| !matches!(line.status, KdsLineStatus::Voided))
}

pub fn ticket_is_ready(ticket: &KdsTicket) -> bool {
    ticket
        .lines
        .iter()
        .all(|line| matches!(line.status, KdsLineStatus::Done | KdsLineStatus::Voided))
}
