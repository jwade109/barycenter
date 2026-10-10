use bary_core::prelude::*;
use bary_ipc::*;
use bary_sim::*;
use log::*;
use log::{debug, info, warn};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use crate::application::Application;

fn serialize_table<T: Serialize>(entities: &Components<T>) -> Option<Vec<u8>> {
    bincode::serialize(&entities).ok()
}

fn get_blob(world: &World, table: TableIdent) -> Option<Blob> {
    let data = match table {
        TableIdent::Blueprints => serialize_table(&world.blueprints),
        TableIdent::Grids => serialize_table(&world.grids),
        TableIdent::Protos => serialize_table(&world.prototypes),
        TableIdent::Parts => serialize_table(&world.parts),
        TableIdent::Thrusters => serialize_table(&world.thrusters),
        TableIdent::Computers => serialize_table(&world.computers),
        TableIdent::Chunks => serialize_table(&world.terrain_chunks),
        TableIdent::Tiles => serialize_table(&world.terrain_tiles),
        TableIdent::Inventories => serialize_table(&world.inventories),
        TableIdent::Machines => serialize_table(&world.machines),
        TableIdent::Asteroids => serialize_table(&world.asteroids),
        TableIdent::Pipes => serialize_table(&world.pipes),
        TableIdent::Lights => serialize_table(&world.lights),
        TableIdent::Excavators => serialize_table(&world.excavators),
        TableIdent::Players => serialize_table(&world.players),
    };
    data.map(|data| Blob::new(data, table))
}

struct PaginatedTransaction {
    blobs: BTreeMap<TableIdent, Blob>,
    offset: usize,
    next_blob: Option<TableIdent>,
    last_sent: Instant,
}

impl PaginatedTransaction {
    fn from_world(world: &World) -> Self {
        let mut blobs = BTreeMap::new();
        let all: Vec<_> = TableIdent::all().collect();
        for ident in &all {
            let blob = get_blob(world, *ident).expect("This is bad");
            blobs.insert(*ident, blob);
        }

        Self {
            blobs,
            next_blob: all.first().cloned(),
            offset: 0,
            last_sent: Instant::now(),
        }
    }
}

pub fn sync_frame_from_world(world: &World) -> SyncFrame {
    let grid_bytes = bincode::serialize(&world.grids).unwrap();
    let grid_hash = u128::from_be_bytes(md5::compute(&grid_bytes).0);
    let inv_bytes = bincode::serialize(&world.inventories).unwrap();
    let inv_hash = u128::from_be_bytes(md5::compute(&inv_bytes).0);
    let thr_bytes = bincode::serialize(&world.thrusters).unwrap();
    let thr_hash = u128::from_be_bytes(md5::compute(&thr_bytes).0);
    SyncFrame {
        tick: world.ticks,
        next_id: world.spawner.next(),
        grids: grid_hash,
        inventories: inv_hash,
        thrusters: thr_hash,
    }
}

pub struct HeadlessServerApp {
    world: World,
    client_telemetry: HashMap<ClientId, ClientTelemetry>,
    usernames: HashMap<ClientId, String>,
    tick_rate: u32,
    node: ServerNode,
    update_timer: WallTimer,
    print_timer: WallTimer,
    md5_sync_timer: WallTimer,
    queued_deltas: Vec<WorldDelta>,
    transactions: HashMap<ClientId, PaginatedTransaction>,
}

impl HeadlessServerApp {
    pub fn new(save_path: Option<String>, port: u16, speed: u32) -> BaryResult<Self> {
        let node = ServerNode::new(port);

        let world = World::empty();

        let mut s = Self {
            world,
            client_telemetry: HashMap::new(),
            usernames: HashMap::new(),
            tick_rate: speed,
            node,
            update_timer: WallTimer::hz(TICKS_PER_SECOND as u32),
            print_timer: WallTimer::hz(1),
            md5_sync_timer: WallTimer::hz(2),
            queued_deltas: Vec::new(),
            transactions: HashMap::new(),
        };

        if let Some(path) = save_path {
            s.world = load_world(path)?;
        }

        Ok(s)
    }

    pub fn get_statistics(&self) -> ServerStatistics {
        let mut clients = Vec::new();
        for client in self.node.renet().clients_id_iter() {
            if let Ok(info) = self.node.renet().network_info(client) {
                clients.push((client, info.into()));
            }
        }
        ServerStatistics { clients }
    }

