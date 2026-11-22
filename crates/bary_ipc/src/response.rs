use bary_core::prelude::{Ent, Isometry2d, ParamValue, TableIdent};
use bary_sim::{SimulationEvent, WorldDelta};
use enum_discriminant::discriminant;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr};

use crate::{ClientId, ClientTelemetry, SyncFrame};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GridTelemetry {
    pub name: String,
    pub model: String,
    pub pos: Isometry2d,
    pub vel: Isometry2d,
    pub active_thrusters: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GridsTelemetry {
    pub tick: u64,
    pub grids: BTreeMap<Ent, GridTelemetry>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientPlayerAssignment {
    pub client_addr: SocketAddr,
    pub id: ClientId,
    pub username: Option<String>,
    pub assigned_player: Option<Ent>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientsTelemetry {
    pub clients: Vec<ClientPlayerAssignment>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct WorldTelemetry {
    pub tick: u64,
    pub grids: usize,
    pub planets: usize,
    pub thrusters: usize,
    pub parts: usize,
    pub blueprints: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EntityData {
    pub table: TableIdent,
    pub id: Ent,
    pub data: Vec<u8>,
}

impl std::fmt::Display for EntityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} ({} bytes)", self.table, self.id, self.data.len())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum DownloadFragment {
    Start {
        name: String,
        n_entities: usize,
        ticks: u64,
    },
    Entity(EntityData),
    End(u64),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ThrusterTelemetry {
    pub tick: u64,
    pub grid: Ent,
    pub thrusters: BTreeMap<Ent, f32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerTelemetry {
    Clients(ClientsTelemetry),
    World(WorldTelemetry),
    Grids(GridsTelemetry),
    Thrusters(ThrusterTelemetry),
}

#[discriminant(u8)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerResponse {
    Pong,
    Ping,
    Ack,
    Nack,
    ClientConnected(ClientId, SocketAddr),
    ClientDisconnected(ClientId, SocketAddr),
    Unsupported(String),
    ServerSays(String),
    /// broadcasted to all clients when the server loads a new world
    HaveNewSave,
    /// hashes of a few key tables to indicate desyncs
    SyncFrame(SyncFrame),
    /// driver packet sent to each client each tick
    Driver {
        ticks: u64,
        speed: u32,
        deltas: Vec<WorldDelta>,
    },
    /// Any kind of telemetry from server
    Telemetry(ServerTelemetry),
    /// Part of a world download.
    Download(DownloadFragment),
    /// sent to a client when they've been kicked
    Kicked,
    /// challenge sent by server for client to provide their username
    PleaseSendUsername,
    /// sent by server to client to indicate they have successfully connected
    SuccessfullyConnected,
    /// sent by the server to clients to let them know someone has connected
    PlayerConnected(String),
    /// sent by the server to clients to let them know someone has disconnected
    PlayerDisconnected(String),
    /// broadcasted by server when a client says something
    ChatMessage(ClientId, String),
    /// propagated from world_update
    Simulation(SimulationEvent),
    /// parameters
    Parameters(BTreeMap<String, ParamValue>),
    /// new sim speed
    NewSimSpeed(u32),
    /// forwarded telemetry from other clients
    ClientTelemetry(ClientId, ClientTelemetry),
}

impl ServerResponse {
    pub fn id(&self) -> u8 {
        self.discriminant()
    }
}
