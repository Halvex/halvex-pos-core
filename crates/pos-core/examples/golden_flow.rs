use std::collections::HashMap;

use pos_core::{
    rebuild_check_from_events, rebuild_kds_from_events, rebuild_order_from_events,
    rebuild_table_from_events, route_command_envelope_enveloped, totals, AggregateRef,
    AggregateType, Check, CheckCommand, CheckTotals, CommandEnvelope, CoreCommand, CoreContext,
    CoreEvent, EventEnvelope, EventStore, ExpectedVersion, KdsTicket, Order, OrderCommand,
    RoutingTable, Table, TableCommand, TenderType, WorkflowCommand,
};
use pos_types::{CheckID, MenuItemID, Money, OrderID, OrderItemID, PaymentID, TableID, VenueID};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct AggregateKey {
    ty: u8,
    id: Uuid,
}

fn aggregate_type_key(ty: AggregateType) -> u8 {
    match ty {
        AggregateType::Order => 1,
        AggregateType::Check => 2,
        AggregateType::Table => 3,
    }
}

impl From<AggregateRef> for AggregateKey {
    fn from(value: AggregateRef) -> Self {
        Self {
            ty: aggregate_type_key(value.ty),
            id: value.id,
        }
    }
}

struct InMemoryEventStore {
    streams: HashMap<AggregateKey, Vec<EventEnvelope>>,
    idempotency: HashMap<(AggregateKey, String), Vec<EventEnvelope>>,
}

impl InMemoryEventStore {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            idempotency: HashMap::new(),
        }
    }

    fn current_version(&self, aggregate: AggregateRef) -> u64 {
        self.streams
            .get(&AggregateKey::from(aggregate))
            .and_then(|stream| stream.last())
            .and_then(|env| env.seq)
            .unwrap_or(0)
    }

    fn append_envelopes_in_order(
        &mut self,
        envelopes: Vec<EventEnvelope>,
    ) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
        let mut out = Vec::with_capacity(envelopes.len());
        for mut env in envelopes {
            let aggregate = env.aggregate;
            let key = AggregateKey::from(aggregate);
            let next_version = self.current_version(aggregate) + 1;
            env.seq = Some(next_version);
            self.streams.entry(key).or_default().push(env.clone());
            out.push(env);
        }
        Ok(out)
    }
}

impl EventStore for InMemoryEventStore {
    fn load_stream(
        &self,
        aggregate: AggregateRef,
    ) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
        Ok(self
            .streams
            .get(&AggregateKey::from(aggregate))
            .cloned()
            .unwrap_or_default())
    }

    fn append_to_stream(
        &mut self,
        aggregate: AggregateRef,
        expected: ExpectedVersion,
        mut events: Vec<EventEnvelope>,
    ) -> Result<u64, pos_core::PosError> {
        let current_version = self.current_version(aggregate);
        match expected {
            ExpectedVersion::Any => {}
            ExpectedVersion::NoStream => {
                if current_version != 0 {
                    return Err(pos_core::PosError::Conflict(
                        "stream is not empty".to_string(),
                    ));
                }
            }
            ExpectedVersion::Exact(version) => {
                if current_version != version {
                    return Err(pos_core::PosError::Conflict(format!(
                        "expected version {version}, got {current_version}"
                    )));
                }
            }
        }

        if events.is_empty() {
            return Ok(current_version);
        }

        let key = AggregateKey::from(aggregate);
        let stream = self.streams.entry(key).or_default();
        let mut next_version = current_version;
        for env in events.iter_mut() {
            if env.aggregate != aggregate {
                return Err(pos_core::PosError::Validation(
                    "event aggregate does not match stream".to_string(),
                ));
            }
            next_version += 1;
            env.seq = Some(next_version);
            stream.push(env.clone());
        }

        Ok(next_version)
    }

    fn get_idempotency(
        &self,
        aggregate: AggregateRef,
        key: &pos_core::IdempotencyKey,
    ) -> Result<Option<Vec<EventEnvelope>>, pos_core::PosError> {
        Ok(self
            .idempotency
            .get(&(AggregateKey::from(aggregate), key.0.clone()))
            .cloned())
    }

    fn put_idempotency(
        &mut self,
        aggregate: AggregateRef,
        key: pos_core::IdempotencyKey,
        events: Vec<EventEnvelope>,
    ) -> Result<(), pos_core::PosError> {
        self.idempotency
            .insert((AggregateKey::from(aggregate), key.0), events);
        Ok(())
    }
}

