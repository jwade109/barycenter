use bary_core::prelude::{Components, Ent, Isometry2d, TableIdent};
use bary_parts::PartPrototype;
use bary_sim::{Computer, Part, Player, Thruster, WorldDelta};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::{
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

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSource {
    Server,
    Client(ClientId),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub source: MessageSource,
    pub kind: MessageKind,
}

impl Message {
    pub fn new(source: MessageSource, kind: MessageKind) -> Self {
        Self { source, kind }
    }

    pub fn ack(source: MessageSource) -> Self {
        Self::new(source, MessageKind::Ack)
    }

    pub fn nack(source: MessageSource) -> Self {
        Self::new(source, MessageKind::Nack)
    }

    pub fn is_ack(&self) -> bool {
        if let MessageKind::Ack = self.kind {
            true
        } else {
            false
        }
    }

    pub fn is_nack(&self) -> bool {
        if let MessageKind::Nack = self.kind {
            true
        } else {
            false
        }
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
pub enum MessageKind {
    Pong,
    Ping,
    Introduction {
        username: String,
    },
    ClientSays(String),
    ServerSays(String),
    ChatMessage(ClientId, String),
    SetSimSpeed(u32),
    Ack,
    Nack,
    Unsupported,
    FindGridByName(String),
    Entity(Ent),
    ServerStatistics(ServerStatistics),
    PrintEntityInfo(TableIdent),
    GridInfo(Ent, String, Isometry2d),
    Proto(Ent, PartPrototype),
    Part(Ent, Part),
    Thruster(Ent, Thruster),
    Computer(Ent, Computer),
    RequestServerStatistics,
    SetWaypoint(Ent, Isometry2d),
    CameraPosition(Isometry2d),
    /// request from the client for table blobs
    ClientBlobRequest(TableIdent),
    /// client requests all blobs
    ClientBlobRequestAll,
    /// server response containing a single blob
    BlobResponse(Blob),
    /// server response containing multiple blobs and the current tick
    MultiBlobResponse(Ent, u64, Vec<Blob>),
    /// information sent consistently by each client
    ClientTelemetry(ClientTelemetry),
    /// driver packet sent to each client each tick
    Driver {
        ticks: u64,
        deltas: Vec<WorldDelta>,
        players: Components<Player>,
    },
    /// request by the client to perform a certain [`WorldDelta`]
    RequestDelta(WorldDelta),
    /// hashes of a few key tables to indicate desyncs
    SyncFrame(SyncFrame),
    /// directive for the server to load a new save file
    LoadSave(String),
    /// directive for the server to send paginated world data
    BeginAsyncWorldDownload,
    FinishWorldDownload,
    /// broadcasted to all clients when the server loads a new world
    HaveNewSave,
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
}

impl MessageKind {
    pub fn with_source(self, source: MessageSource) -> Message {
        Message { source, kind: self }
    }
}

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
