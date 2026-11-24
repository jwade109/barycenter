use bary_core::prelude::*;
use bary_ipc::*;
use bary_sim::*;
use log::*;
use log::{debug, info, warn};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::{make_grid_telemetry, make_world_telemetry};

fn table_to_blob<T: Serialize>(entities: &Components<T>) -> Option<BTreeMap<Ent, Vec<u8>>> {
    let mut ret = BTreeMap::new();
    for (id, entity) in entities.iter() {
        let bytes = postcard::to_allocvec(entity).ok()?;
        ret.insert(*id, bytes);
    }
    Some(ret)
}

fn get_blob(world: &World, ident: TableIdent) -> Option<Blob> {
    let payload = match ident {
        TableIdent::Blueprints => table_to_blob(&world.blueprints),
        TableIdent::Grids => table_to_blob(&world.grids),
        TableIdent::Protos => table_to_blob(&world.prototypes),
        TableIdent::Parts => table_to_blob(&world.parts),
        TableIdent::Thrusters => table_to_blob(&world.thrusters),
        TableIdent::Computers => table_to_blob(&world.computers),
        TableIdent::Chunks => table_to_blob(&world.terrain_chunks),
        TableIdent::Tiles => table_to_blob(&world.terrain_tiles),
        TableIdent::Inventories => table_to_blob(&world.inventories),
        TableIdent::Machines => table_to_blob(&world.machines),
        TableIdent::Asteroids => table_to_blob(&world.asteroids),
        TableIdent::Pipes => table_to_blob(&world.pipes),
        TableIdent::Lights => table_to_blob(&world.lights),
        TableIdent::Excavators => table_to_blob(&world.excavators),
        TableIdent::Players => table_to_blob(&world.players),
        TableIdent::Planets => table_to_blob(&world.planets),
    }?;
    Some(Blob::new(ident, payload))
}

struct PaginatedTransaction {
    has_sent_header: bool,
    entities: Vec<EntityData>,
}

impl PaginatedTransaction {
    fn from_world(world: &World) -> Self {
        let mut entities = Vec::new();
        let all: Vec<_> = TableIdent::all().collect();
        for ident in &all {
            let blob = get_blob(world, *ident).expect("This is bad");
            let table = blob.table();
            for (id, data) in blob.consume() {
                let data = EntityData { table, id, data };
                entities.push(data);
            }
        }

        info!("Transaction contains {} entities", entities.len());

        Self {
            has_sent_header: false,
            entities,
        }
    }
}

pub fn sync_frame_from_world(world: &World) -> SyncFrame {
    let grid_bytes = postcard::to_allocvec(&world.grids).unwrap();
    let grid_hash = u128::from_be_bytes(md5::compute(&grid_bytes).0);
    let inv_bytes = postcard::to_allocvec(&world.inventories).unwrap();
    let inv_hash = u128::from_be_bytes(md5::compute(&inv_bytes).0);
    let thr_bytes = postcard::to_allocvec(&world.thrusters).unwrap();
    let thr_hash = u128::from_be_bytes(md5::compute(&thr_bytes).0);
    SyncFrame {
        tick: world.ticks,
        next_id: world.spawner.next(),
        grids: grid_hash,
        inventories: inv_hash,
        thrusters: thr_hash,
    }
}

#[derive(Debug, Default)]
struct ClientInfo {
    username: Option<String>,
    allocated_player_id: Option<Ent>,
}

pub struct BaryServer {
    world: World,
    save_file: Option<PathBuf>,
    save_files_dir: PathBuf,
    client_telemetry: HashMap<ClientId, ClientTelemetry>,
    client_info: HashMap<ClientId, ClientInfo>,
    tick_rate: u32,
    node: ServerNode,
    update_timer: WallTimer,
    queued_deltas: Vec<WorldDelta>,
    transactions: HashMap<ClientId, PaginatedTransaction>,
}

fn list_save_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut ret = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        ret.push(entry.path());
    }
    Ok(ret)
}

