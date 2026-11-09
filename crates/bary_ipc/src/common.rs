use bary_core::prelude::{Components, Ent, ParamValue};
use bary_sim::{Player, SimulationEvent, WorldDelta};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::Blob;

#[deprecated]
pub type MessageQueue<T> = Arc<SegQueue<T>>;

#[deprecated]
pub fn new_message_queue<T>() -> MessageQueue<T> {
    Arc::new(SegQueue::new())
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId({})", self.0)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientTelemetry {
    pub ticks: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SyncFrame {
    pub next_id: Ent,
    pub tick: u64,
    pub grids: u128,
    pub inventories: u128,
    pub thrusters: u128,
}

/// clone of [`renet::NetworkInfo`]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct NetworkInfo {
    pub rtt: f64,
    pub packet_loss: f64,
    pub bytes_sent_per_second: f64,
    pub bytes_received_per_second: f64,
}

impl From<renet::NetworkInfo> for NetworkInfo {
    fn from(value: renet::NetworkInfo) -> Self {
        Self {
            rtt: value.rtt,
            packet_loss: value.packet_loss,
            bytes_received_per_second: value.bytes_received_per_second,
            bytes_sent_per_second: value.bytes_sent_per_second,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ServerStatistics {
    pub clients: Vec<(u64, NetworkInfo)>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerMessage {
    Pong,
    Ping,
    Ack,
    Nack,
    Unsupported(String),
    ServerSays(String),
    ServerStatistics(ServerStatistics),
    /// broadcasted to all clients when the server loads a new world
    HaveNewSave,
    /// server response containing a single blob
    BlobResponse(Blob),
    /// server response containing multiple blobs and the current tick
    MultiBlobResponse(Ent, u64, Vec<Blob>),
    /// hashes of a few key tables to indicate desyncs
    SyncFrame(SyncFrame),
    /// driver packet sent to each client each tick
    Driver {
        ticks: u64,
        deltas: Vec<WorldDelta>,
        players: Components<Player>,
    },
    /// Indicates to client that the download is finished
    TransactionComplete,
    /// sent to a client when they've been kicked
    Kicked,
    /// challenge sent by server for client to provide their username
    WhoGoesThere,
    /// sent by server to client to indicate they have successfully connected
    TakeYourHatOff,
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

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
