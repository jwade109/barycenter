use crate::ClientTelemetry;
use bary_core::prelude::{ParamValue, TableIdent};
use bary_sim::WorldDelta;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerRequest {
    Ping,
    Introduction {
        username: String,
    },
    // sent by a client who wants to join the simulation world
    // as an embodied player.
    RequestPlayerId,
    Rebroadcast(String),
    SetSimSpeed(u32),
    Ack,
    Nack,
    /// request from the client for table blobs
    DownloadTable(TableIdent),
    /// information sent consistently by each client
    ClientTelemetry(ClientTelemetry),
    /// request by the client to perform a certain [`WorldDelta`]
    RequestDelta(WorldDelta),
    /// request for a list of available save files to load
    ListAvailableSaves,
    /// directive for the server to load a new save file
    LoadSave(PathBuf),
    /// directive for the server to send paginated world data
    BeginWorldDownload,
    /// commands the server to save the world to disk
    SaveWorld,
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
