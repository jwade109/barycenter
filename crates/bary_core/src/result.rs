use crate::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound(Ent),
    BadPartName,
    BadBlueprint,
    FailedToSaveBlueprint,
    FailedToSaveGrid,
    SaveAlreadyExists,
    NoPrimaryComputer,
    GridSpaceOccupied,
    NoPartsAt(PartCoord),
    NoPartsInLayer(PartLayer),
    NoInvSlot(usize),
    NoInvAt(PartCoord),
    PartHasNoInv(Ent),
    ZeroPipeExtent,
    SameInvSlot(Ent, usize),
    NoPartWithName(String),
    NoProtoWithName(String),
    NoChunk,
    NoTile,
    TileAlreadyExists,
    PlayerNotDriving(Ent),
    NoPlayerNamed(String),
    PlayerAlreadyExists(String),
    IoError(String),
    SerdeYaml(String),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
    BincodeError,
    FailedToSerialize(TableIdent),
}

impl std::error::Error for BaryError {}

pub type BaryResult<E> = Result<E, BaryError>;

impl std::fmt::Display for BaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for BaryError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(format!("{:?}", value))
    }
}

impl From<serde_yaml::Error> for BaryError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::SerdeYaml(format!("{:?}", value))
    }
}

impl From<toml::ser::Error> for BaryError {
    fn from(value: toml::ser::Error) -> Self {
        Self::TomlSer(value)
    }
}

impl From<toml::de::Error> for BaryError {
    fn from(value: toml::de::Error) -> Self {
        Self::TomlDe(value)
    }
}
