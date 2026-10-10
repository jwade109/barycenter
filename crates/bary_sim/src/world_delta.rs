use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::TerrainMaterial;
use bary_parts::BlueprintId;
use serde::{Deserialize, Serialize};

use crate::GlobalTileIndex;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WorldDelta {
    ClearAll,
    Ping(DVec2),
    SpawnShipAt(String, BlueprintId, Isometry2d),
    FastForwardTo(u64),
    SetWaypoint {
        grid_id: Ent,
        waypoint: Isometry2d,
    },
    SetWaypointByName {
        name: String,
        waypoint: Isometry2d,
    },
    DespawnGrid(Ent),
    InsertPart {
        grid_id: Ent,
        name: String,
        coord: PartCoord,
        rotation: Rotation,
        layer: PartLayer,
    },
    DestroyPartAt {
        loc: GridLocation,
        layer: Option<PartLayer>,
    },
    SetSourceItem {
        grid_id: Ent,
        coord: PartCoord,
        item: Item,
    },
    InsertPipe {
        grid_id: Ent,
        src: PartCoord,
        dst: PartCoord,
    },
    SetRecipe {
        grid_id: Ent,
        coord: PartCoord,
        recipe: RecipeListing,
    },
    SpawnAsteroid {
        iso: Isometry2d,
        radius: f32,
        seed: u32,
    },
    SpawnPlanet {
        iso: Isometry2d,
        radius: f64,
    },
    Explode(GridLocation),
    ToggleTracking(Ent),
    RemoveTerrainTile {
        asteroid: Ent,
        tile: GlobalTileIndex,
    },
    AddTerrainTile {
        asteroid: Ent,
        tile: GlobalTileIndex,
        material: TerrainMaterial,
    },
    FullyRevealTerrainTile {
        asteroid: Ent,
        tile: GlobalTileIndex,
    },
    GoToAsteroid {
        grid_id: Ent,
        ast_id: Ent,
    },
    SetAnchored(Ent, bool),
    SpawnPlayer(String, Isometry2d),
    PlayerPilotingEnterGrid {
        player_id: Ent,
        grid_id: Ent,
    },
    PlayerPilotingExitGrid(Ent),
    SetPlayerPosition(Ent, Isometry2d),
    SetPlayerCursorPosition(Ent, Option<Vec2>),
    MergeGrids(Ent, Ent, PartCoord, Rotation),
    RenameGrid(Ent, String),
    SetGridBlueprint(Ent, Option<BlueprintId>),
    SetMachineState(Ent, bool),
    SetThrusterState(Ent, bool),
}
