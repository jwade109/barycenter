use enum_iterator::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Sequence, PartialOrd, Ord)]
pub enum TableIdent {
    Blueprints,
    Grids,
    Protos,
    Parts,
    Thrusters,
    Computers,
    Asteroids,
    Chunks,
    Tiles,
    Inventories,
    Machines,
    Pipes,
    Lights,
    Excavators,
    Players,
    Planets,
}

impl TableIdent {
    pub fn all() -> impl Iterator<Item = Self> {
        all::<Self>()
    }

    pub fn next(&self) -> Option<Self> {
        next::<Self>(self)
    }
}

impl std::fmt::Display for TableIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
