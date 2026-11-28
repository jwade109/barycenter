use crate::{
    AppearingText, Assets, KeybindsSet, LockedState, Raylib, SceneGraph, SoundEffects,
    load_keybinds,
};
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
    pub input_timer: WallTimer,
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
            input_timer: WallTimer::hz(20),
        }
    }

    pub fn update(&mut self) {
        self.animations.retain(|_, anim| anim.should_remain());
    }

    /// Toggles whether the debug menu is shown.
    pub fn toggle_debug_text(&mut self) {
        self.show_debug_text ^= true;
        self.spawn_animation("Toggled debug view", true);
    }

    /// Toggles whether "alt" mode is shown; shows additional debug rendering.
    pub fn toggle_alt_mode(&mut self) {
        self.alt_mode ^= true;
        self.spawn_animation("Toggled alt mode", true);
    }

    /// Toggles whether the keybinds debug menu is shown.
    pub fn toggle_show_keybinds(&mut self) {
        self.show_keybinds ^= true;
        self.spawn_animation("Toggled show keybinds", true);
    }

    fn go_to_scene_free(&mut self, camera: Camera) {
        self.spawn_animation("Free Camera", true);
        self.scene_graph = SceneGraph::Free(FreeFlying::new(camera, None))
    }

    fn go_to_player_locked(&mut self) {
        if let Some(id) = self.player_id {
            self.spawn_animation("Locked Camera", true);
            self.scene_graph = SceneGraph::LockedToPlayer(LockedState::new(id));
        } else {
            self.spawn_animation("No player to lock to", true);
        }
    }

    pub fn toggle_camera_lock(&mut self, world: &World) {
        let camera = self.scene_graph.camera(world).unwrap_or_default();
        match &self.scene_graph {
            SceneGraph::Free(..) => {
                self.go_to_player_locked();
            }
            SceneGraph::LockedToPlayer(..) => {
                self.go_to_scene_free(camera);
            }
            _ => {
                self.spawn_animation("Invalid state to toggle lock", true);
            }
        }
    }

    pub fn hot_reload_assets(&mut self, raylib: &mut Raylib) {
        self.assets = Assets::load(raylib);
        self.spawn_animation("Reloaded assets", true);
    }

    pub fn try_enter_camera_locked(&mut self) {
        if let Some(id) = self.player_id {
            self.scene_graph = SceneGraph::LockedToPlayer(LockedState::new(id));
            self.spawn_animation("Locked to Player", true);
        } else {
            self.spawn_animation("Cannot lock camera", true);
        }
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
            _ => None,
        }
    }

    pub fn spawn_animation(&mut self, anim: impl Into<String>, is_debug: bool) {
        let id = self.spawner.spawn();
        let anim = AppearingText::new(anim.into(), self.assets.ui_font(), is_debug);
        self.animations.spawn(id, anim);
    }

    pub fn go_to_world(&mut self) {
        let world_entry = if let Some(id) = self.player_id {
            SceneGraph::LockedToPlayer(LockedState::new(id))
        } else {
            SceneGraph::Free(FreeFlying::at_zero())
        };

        match self.scene_graph {
            SceneGraph::MainMenu => {
                self.scene_graph = world_entry;
            }
            SceneGraph::ListSaves => {
                self.scene_graph = world_entry;
            }
            SceneGraph::Kicked => {
                self.scene_graph = world_entry;
            }
            SceneGraph::Settings => {
                self.scene_graph = world_entry;
            }
            SceneGraph::Editor(..) => {
                // PASS
            }
            SceneGraph::Free(..) => {
                // PASS
            }
            SceneGraph::LockedToPlayer(..) => {
                // PASS
            }
        }
    }

    pub fn is_satisfied(&self, id: &str) -> bool {
        self.keybinds.is_satisfied(&self.input, id)
    }

    pub fn is_satisfied_ignoring_groups(&self, id: &str) -> bool {
        self.keybinds.is_satisfied_ignoring_groups(&self.input, id)
    }
}