    fn send_driver_packet(&mut self) {
        let deltas = self.queued_deltas.drain(..).collect();
        let msg = MessageKind::Driver {
            ticks: self.world.ticks,
            deltas,
            players: self.world.players.clone(),
        };
        self.node.broadcast(msg);
    }

    fn send_tlm_ack(&mut self) {
        self.node.broadcast(MessageKind::Ack);
    }

    fn send_tlm_server_info(&mut self) {
        self.node
            .broadcast(MessageKind::ServerStatistics(self.get_statistics()));
    }

    fn send_md5_sync_packet(&mut self) {
        let frame = sync_frame_from_world(&self.world);
        let msg = MessageKind::SyncFrame(frame);
        self.node.broadcast(msg);
    }

    fn process_transactions(&mut self) {
        let dur = Duration::from_millis(80);
        let now = Instant::now();
        for (client, tr) in &mut self.transactions {
            let next_send = tr.last_sent + dur;
            if now < next_send {
                continue;
            }

            if let Some(next) = tr.next_blob {
                debug!("Sending blob {next} to {client}");
                if let Some(blob) = tr.blobs.remove(&next) {
                    let kind = MessageKind::BlobResponse(blob);
                    self.node.send(*client, kind);
                }
                tr.next_blob = tr.next_blob.map(|t| t.next()).flatten();
                tr.last_sent = now;
            }

            if tr.next_blob.is_none() {
                info!("Finished transaction!");
                let kind = MessageKind::FinishWorldDownload;
                self.node.send(*client, kind);
            }
        }

        self.transactions
            .retain(|_client, tr| tr.next_blob.is_some());
    }

    fn on_accept_message(&mut self, msg: Message) {
        debug!("Got a command: {:?}", msg);
        self.send_tlm_ack();

        let MessageSource::Client(client_id) = msg.source else {
            warn!("Got a message with unexpected source: {:?}", msg.source);
            return;
        };

        match msg.kind {
            MessageKind::Introduction { username } => {
                self.on_accept_introduction(client_id, username);
            }
            MessageKind::ClientTelemetry(tlm) => {
                self.on_accept_client_tlm(client_id, tlm);
            }
            MessageKind::Ping => {
                self.on_accept_ping(client_id);
            }
            MessageKind::ClientSays(s) => {
                self.on_accept_client_say(client_id, s);
            }
            MessageKind::SetSimSpeed(s) => {
                self.on_accept_set_sim_speed(client_id, s);
            }
            MessageKind::RequestServerStatistics => {
                self.on_accept_req_server_info(client_id);
            }
            MessageKind::ClientBlobRequest(table) => {
                self.on_accept_client_blob_request(client_id, table);
            }
            MessageKind::ClientBlobRequestAll => {
                self.on_accept_client_blob_request_all(client_id);
            }
            MessageKind::RequestDelta(delta) => {
                self.on_accept_world_delta(client_id, delta);
            }
            MessageKind::LoadSave(path) => {
                self.on_accept_load_save(path);
            }
            MessageKind::BeginAsyncWorldDownload => self.begin_async_world_download(client_id),
            _ => self.on_unsupported_message(),
        }
    }