impl BaryServer {
    pub fn new(save_files_dir: impl Into<PathBuf>, port: u16) -> BaryResult<Self> {
        info!("Creating a new server node on port {port}");

        let node = ServerNode::new(port);

        let world = World::empty();

        let save_files_dir = save_files_dir.into();

        for save in list_save_files(&save_files_dir)? {
            info!("Found save file: {}", save.display());
        }

        Ok(Self {
            world,
            save_file: None,
            save_files_dir,
            client_telemetry: HashMap::new(),
            client_info: HashMap::new(),
            tick_rate: 1,
            node,
            update_timer: WallTimer::hz(TICKS_PER_SECOND as u32),
            queued_deltas: Vec::new(),
            transactions: HashMap::new(),
        })
    }

    pub fn ticks(&self) -> u64 {
        self.world.ticks
    }

    pub fn save_file(&self) -> Option<&Path> {
        self.save_file.as_ref().map(|p| p.as_path())
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn client_telemetry(&self) -> &HashMap<ClientId, ClientTelemetry> {
        &self.client_telemetry
    }

    fn send_driver_packet(&mut self) {
        let deltas = std::mem::take(&mut self.queued_deltas);
        let msg = ServerResponse::Driver {
            ticks: self.world.ticks,
            speed: self.tick_rate,
            deltas,
        };
        self.node.broadcast(msg);
    }

    fn send_param_update(&mut self, id: ClientId) {
        let params = ServerResponse::Parameters(self.world.parameters.inner().clone());
        self.node.send(id, params);
    }

    fn ack(&mut self, id: ClientId) {
        self.node.send(id, ServerResponse::Ack);
    }

    fn nack(&mut self, id: ClientId, reason: impl Into<String>) {
        let reason = reason.into();
        error!("Nack to {id}: {reason}");
        self.node.send(id, ServerResponse::Nack(reason));
    }

    #[allow(unused)]
    fn send_md5_sync_packet(&mut self) {
        let frame = sync_frame_from_world(&self.world);
        let msg = ServerResponse::SyncFrame(frame);
        self.node.broadcast(msg);
    }

    fn process_transactions(&mut self) {
        for (client, tr) in &mut self.transactions {
            if !tr.has_sent_header {
                let name = self
                    .save_file
                    .as_ref()
                    .map(|s| s.display().to_string())
                    .unwrap_or("<anonymous>".into());
                let fragment = DownloadFragment::Start {
                    name,
                    n_entities: tr.entities.len(),
                    next_entity_id: self.world.spawner.next(),
                    ticks: self.world.ticks,
                };
                let msg = ServerResponse::Download(fragment);
                self.node.send(*client, msg);
                tr.has_sent_header = true;
            }
            for _ in 0..500 {
                if let Some(data) = tr.entities.pop() {
                    info!("[{client}] Sending entity {}", data);
                    let fragment = DownloadFragment::Entity(data);
                    let msg = ServerResponse::Download(fragment);
                    if !self.node.send(*client, msg) {
                        error!("Failed to send entity data");
                    }
                } else {
                    info!("Finished transaction!");
                    let fragment = DownloadFragment::End(0);
                    let msg = ServerResponse::Download(fragment);
                    self.node.send(*client, msg);
                    break;
                }
            }
        }

        self.transactions
            .retain(|_client, tr| !tr.entities.is_empty());
    }

    fn on_request_player_id(&mut self, id: ClientId) {
        let Some(username) = self.client_info.get(&id).and_then(|s| s.username.clone()) else {
            return self.nack(id, "No info available for this client");
        };

        let player_id = if let Some(player_id) = Player::find_by_name(&self.world, &username) {
            let delta = WorldDelta::ConnectPlayer(player_id);
            match apply_delta(&mut self.world, delta.clone()) {
                Ok(()) => {
                    self.queued_deltas.push(delta);
                }
                Err(e) => {
                    error!("Failed to connect player: {e:?}");
                }
            }
            info!("Found existing player for username {username}");
            player_id
        } else {
            info!("Spawning a new player for username {username}");
            let iso = Isometry2d::ZERO;
            let delta = WorldDelta::SpawnPlayer(username.clone(), iso);
            self.queued_deltas.push(delta);
            let id = Player::spawn(&mut self.world, username.clone(), iso, Isometry2d::ZERO);
            self.send_driver_packet();
            id
        };

        let client_info = self.client_info.entry(id).or_default();
        client_info.allocated_player_id = Some(player_id);

        let reply = ServerResponse::YourPlayerIdIs(player_id);
        info!("For client {}, player ID is {}", id, player_id);
        self.node.send(id, reply);
    }

    fn on_request_list_available_saves(&mut self, id: ClientId) {
        match list_save_files(&self.save_files_dir) {
            Ok(saves) => {
                let response = ServerResponse::SaveFiles(saves);
                self.node.send(id, response);
            }
            Err(e) => {
                self.nack(id, format!("Filesystem error: {e:?}"));
            }
        }
    }

    fn on_accept_message(&mut self, client_id: ClientId, msg: ServerRequest) {
        debug!("Got a command: {:?}", msg);

        match msg {
            ServerRequest::Introduction { username } => {
                self.on_accept_introduction(client_id, username);
            }
            ServerRequest::ClientTelemetry(tlm) => {
                self.on_accept_client_tlm(client_id, tlm);
            }
            ServerRequest::Ping => {
                self.on_accept_ping(client_id);
            }
            ServerRequest::Rebroadcast(s) => {
                self.on_accept_rebroadcast(client_id, s);
            }
            ServerRequest::SetSimSpeed(s) => {
                self.on_accept_set_sim_speed(client_id, s);
            }
            ServerRequest::DownloadTable(table) => {
                self.on_accept_table_dl_request(client_id, table);
            }
            ServerRequest::RequestDelta(delta) => {
                self.on_accept_world_delta(client_id, delta);
            }
            ServerRequest::LoadSave(path) => {
                self.load_world(path);
            }
            ServerRequest::BeginWorldDownload => {
                self.begin_async_world_download(client_id);
            }
            ServerRequest::SetParameters(params) => {
                for (k, v) in params {
                    self.world.parameters.insert(k, v);
                }
                self.ack(client_id);
            }
            ServerRequest::RequestParams => {
                self.send_param_update(client_id);
            }
            ServerRequest::RequestPlayerId => {
                self.on_request_player_id(client_id);
            }
            ServerRequest::Ack => {
                info!("Client {client_id} has ACKed");
            }
            ServerRequest::Nack => {
                info!("Client {client_id} has NACKed");
            }
            ServerRequest::ListAvailableSaves => {
                self.on_request_list_available_saves(client_id);
            }
            ServerRequest::SaveWorld => {
                self.on_request_save_world(client_id);
            }
        }
    }

    fn on_accept_world_delta(&mut self, id: ClientId, delta: WorldDelta) {
        if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
            self.nack(id, format!("Failed to apply delta: {e}"));
        } else {
            debug!("Successfully applied delta: {delta:?}");
            self.queued_deltas.push(delta.clone());
            self.send_driver_packet();
        }
    }