fn load_order(store: &InMemoryEventStore, order_id: OrderID) -> Result<Order, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::order(order_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Order(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-order event in order stream".to_string(),
                ))
            }
        }
    }
    rebuild_order_from_events(&events)
}

fn load_check(store: &InMemoryEventStore, check_id: CheckID) -> Result<Check, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::check(check_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Check(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-check event in check stream".to_string(),
                ))
            }
        }
    }
    rebuild_check_from_events(&events)
}

fn load_table(store: &InMemoryEventStore, table_id: TableID) -> Result<Table, pos_core::PosError> {
    let stream = store.load_stream(AggregateRef::table(table_id))?;
    let mut events = Vec::with_capacity(stream.len());
    for env in stream {
        match env.event {
            CoreEvent::Table(event) => events.push(event),
            _ => {
                return Err(pos_core::PosError::Validation(
                    "non-table event in table stream".to_string(),
                ))
            }
        }
    }
    rebuild_table_from_events(&events)
}

fn route_and_store(
    store: &mut InMemoryEventStore,
    ctx: CoreContext<'_>,
    env: CommandEnvelope,
    commands: &mut Vec<CommandEnvelope>,
    events: &mut Vec<EventEnvelope>,
) -> Result<Vec<EventEnvelope>, pos_core::PosError> {
    commands.push(env.clone());
    let routed = route_command_envelope_enveloped(ctx, env)?;
    let appended = store.append_envelopes_in_order(routed)?;
    events.extend(appended.clone());
    Ok(appended)
}

#[derive(Serialize)]
struct GoldenOutput {
    commands: Vec<CommandEnvelope>,
    events: Vec<EventEnvelope>,
    tickets: Vec<KdsTicket>,
    totals: CheckTotals,
}

