use bary_core::prelude::*;
use bary_ipc::*;
use bary_sim::*;
use log::*;
use log::{debug, info, warn};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn table_to_blob<T: Serialize>(entities: &Components<T>) -> Option<BTreeMap<Ent, Vec<u8>>> {
    let mut ret = BTreeMap::new();
    for (id, entity) in entities.iter() {
        let bytes = bincode::serialize(entity).ok()?;
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
    entities: Vec<(TableIdent, Ent, Vec<u8>)>,
    last_sent: Instant,
}

impl PaginatedTransaction {
    fn from_world(world: &World) -> Self {
        let mut entities = Vec::new();
        let all: Vec<_> = TableIdent::all().collect();
        for ident in &all {
            let blob = get_blob(world, *ident).expect("This is bad");
            let ident = blob.table();
            for (id, data) in blob.consume() {
                entities.push((ident, id, data));
            }
        }

        info!("Transaction contains {} entities", entities.len());

        Self {
            entities,
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

pub struct BaryServer {
    world: World,
    save_file: Option<PathBuf>,
    client_telemetry: HashMap<ClientId, ClientTelemetry>,
    usernames: HashMap<ClientId, String>,
    tick_rate: u32,
    node: ServerNode,
    update_timer: WallTimer,
    queued_deltas: Vec<WorldDelta>,
    transactions: HashMap<ClientId, PaginatedTransaction>,
}

impl BaryServer {
    pub fn new(port: u16, speed: u32) -> BaryResult<Self> {
        info!("Creating a new server node on port {port}");

        let node = ServerNode::new(port);

        let world = World::empty();

        Ok(Self {
            world,
            save_file: None,
            client_telemetry: HashMap::new(),
            usernames: HashMap::new(),
            tick_rate: speed,
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

    fn nack(&mut self, id: ClientId) {
        self.node.send(id, ServerResponse::Nack);
    }

    #[allow(unused)]
    fn send_md5_sync_packet(&mut self) {
        let frame = sync_frame_from_world(&self.world);
        let msg = ServerResponse::SyncFrame(frame);
        self.node.broadcast(msg);
    }

    fn process_transactions(&mut self) {
        for (client, tr) in &mut self.transactions {
            for _ in 0..100 {
                if let Some((ident, id, data)) = tr.entities.pop() {
                    info!("Sending entity {ident} {id} ({} bytes)", data.len());
                    let msg = ServerResponse::Entity(ident, id, data);
                    self.node.send(*client, msg);
                } else {
                    info!("Finished transaction!");
                    let msg = ServerResponse::TransactionComplete(0);
                    self.node.send(*client, msg);
                    break;
                }
            }
        }

        self.transactions
            .retain(|_client, tr| !tr.entities.is_empty());
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
            ServerRequest::ClientSays(s) => {
                self.on_accept_client_say(client_id, s);
            }
            ServerRequest::SetSimSpeed(s) => {
                self.on_accept_set_sim_speed(client_id, s);
            }
            ServerRequest::ClientBlobRequest(table) => {
                self.on_accept_client_blob_request(client_id, table);
            }
            ServerRequest::RequestDelta(delta) => {
                self.on_accept_world_delta(client_id, delta);
            }
            ServerRequest::LoadSave(path) => {
                self.load_world(path);
            }
            ServerRequest::BeginAsyncWorldDownload => {
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
            e => {
                let e = format!("{e:?}");
                self.on_unsupported_message(e);
            }
        }
    }

    fn on_accept_world_delta(&mut self, id: ClientId, delta: WorldDelta) {
        if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
            error!("Failed to apply delta: {e}");
            self.nack(id);
        } else {
            debug!("Successfully applied delta: {delta:?}");
            self.queued_deltas.push(delta.clone());
            self.send_driver_packet();
        }
    }

    fn on_accept_introduction(&mut self, id: ClientId, username: String) {
        if !self.usernames.contains_key(&id) {
            info!("Client {id} has introduced themselves as {username}");
            self.usernames.insert(id, username.clone());
            self.node
                .broadcast(ServerResponse::PlayerConnected(username));
            self.node.send(id, ServerResponse::TakeYourHatOff);
        } else {
            error!("Client {id} has reintroduced themselves");
            self.nack(id);
        }
    }

    fn begin_async_world_download(&mut self, id: ClientId) {
        if self.transactions.contains_key(&id) {
            error!("Transaction already underway for client {id}");
            self.nack(id);
            return;
        }
        let tr = PaginatedTransaction::from_world(&self.world);
        self.transactions.insert(id, tr);
        info!("Started async world download for {id}");
    }

    fn on_unsupported_message(&mut self, desc: String) {
        warn!("Unsupported message type!");
        self.node.broadcast(ServerResponse::Unsupported(desc));
    }

    pub fn load_world(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        warn!("Trying to load save file: {}", path.display());
        match load_world(path) {
            Ok(world) => {
                self.world = world;
                self.save_file = Some(path.to_path_buf());
                info!("Loaded {}", path.display());
                self.node.broadcast(ServerResponse::HaveNewSave);
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
    }

    fn on_accept_ping(&mut self, id: ClientId) -> Option<()> {
        self.node.send(id, ServerResponse::Pong);
        Some(())
    }

    fn ensure_client_is_connected(&mut self, id: ClientId) -> Option<()> {
        if !self.usernames.contains_key(&id) {
            error!("Client {id} is not connected!");
            self.node.send(id, ServerResponse::Nack);
            return None;
        }
        Some(())
    }

    fn on_accept_client_tlm(&mut self, id: ClientId, tlm: ClientTelemetry) {
        self.ensure_client_is_connected(id);
        self.client_telemetry.insert(id, tlm);
    }

    fn on_accept_client_say(&mut self, id: ClientId, s: String) -> Option<()> {
        self.node.broadcast(ServerResponse::ChatMessage(id, s));
        Some(())
    }

    fn on_accept_set_sim_speed(&mut self, id: ClientId, speed: u32) -> Option<()> {
        self.tick_rate = speed;
        self.node.send(id, ServerResponse::Ack);
        Some(())
    }

    fn send_blob(&mut self, client: ClientId, blob: Blob) {
        debug!("Sending blob {blob} to {client}");
        for (id, data) in blob.payload() {
            let kind = ServerResponse::Entity(blob.table(), *id, data.clone());
            self.node.send(client, kind);
        }
    }

    fn on_accept_client_blob_request(&mut self, client: ClientId, table: TableIdent) {
        info!("Responding to blob request for {table} from {client}");
        if let Some(blob) = get_blob(&self.world, table) {
            self.send_blob(client, blob);
        } else {
            let msg = ServerResponse::ServerSays(format!("Failed to serialize table: {:?}", table));
            self.node.send(client, msg);
        }
    }

    fn on_client_connect(&mut self, id: ClientId) {
        info!("Client {id} connected");
        self.node.send(id, ServerResponse::WhoGoesThere);
    }

    fn on_client_disconnect(&mut self, id: ClientId) {
        info!("Client {id} disconnected");
        if let Some(username) = self.usernames.remove(&id) {
            self.node
                .broadcast(ServerResponse::PlayerDisconnected(username));
        }
    }

    fn send_info_packets(&mut self) {
        let info = ServerResponse::Info {
            clients: self.node.client_count(),
            grids: self.world.grids.len(),
            planets: self.world.planets.len(),
            thrusters: self.world.thrusters.len(),
            parts: self.world.parts.len(),
            blueprints: self.world.blueprints.len(),
        };
        self.node.broadcast(info);

        let mut grids = BTreeMap::new();

        for (id, grid) in self.world.grids.iter() {
            let mut info = BTreeMap::new();
            for thruster_id in &grid.thrusters {
                if let Some(thruster) = self.world.thrusters.get(*thruster_id) {
                    info.insert(*thruster_id, thruster.unclamped_throttle());
                }
            }

            let mut active_thrusters = 0;

            for thruster_id in &grid.thrusters {
                let Some(thruster) = self.world.thrusters.get(*thruster_id) else {
                    continue;
                };
                if thruster.unclamped_throttle() > 0.001 {
                    active_thrusters += 1;
                }
            }

            let model = grid
                .blueprint
                .as_ref()
                .map(|bp| bp.0.clone())
                .unwrap_or_default();

            let tlm = GridTelemetry {
                name: grid.name.clone(),
                model,
                pos: grid.particle_location,
                vel: grid.velocity,
                active_thrusters,
            };

            grids.insert(*id, tlm);
        }

        let msg = ServerResponse::Grids {
            tick: self.world.ticks,
            grids,
        };
        self.node.broadcast(msg);
    }

    pub fn update(&mut self) {
        if !self.update_timer.tick() {
            return;
        }

        for msg in self.node.update() {
            match msg {
                ServerEvent::Connected(id) => {
                    self.on_client_connect(id);
                }
                ServerEvent::Disconnected(id) => {
                    self.on_client_disconnect(id);
                }
                ServerEvent::Message(id, msg) => {
                    self.on_accept_message(id, msg);
                }
            }
        }

        for _ in 0..self.tick_rate {
            update_world(&mut self.world);
            self.send_info_packets();
            self.send_driver_packet();
        }

        self.process_transactions();
    }
}
