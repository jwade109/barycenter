use crate::SceneGraph;
use bary_core::prelude::*;
use bary_input::*;
use bary_sim::Camera;
use bary_sim::*;

#[derive(Debug, Clone, Copy)]
pub struct TerrainSelectionInfo {
    pub asteroid: Ent,
    pub tile: GlobalTileIndex,
}

#[derive(Debug)]
pub struct FreeFlying {
    pub follow_vehicle: Option<Ent>,
    pub lock_rotation: bool,
    pub hovered_vehicle: Option<GridLocation>,
    pub selected_vehicles: Vec<GridLocation>,
    pub hovered_chunk: Option<TerrainSelectionInfo>,
    pub waypoint_widget: Option<DVec2>,

    // for docking
    pub offset: PartCoord,
    pub rotation: Rotation,
}

impl FreeFlying {
    pub fn new() -> Self {
        Self {
            follow_vehicle: None,
            lock_rotation: false,
            hovered_vehicle: None,
            selected_vehicles: Vec::new(),
            waypoint_widget: None,
            hovered_chunk: None,
            offset: PartCoord::ZERO,
            rotation: Rotation::East,
        }
    }

    pub fn docking_interface(&self) -> Option<DockingInterface> {
        if self.selected_vehicles.len() != 2 {
            return None;
        }

        let parent = *self.selected_vehicles.first()?;
        let child = *self.selected_vehicles.get(1)?;

        if parent.grid_id == child.grid_id {
            return None;
        }

        Some(DockingInterface {
            parent,
            child,
            offset: self.offset,
            rotation: self.rotation,
        })
    }
}

pub struct DockingInterface {
    pub offset: PartCoord,
    pub rotation: Rotation,
    pub parent: GridLocation,
    pub child: GridLocation,
}

/// Information that doesn't *in general* need to be synchronized across
/// clients in multiplayer.
pub struct ClientSpecificInfo {
    pub player_id: Option<Ent>,
    pub ticks: u64,
    pub tick_rate: u32,
    pub chat: Chat,
    pub camera: Camera,
    pub target_camera: Camera,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub scene_graph: SceneGraph,
    pub input: InputState,
    pub alt_mode: bool,
}

impl ClientSpecificInfo {
    pub fn new() -> Self {
        Self {
            player_id: None,
            ticks: 0,
            tick_rate: 1,
            chat: Chat::default(),
            // camera info
            camera: Camera {
                zoom: 0.1,
                ..Camera::default()
            },
            target_camera: Camera {
                zoom: 8.0,
                ..Camera::default()
            },
            mouse_screen_position: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            scene_graph: SceneGraph::MainMenu,
            input: InputState::default(),
            alt_mode: false,
        }
    }

    pub fn leave_editor(&mut self) -> bool {
        let SceneGraph::Editor(editor) = &self.scene_graph else {
            return false;
        };

        self.scene_graph = SceneGraph::Free(FreeFlying {
            follow_vehicle: Some(editor.vehicle),
            ..FreeFlying::new()
        });

        self.chat.log("Left ship editor");
        true
    }

    pub fn selected_grid_loc(&self) -> Option<GridLocation> {
        match &self.scene_graph {
            SceneGraph::Editor(e) => {
                let coord = e.select_start?;
                Some(GridLocation {
                    grid_id: e.vehicle,
                    coord,
                })
            }
            SceneGraph::Free(f) => f.selected_vehicles.first().cloned(),
            _ => None,
        }
    }

    pub fn hovered_grid_loc(&self) -> Option<GridLocation> {
        match &self.scene_graph {
            SceneGraph::Editor(e) => {
                let coord = e.hovered?;
                Some(GridLocation {
                    grid_id: e.vehicle,
                    coord,
                })
            }
            SceneGraph::Free(f) => f.hovered_vehicle,
            _ => None,
        }
    }

    pub fn focused_grid_id(&self) -> Option<Ent> {
        match &self.scene_graph {
            SceneGraph::Editor(e) => Some(e.vehicle),
            SceneGraph::Free(f) => f.selected_vehicles.first().map(|s| s.grid_id),
            _ => None,
        }
    }
}
