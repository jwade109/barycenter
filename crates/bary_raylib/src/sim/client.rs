use crate::{AppearingText, Assets, KeybindsSet, SceneGraph, SoundEffects, load_keybinds};
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

    pub camera: Camera,
    pub target_camera: Camera,

    // for docking
    pub offset: PartCoord,
    pub rotation: Rotation,
}

impl FreeFlying {
    pub fn new(camera: Camera, follow_vehicle: Option<Ent>) -> Self {
        Self {
            follow_vehicle,
            lock_rotation: false,
            hovered_vehicle: None,
            selected_vehicles: Vec::new(),
            waypoint_widget: None,
            hovered_chunk: None,
            offset: PartCoord::ZERO,
            rotation: Rotation::East,
            camera,
            target_camera: camera,
        }
    }

    pub fn at_zero() -> Self {
        Self::new(
            Camera {
                zoom: 0.1,
                ..Camera::default()
            },
            None,
        )
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

/// Information that doesn't need to be synchronized across
/// clients in multiplayer.
pub struct FrontEnd {
    pub player_id: Option<Ent>,
    pub username: String,
    pub ticks: u64,
    pub chat: Chat,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub scene_graph: SceneGraph,
    pub input: InputState,
    pub alt_mode: bool,
    pub show_debug_text: bool,
    pub show_keybinds: bool,
    pub sounds: SoundEffects,
    pub assets: Assets,
    pub keybinds: KeybindsSet,
    pub spawner: EntitySpawner,
    pub transmissions: Components<ReceivedTransmission>,
    pub animations: Components<AppearingText>,
}

pub type ClientSpecificInfo = FrontEnd;

impl FrontEnd {
    pub fn new(username: String, assets: Assets) -> Self {
        let spawner = EntitySpawner::default();

        let animations = Components::default();

        let keybinds = load_keybinds("assets/real_keybinds.yaml");

        Self {
            player_id: None,
            username,
            ticks: 0,
            chat: Chat::default(),
            mouse_screen_position: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            scene_graph: SceneGraph::MainMenu,
            input: InputState::default(),
            alt_mode: false,
            show_debug_text: false,
            show_keybinds: false,
            sounds: SoundEffects::new(),
            spawner,
            transmissions: Components::default(),
            animations,
            assets,
            keybinds,
        }
    }

    pub fn update(&mut self) {
        self.animations.retain(|_, anim| anim.should_remain());
    }

    pub fn toggle_debug_text(&mut self) {
        self.show_debug_text ^= true;
        self.spawn_animation("Toggled debug view", true);
    }

    pub fn toggle_alt_mode(&mut self) {
        self.alt_mode ^= true;
        self.spawn_animation("Toggled alt mode", true);
    }

    pub fn toggle_show_keybinds(&mut self) {
        self.show_keybinds ^= true;
        self.spawn_animation("Toggled show keybinds", true);
    }

    pub fn leave_editor(&mut self) -> bool {
        let SceneGraph::Editor(editor) = &self.scene_graph else {
            return false;
        };
        self.scene_graph = SceneGraph::Free(FreeFlying::new(editor.camera, Some(editor.vehicle)));
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
            SceneGraph::Piloting(p) => Some(p.ownship),
            _ => None,
        }
    }

    pub fn spawn_animation(&mut self, anim: impl Into<String>, is_debug: bool) {
        let id = self.spawner.spawn();
        let anim = AppearingText::new(anim.into(), self.assets.ui_font(), is_debug);
        self.animations.spawn(id, anim);
    }

    pub fn is_satisfied(&self, id: &str) -> bool {
        self.keybinds.is_satisfied(&self.input, id)
    }
}
