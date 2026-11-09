use bary_core::prelude::{Components, Ent, ParamValue};
use bary_sim::{Player, SimulationEvent, WorldDelta};
use enum_discriminant::discriminant;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{Blob, ClientId, SyncFrame};

#[discriminant(u8)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerResponse {
    Pong,
    Ping,
    Ack,
    Nack,
    Unsupported(String),
    ServerSays(String),
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
        speed: u32,
        deltas: Vec<WorldDelta>,
        players: Components<Player>,
    },
    /// General server info sent at 1 Hz
    Info {
        grids: usize,
        planets: usize,
        thrusters: usize,
        parts: usize,
        blueprints: usize,
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

impl ServerResponse {
    pub fn id(&self) -> u8 {
        self.discriminant()
    }
}
