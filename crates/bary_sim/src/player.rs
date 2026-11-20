use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub username: String,
    pub iso: Isometry2d,
}
