use crate::WorldDelta;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TermCmd {
    World(WorldDelta),
    Exit,
    Say(String),
    Ping,
    Clear,
    SetSimSpeed(u32),
    FindGridByName(String),

    /// print info about a particular entity table to the terminal
    PrintEntityInfo(TableIdent),

    /// print blueprint info to the terminal
    ServerDisconnect,
    ServerConnect,
    RequestServerStatistics,

    /// print save info to the terminal
    PrintSaveInfo,
    /// save the current world to disk with the given name
    SaveWorldToDisk(String, bool),
    /// list available savefiles which can be Loaded
    ListSaves,
    /// load the save with the given name
    LoadSave(String),
    /// print blob info to the terminal
    PrintBlobInfo(TableIdent),
    /// client blob requests
    ClientReqBlob(TableIdent),
    /// client requests all table blobs
    ClientReqAllBlobs,
    /// get help about the commands on this terminal
    Help,
    /// get the current working directory
    Pwd,
    /// Go to a different directory
    Cd(String),
    /// toggle terminal debug output
    ToggleDebugMode,
}
