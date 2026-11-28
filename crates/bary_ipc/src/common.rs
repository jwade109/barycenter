use bary_core::prelude::Ent;
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{GridsTelemetry, WorldTelemetry};

#[deprecated]
pub type MessageQueue<T> = Arc<SegQueue<T>>;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(pub u64);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId({})", self.0)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientTelemetry {
    pub grids: GridsTelemetry,
    pub world: WorldTelemetry,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SyncFrame {
    pub next_id: Ent,
    pub tick: u64,
    pub grids: u128,
    pub inventories: u128,
    pub thrusters: u128,
}
