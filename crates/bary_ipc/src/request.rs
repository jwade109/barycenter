use crate::ClientTelemetry;
use bary_core::prelude::{Ent, Isometry2d, ParamValue, TableIdent};
use bary_parts::PartPrototype;
use bary_sim::{Computer, Part, Thruster, WorldDelta};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerRequest {
    Pong,
    Ping,
    Introduction {
        username: String,
    },
    ClientSays(String),
    SetSimSpeed(u32),
    Ack,
    Nack,
    Unsupported,
    FindGridByName(String),
    Entity(Ent),
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
    /// information sent consistently by each client
    ClientTelemetry(ClientTelemetry),
    /// request by the client to perform a certain [`WorldDelta`]
    RequestDelta(WorldDelta),
    /// directive for the server to load a new save file
    LoadSave(String),
    /// directive for the server to send paginated world data
    BeginAsyncWorldDownload,
    /// gets parameters
    RequestParams,
    /// sets parameters
    SetParameters(BTreeMap<String, ParamValue>),
}

impl From<WorldDelta> for ServerRequest {
    fn from(value: WorldDelta) -> Self {
        Self::RequestDelta(value)
    }
}
