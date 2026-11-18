use bary_core::prelude::{Ent, Isometry2d, ParamValue, TableIdent};
use bary_sim::{SimulationEvent, WorldDelta};
use enum_discriminant::discriminant;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr};

use crate::{ClientId, SyncFrame};

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

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct ServerTelemetry {
    pub n_clients: usize,
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
    /// General server info sent at 1 Hz
    Summary(WorldTelemetry),
    /// Grid-specific info
    Grids(GridsTelemetry),
    /// Server-specific info
    Server(ServerTelemetry),
    /// Thruster-specific info
    Thrusters {
        tick: u64,
        grid: Ent,
        thrusters: BTreeMap<Ent, f32>,
    },
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
}

impl ServerResponse {
    pub fn id(&self) -> u8 {
        self.discriminant()
    }
}