    fn on_accept_world_delta(&mut self, id: ClientId, delta: WorldDelta) -> Option<()> {
        self.ensure_client_is_connected(id)?;
        if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
            error!("Failed to apply delta: {e}");
        } else {
            debug!("Successfully applied delta: {delta:?}");
            self.queued_deltas.push(delta.clone());
        }
        Some(())
    }

    fn on_accept_introduction(&mut self, id: ClientId, username: String) {
        if !self.usernames.contains_key(&id) {
            info!("Client {id} has introduced themselves as {username}");
            self.usernames.insert(id, username.clone());
            self.node.broadcast(MessageKind::PlayerConnected(username));
            self.node.send(id, MessageKind::TakeYourHatOff);
        } else {
            error!("Client {id} has reintroduced themselves")
        }
    }

    fn begin_async_world_download(&mut self, client: ClientId) {
        if self.transactions.contains_key(&client) {
            error!("Transaction already underway for client {client}");
            return;
        }
        let tr = PaginatedTransaction::from_world(&self.world);
        self.transactions.insert(client, tr);
        info!("Started async world download for {client}");
    }

    fn on_unsupported_message(&mut self) {
        warn!("Unsupported message type!");
        self.node.broadcast(MessageKind::Unsupported);
    }

    fn on_accept_load_save(&mut self, path: String) {
        match load_world(&path) {
            Ok(world) => {
                self.world = world;
                info!("Loaded {path}");
                self.node.broadcast(MessageKind::HaveNewSave);
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
    }

    fn on_accept_ping(&mut self, id: ClientId) -> Option<()> {
        self.ensure_client_is_connected(id)?;
        self.node.broadcast(MessageKind::Pong);
        Some(())
    }

    fn ensure_client_is_connected(&self, id: ClientId) -> Option<()> {
        if !self.usernames.contains_key(&id) {
            error!("Client {id} is not connected!");
            return None;
        }
        Some(())
    }

    fn on_accept_client_tlm(&mut self, id: ClientId, tlm: ClientTelemetry) {
        self.ensure_client_is_connected(id);
        self.client_telemetry.insert(id, tlm);
    }

    fn on_accept_client_say(&mut self, id: ClientId, s: String) -> Option<()> {
        self.ensure_client_is_connected(id)?;
        self.node.broadcast(MessageKind::ChatMessage(id, s));
        Some(())
    }

    fn on_accept_set_sim_speed(&mut self, id: ClientId, speed: u32) -> Option<()> {
        self.ensure_client_is_connected(id)?;
        self.tick_rate = speed;
        self.node.send(id, MessageKind::Ack);
        Some(())
    }

    fn on_accept_req_server_info(&mut self, id: ClientId) {
        self.node
            .send(id, MessageKind::ServerStatistics(self.get_statistics()));
    }

    fn on_accept_client_blob_request(&mut self, client: ClientId, table: TableIdent) {
        info!("Responding to blob request for {table} from {client}");
        if let Some(blob) = get_blob(&self.world, table) {
            let kind = MessageKind::BlobResponse(blob);
            self.node.send(client, kind);
        } else {
            let kind = MessageKind::ServerSays(format!("Failed to serialize table: {:?}", table));
            self.node.send(client, kind);
        }
    }

    fn on_accept_client_blob_request_all(&mut self, client: ClientId) {
        let blobs: Result<Vec<Blob>, BaryError> = TableIdent::all()
            .map(|table| get_blob(&self.world, table).ok_or(BaryError::FailedToSerialize(table)))
            .collect();

        match blobs {
            Ok(blobs) => {
                let kind = MessageKind::MultiBlobResponse(
                    self.world.spawner.next(),
                    self.world.ticks,
                    blobs,
                );
                self.node.send(client, kind);
            }
            Err(err) => {
                let kind = MessageKind::ServerSays(format!("Failed to serialize table: {:?}", err));
                self.node.send(client, kind);
            }
        }
    }

    fn on_client_connect(&mut self, id: ClientId) {
        info!("Client {id} connected");
        self.node.send(id, MessageKind::WhoGoesThere);
    }

    fn on_client_disconnect(&mut self, id: ClientId, reason: DisconnectReason) {
        info!("Client {id} disconnected: {reason}");
        if let Some(username) = self.usernames.remove(&id) {
            self.node
                .broadcast(MessageKind::PlayerDisconnected(username));
        }
    }
}

impl Application for HeadlessServerApp {
    fn update(&mut self) {
        if !self.update_timer.tick() {
            return;
        }

        if self.md5_sync_timer.tick() {
            self.send_md5_sync_packet();
        }

        for msg in self.node.update() {
            match msg {
                ServerEvent::Connected(id) => {
                    self.on_client_connect(id);
                }
                ServerEvent::Disconnected(id, reason) => {
                    self.on_client_disconnect(id, reason);
                }
                ServerEvent::Message(msg) => {
                    self.on_accept_message(msg);
                }
            }
        }

        for _ in 0..self.tick_rate {
            update_world(&mut self.world);
        }

        self.send_driver_packet();
        self.send_tlm_server_info();
        self.process_transactions();
    }

    fn draw(&mut self) {
        if !self.print_timer.tick() {
            return;
        }

        debug!("Running da server.");
        debug!("Ticks:  {}", self.world.ticks);
        debug!(
            "Update: {:0.2} / {:0.2} Hz",
            self.update_timer.actual_rate(),
            self.update_timer.nominal_rate()
        );
        debug!(
            "Print:  {:0.2} / {:0.2} Hz",
            self.print_timer.actual_rate(),
            self.print_timer.nominal_rate()
        );
    }

    fn should_exit(&self) -> bool {
        false
    }
}
