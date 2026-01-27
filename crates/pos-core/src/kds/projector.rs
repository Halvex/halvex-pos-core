use pos_types::{OrderItemID, PosError};
use uuid::Uuid;

use crate::core::{CoreEvent, EventEnvelope};
use crate::orders::OrderEvent;
use crate::tables::Table;

use super::events::KdsEvent;
use super::model::{
    KdsChange, KdsChangeKind, KdsLine, KdsLineStatus, KdsState, KdsTicket, KdsTicketKey,
    KdsTicketStatus,
};
use super::routing::{station_for_item, RoutingTable};

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a64(data: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// Deterministic ticket IDs keep KDS tickets stable across replays.
fn stable_uuid_for_ticket(order_id: pos_types::OrderID, station: &str) -> Uuid {
    let mut data = Vec::with_capacity(16 + station.len());
    data.extend_from_slice(order_id.0.as_bytes());
    data.extend_from_slice(station.as_bytes());

    let h1 = fnv1a64(&data, FNV_OFFSET_BASIS);
    let h2 = fnv1a64(&data, FNV_OFFSET_BASIS ^ 0x9e3779b97f4a7c15);
    Uuid::from_u128(((h1 as u128) << 64) | (h2 as u128))
}

// Use order item IDs to keep line IDs consistent across projections.
fn line_id_for_item(order_item_id: OrderItemID) -> Uuid {
    order_item_id.0
}

fn ensure_ticket<'a>(
    state: &'a mut KdsState,
    key: KdsTicketKey,
    venue_id: pos_types::VenueID,
    table: Option<&Table>,
    seq: u64,
) -> &'a mut KdsTicket {
    state.tickets.entry(key.clone()).or_insert_with(|| {
        let mut ticket = KdsTicket {
            ticket_id: stable_uuid_for_ticket(key.order_id, &key.station),
            venue_id,
            station: key.station.clone(),
            order_id: key.order_id,
            table_id: table.map(|t| t.table_id),
            table_label: table.map(|t| t.label.clone()),
            status: KdsTicketStatus::New,
            created_at_seq: seq,
            updated_at_seq: seq,
            lines: Vec::new(),
            notes: Vec::new(),
        };
        if let Some(t) = table {
            ticket.table_id = Some(t.table_id);
            ticket.table_label = Some(t.label.clone());
        }
        ticket
    })
}

fn update_ticket_table_meta(ticket: &mut KdsTicket, table: Option<&Table>) {
    if let Some(t) = table {
        ticket.table_id = Some(t.table_id);
        ticket.table_label = Some(t.label.clone());
    }
}