    fn on_accept_introduction(&mut self, id: ClientId, username: String) {
        let info = self.client_info.entry(id).or_default();
        if info.username.is_some() {
            return self.nack(id, "Client has reintroduced themselves");
        }
        info!("Client {id} has introduced themselves as {username}");
        info.username = Some(username.clone());
        self.node
            .broadcast(ServerResponse::PlayerConnected(username));
    }

    fn begin_async_world_download(&mut self, id: ClientId) {
        if self.transactions.contains_key(&id) {
            return self.nack(id, "Transaction already underway");
        }
        let tr = PaginatedTransaction::from_world(&self.world);
        self.transactions.insert(id, tr);
        info!("Started async world download for {id}");
    }

    pub fn load_world(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        match load_world(path) {
            Ok(world) => {
                self.world = world;
                self.save_file = Some(path.to_path_buf());
                self.tick_rate = 1;
                info!("Loaded {}", path.display());
                self.node.broadcast(ServerResponse::HaveNewSave);
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
    }

    pub fn on_request_save_world(&mut self, id: ClientId) {
        let now = chrono::offset::Local::now();
        let save_path = format!("saves/world-{}/", now.format("%Y-%m-%d-%H-%M-%S%.3f"));
        match save_world(&save_path, &self.world, false) {
            Ok(()) => self.ack(id),
            Err(e) => {
                self.nack(id, format!("Failed to save world to {save_path}: {e:?}"));
            }
        }
    }

    fn on_accept_ping(&mut self, id: ClientId) -> Option<()> {
        self.node.send(id, ServerResponse::Pong);
        Some(())
    }

    fn on_accept_client_tlm(&mut self, id: ClientId, tlm: ClientTelemetry) {
        self.client_telemetry.insert(id, tlm.clone());
        self.node
            .broadcast(ServerResponse::ClientTelemetry(id, tlm));
    }

    fn on_accept_rebroadcast(&mut self, id: ClientId, s: String) -> Option<()> {
        self.node.broadcast(ServerResponse::ChatMessage(id, s));
        Some(())
    }

    fn on_accept_set_sim_speed(&mut self, id: ClientId, speed: u32) -> Option<()> {
        self.tick_rate = speed;
        self.node.broadcast(ServerResponse::NewSimSpeed(speed));
        Some(())
    }

    fn send_blob(&mut self, client: ClientId, blob: Blob) {
        debug!("Sending blob {blob} to {client}");
        let table = blob.table();
        for (id, data) in blob.consume() {
            let fragment = DownloadFragment::Entity(EntityData { table, id, data });
            let kind = ServerResponse::Download(fragment);
            self.node.send(client, kind);
        }
    }

    fn on_accept_table_dl_request(&mut self, client: ClientId, table: TableIdent) {
        info!("Responding to blob request for {table} from {client}");
        if let Some(blob) = get_blob(&self.world, table) {
            self.send_blob(client, blob);
        } else {
            let msg = ServerResponse::ServerSays(format!("Failed to serialize table: {:?}", table));
            self.node.send(client, msg);
        }
    }

    fn on_client_connect(&mut self, id: ClientId, addr: SocketAddr) {
        info!("Client {id} connected");
        self.node.send(id, ServerResponse::PleaseSendUsername);
        self.node
            .broadcast(ServerResponse::ClientConnected(id, addr));
    }

    fn on_client_disconnect(&mut self, id: ClientId, addr: SocketAddr) {
        info!("Client {id} disconnected");
        self.node
            .broadcast(ServerResponse::ClientDisconnected(id, addr));

        let Some(info) = self.client_info.remove(&id) else {
            return;
        };

        if let Some(username) = info.username {
            self.node
                .broadcast(ServerResponse::PlayerDisconnected(username));
        }

        if let Some(id) = info.allocated_player_id {
            let delta = WorldDelta::DisconnectPlayer(id);
            match apply_delta(&mut self.world, delta.clone()) {
                Ok(()) => {
                    self.queued_deltas.push(delta);
                }
                Err(e) => {
                    error!("Failed to disconnect player: {e:?}");
                }
            }
        }
    }

    fn send_info_packets(&mut self) {
        let tlm = make_world_telemetry(&self.world);
        let tlm = ServerTelemetry::World(tlm);
        self.node.broadcast(ServerResponse::Telemetry(tlm));
        let tlm = make_grid_telemetry(&self.world);
        let tlm = ServerTelemetry::Grids(tlm);
        self.node.broadcast(ServerResponse::Telemetry(tlm));

        let tlm = ClientsTelemetry {
            clients: self
                .node
                .clients()
                .map(|(id, addr)| {
                    let info = self.client_info.get(id);
                    let username = info.and_then(|c| c.username.clone());
                    let assigned_player = info.and_then(|c| c.allocated_player_id);

                    ClientPlayerAssignment {
                        client_addr: *addr,
                        id: *id,
                        username,
                        assigned_player,
                    }
                })
                .collect(),
        };
        let tlm = ServerTelemetry::Clients(tlm);
        self.node.broadcast(ServerResponse::Telemetry(tlm));
    }

    pub fn update(&mut self) {
        if !self.update_timer.tick() {
            return;
        }

        for msg in self.node.update() {
            match msg {
                ServerEvent::Connected(id, addr) => {
                    self.on_client_connect(id, addr);
                }
                ServerEvent::Disconnected(id, addr) => {
                    self.on_client_disconnect(id, addr);
                }
                ServerEvent::Message(id, msg) => {
                    self.on_accept_message(id, msg);
                }
            }
        }

        if self.transactions.is_empty() {
            for _ in 0..self.tick_rate {
                let res = update_world(&mut self.world);
                self.send_info_packets();
                self.send_driver_packet();

                for event in res.events.take() {
                    let msg = ServerResponse::Simulation(event);
                    self.node.broadcast(msg);
                }
            }
        }

        self.process_transactions();
    }
}