fn main() -> Result<(), pos_core::PosError> {
    let mut store = InMemoryEventStore::new();
    let mut commands = Vec::new();
    let mut events = Vec::new();

    let venue_id = VenueID::new();
    let table_id = TableID::new();
    let order_id = OrderID::new();
    let check_id = CheckID::new();

    let burger_item_id = OrderItemID::new();
    let fries_item_id = OrderItemID::new();
    let burger_menu_id = MenuItemID::new();
    let fries_menu_id = MenuItemID::new();

    let burger_price = Money::usd(Decimal::new(1299, 2));
    let fries_price = Money::usd(Decimal::new(499, 2));

    let create_table = CommandEnvelope::new(CoreCommand::Table(TableCommand::CreateTable {
        venue_id,
        table_id,
        label: "T1".to_string(),
        area_id: None,
    }));
    route_and_store(
        &mut store,
        CoreContext::empty(),
        create_table,
        &mut commands,
        &mut events,
    )?;

    let table = load_table(&store, table_id)?;
    let open_table =
        CommandEnvelope::new(CoreCommand::Workflow(WorkflowCommand::OpenTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
        }));
    route_and_store(
        &mut store,
        CoreContext {
            table: Some(&table),
            order: None,
            check: None,
            to_check: None,
        },
        open_table,
        &mut commands,
        &mut events,
    )?;

    let order = load_order(&store, order_id)?;
    let add_burger = CommandEnvelope::new(CoreCommand::Order(OrderCommand::AddItem {
        venue_id,
        order_id,
        order_item_id: burger_item_id,
        menu_item_id: burger_menu_id,
        name: "Burger".to_string(),
        unit_price: burger_price,
        qty: 1,
        notes: Some("No onions".to_string()),
    }));
    route_and_store(
        &mut store,
        CoreContext {
            order: Some(&order),
            check: None,
            table: None,
            to_check: None,
        },
        add_burger,
        &mut commands,
        &mut events,
    )?;

    let order = load_order(&store, order_id)?;
    let add_fries = CommandEnvelope::new(CoreCommand::Order(OrderCommand::AddItem {
        venue_id,
        order_id,
        order_item_id: fries_item_id,
        menu_item_id: fries_menu_id,
        name: "Fries".to_string(),
        unit_price: fries_price,
        qty: 2,
        notes: None,
    }));
    route_and_store(
        &mut store,
        CoreContext {
            order: Some(&order),
            check: None,
            table: None,
            to_check: None,
        },
        add_fries,
        &mut commands,
        &mut events,
    )?;

    let check = load_check(&store, check_id)?;
    let add_burger_line =
        CommandEnvelope::new(CoreCommand::Check(CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id: burger_item_id,
            qty: 1,
            unit_price: burger_price,
            name: "Burger".to_string(),
        }));
    route_and_store(
        &mut store,
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        add_burger_line,
        &mut commands,
        &mut events,
    )?;

    let check = load_check(&store, check_id)?;
    let add_fries_line =
        CommandEnvelope::new(CoreCommand::Check(CheckCommand::AddLineFromOrderItem {
            venue_id,
            check_id,
            order_item_id: fries_item_id,
            qty: 2,
            unit_price: fries_price,
            name: "Fries".to_string(),
        }));
    route_and_store(
        &mut store,
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        add_fries_line,
        &mut commands,
        &mut events,
    )?;

    let table = load_table(&store, table_id)?;
    let routing = RoutingTable::default();
    let kds_state = rebuild_kds_from_events(&events, &routing, Some(&table))?;
    let mut tickets = pos_core::kitchen_tickets(&kds_state, None);
    tickets.sort_by(|a, b| {
        let station_cmp = a.station.cmp(&b.station);
        if station_cmp == std::cmp::Ordering::Equal {
            a.ticket_id.as_bytes().cmp(b.ticket_id.as_bytes())
        } else {
            station_cmp
        }
    });

    let check = load_check(&store, check_id)?;
    let payment_amount = totals(&check).total;
    let payment_id = PaymentID::new();

    let authorise = CommandEnvelope::new(CoreCommand::Check(CheckCommand::AuthorisePayment {
        venue_id,
        check_id,
        payment_id,
        tender: TenderType::Card,
        amount: payment_amount,
        tip: None,
        processor: None,
    }));
    route_and_store(
        &mut store,
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        authorise,
        &mut commands,
        &mut events,
    )?;

    let check = load_check(&store, check_id)?;
    let capture = CommandEnvelope::new(CoreCommand::Check(CheckCommand::CapturePayment {
        venue_id,
        check_id,
        payment_id,
        amount: payment_amount,
    }));
    route_and_store(
        &mut store,
        CoreContext {
            order: None,
            check: Some(&check),
            table: None,
            to_check: None,
        },
        capture,
        &mut commands,
        &mut events,
    )?;

    let order = load_order(&store, order_id)?;
    let check = load_check(&store, check_id)?;
    let table = load_table(&store, table_id)?;
    let close_table =
        CommandEnvelope::new(CoreCommand::Workflow(WorkflowCommand::CloseTableSession {
            venue_id,
            table_id,
            order_id,
            check_id,
            reason: "settled".to_string(),
        }));
    route_and_store(
        &mut store,
        CoreContext {
            order: Some(&order),
            check: Some(&check),
            table: Some(&table),
            to_check: None,
        },
        close_table,
        &mut commands,
        &mut events,
    )?;

    let check = load_check(&store, check_id)?;
    let final_totals = totals(&check);

    let output = GoldenOutput {
        commands,
        events,
        tickets,
        totals: final_totals,
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|err| pos_core::PosError::Validation(err.to_string()))?;
    println!("{json}");

    Ok(())
}