pub fn apply_order_event(
    state: &mut KdsState,
    event: &OrderEvent,
    seq: u64,
    routing: &RoutingTable,
    table: Option<&Table>,
) -> Result<(), PosError> {
    state.cursor = state.cursor.max(seq);

    match event {
        OrderEvent::OrderOpened { .. } => {
            // no-op until items arrive
        }
        OrderEvent::ItemAdded {
            venue_id,
            order_id,
            order_item_id,
            menu_item_id,
            name,
            qty,
            notes,
            ..
        } => {
            let station = station_for_item(*menu_item_id, name, routing);
            let key = KdsTicketKey::new(*order_id, station.clone());
            let line_id = line_id_for_item(*order_item_id);
            let (change, new_mappings) = {
                let ticket = ensure_ticket(state, key.clone(), *venue_id, table, seq);
                update_ticket_table_meta(ticket, table);

                if let Some(line) = ticket.lines.iter_mut().find(|l| l.line_id == line_id) {
                    let from_qty = line.qty;
                    line.qty = line.qty.saturating_add(*qty);
                    if line.status == KdsLineStatus::Voided {
                        line.status = KdsLineStatus::New;
                    }
                    ticket.updated_at_seq = seq;
                    (
                        Some(KdsChange {
                            seq,
                            station: station.clone(),
                            order_id: *order_id,
                            change: KdsChangeKind::LineQtyChanged {
                                line_id,
                                name: line.name.clone(),
                                from_qty,
                                to_qty: line.qty,
                                modifiers: line.modifiers.clone(),
                                note: line.note.clone(),
                            },
                        }),
                        None,
                    )
                } else {
                    let line = KdsLine {
                        line_id,
                        name: name.clone(),
                        qty: *qty,
                        modifiers: Vec::new(),
                        note: notes.clone(),
                        status: KdsLineStatus::New,
                    };
                    ticket.lines.push(line.clone());
                    ticket.updated_at_seq = seq;

                    (
                        Some(KdsChange {
                            seq,
                            station: station.clone(),
                            order_id: *order_id,
                            change: KdsChangeKind::LineAdded {
                                line_id,
                                name: line.name.clone(),
                                qty: line.qty,
                                modifiers: line.modifiers.clone(),
                                note: line.note.clone(),
                            },
                        }),
                        Some((*order_item_id, key.clone(), line_id)),
                    )
                }
            };

            if let Some((item_id, ticket_key, line_id)) = new_mappings {
                state.item_to_ticket.insert(item_id, ticket_key.clone());
                state.line_to_ticket.insert(line_id, ticket_key);
            }

            if let Some(change) = change {
                state.changes.push(change);
            }
        }
        OrderEvent::ItemRemoved {
            order_item_id,
            order_id,
            ..
        } => {
            let key = state
                .item_to_ticket
                .get(order_item_id)
                .cloned()
                .ok_or_else(|| PosError::NotFound("kds ticket for item not found".into()))?;
            let line_id = line_id_for_item(*order_item_id);
            let mut change: Option<KdsChange> = None;

            {
                let ticket = state
                    .tickets
                    .get_mut(&key)
                    .ok_or_else(|| PosError::NotFound("kds ticket not found".into()))?;
                let line = ticket
                    .lines
                    .iter_mut()
                    .find(|l| l.line_id == line_id)
                    .ok_or_else(|| PosError::NotFound("kds line not found".into()))?;

                if line.status != KdsLineStatus::Voided {
                    line.status = KdsLineStatus::Voided;
                    ticket.updated_at_seq = seq;
                    change = Some(KdsChange {
                        seq,
                        station: key.station.clone(),
                        order_id: *order_id,
                        change: KdsChangeKind::LineVoided {
                            line_id,
                            name: line.name.clone(),
                            qty: line.qty,
                            modifiers: line.modifiers.clone(),
                            note: line.note.clone(),
                        },
                    });
                }
            }

            if let Some(change) = change {
                state.changes.push(change);
            }
        }
        OrderEvent::OrderClosed { order_id, .. } => {
            for (key, ticket) in state.tickets.iter_mut() {
                if key.order_id == *order_id {
                    if ticket.status != KdsTicketStatus::Cancelled {
                        ticket.status = KdsTicketStatus::Completed;
                    }
                    ticket.updated_at_seq = seq;
                }
            }
        }
    }

    Ok(())
}

fn find_ticket_mut<'a>(
    state: &'a mut KdsState,
    station: &str,
    order_id: pos_types::OrderID,
) -> Result<&'a mut KdsTicket, PosError> {
    let key = KdsTicketKey::new(order_id, station.to_string());
    state
        .tickets
        .get_mut(&key)
        .ok_or_else(|| PosError::NotFound("kds ticket not found".into()))
}

fn find_ticket_by_line_mut(
    state: &mut KdsState,
    line_id: Uuid,
) -> Result<&mut KdsTicket, PosError> {
    let key = state
        .line_to_ticket
        .get(&line_id)
        .cloned()
        .ok_or_else(|| PosError::NotFound("kds ticket for line not found".into()))?;
    state
        .tickets
        .get_mut(&key)
        .ok_or_else(|| PosError::NotFound("kds ticket not found".into()))
}

pub fn apply_kds_event(state: &mut KdsState, event: &KdsEvent, seq: u64) -> Result<(), PosError> {
    state.cursor = state.cursor.max(seq);

    match event {
        KdsEvent::TicketAcknowledged {
            station, order_id, ..
        } => {
            let ticket = find_ticket_mut(state, station, *order_id)?;
            ticket.status = KdsTicketStatus::Acknowledged;
            ticket.updated_at_seq = seq;
        }
        KdsEvent::TicketBumped {
            station, order_id, ..
        } => {
            let ticket = find_ticket_mut(state, station, *order_id)?;
            ticket.status = KdsTicketStatus::Completed;
            ticket.updated_at_seq = seq;
        }
        KdsEvent::TicketRecalled {
            station, order_id, ..
        } => {
            let ticket = find_ticket_mut(state, station, *order_id)?;
            ticket.status = KdsTicketStatus::InProgress;
            ticket.updated_at_seq = seq;
        }
        KdsEvent::LineStarted { line_id, .. } => {
            let ticket = find_ticket_by_line_mut(state, *line_id)?;
            let line = ticket
                .lines
                .iter_mut()
                .find(|l| l.line_id == *line_id)
                .ok_or_else(|| PosError::NotFound("kds line not found".into()))?;
            line.status = KdsLineStatus::InProgress;
            if matches!(
                ticket.status,
                KdsTicketStatus::New | KdsTicketStatus::Acknowledged
            ) {
                ticket.status = KdsTicketStatus::InProgress;
            }
            ticket.updated_at_seq = seq;
        }
        KdsEvent::LineDone { line_id, .. } => {
            let ticket = find_ticket_by_line_mut(state, *line_id)?;
            let line = ticket
                .lines
                .iter_mut()
                .find(|l| l.line_id == *line_id)
                .ok_or_else(|| PosError::NotFound("kds line not found".into()))?;
            line.status = KdsLineStatus::Done;
            if ticket
                .lines
                .iter()
                .all(|l| matches!(l.status, KdsLineStatus::Done | KdsLineStatus::Voided))
                && !matches!(
                    ticket.status,
                    KdsTicketStatus::Completed | KdsTicketStatus::Cancelled
                )
            {
                ticket.status = KdsTicketStatus::Ready;
            }
            ticket.updated_at_seq = seq;
        }
        KdsEvent::TicketNoteAdded {
            station,
            order_id,
            note,
            ..
        } => {
            {
                let ticket = find_ticket_mut(state, station, *order_id)?;
                ticket.notes.push(note.clone());
                ticket.updated_at_seq = seq;
            }
            state.changes.push(KdsChange {
                seq,
                station: station.clone(),
                order_id: *order_id,
                change: KdsChangeKind::TicketNoteAdded { note: note.clone() },
            });
        }
    }

    Ok(())
}

pub fn apply_core_event_to_kds(
    state: &mut KdsState,
    env: &EventEnvelope,
    routing: &RoutingTable,
    table: Option<&Table>,
) -> Result<(), PosError> {
    let seq = env.seq.unwrap_or(state.cursor + 1);
    match &env.event {
        CoreEvent::Order(order_event) => apply_order_event(state, order_event, seq, routing, table),
        _ => Ok(()),
    }
}

pub fn kitchen_tickets(state: &KdsState, station: Option<&str>) -> Vec<KdsTicket> {
    let mut tickets: Vec<KdsTicket> = state
        .tickets
        .values()
        .filter(|ticket| station.map(|s| ticket.station == s).unwrap_or(true))
        .cloned()
        .collect();

    for ticket in &mut tickets {
        ticket
            .lines
            .sort_by(|a, b| a.line_id.as_bytes().cmp(b.line_id.as_bytes()));
    }

    tickets
}

pub fn expo_view(state: &KdsState) -> Vec<KdsTicket> {
    kitchen_tickets(state, None)
}
