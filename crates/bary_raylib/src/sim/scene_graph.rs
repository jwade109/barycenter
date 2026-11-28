use crate::{EditorState, FreeFlying};
use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::{Camera, World};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scene {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Pilot(Ent),
    Editor(Ent),
    Free,
}

#[derive(Debug)]
pub struct PartFocusState {
    pub part_id: Ent,
    pub command_prompt: String,
}

impl PartFocusState {
    pub fn new(part_id: Ent) -> Self {
        Self {
            part_id,
            command_prompt: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct LockedState {
    pub player_id: Ent,
    pub zoom: f32,
}

impl LockedState {
    pub fn new(player_id: Ent) -> Self {
        Self {
            player_id,
            zoom: 100.0,
        }
    }

    pub fn step(&mut self, input: &InputState) {
        let scale = 1.15;
        for event in input.events() {
            if let rdev::EventType::Wheel {
                delta_x: _,
                delta_y,
            } = event.event_type
            {
                if delta_y > 0 {
                    self.zoom *= scale;
                } else if delta_y < 0 {
                    self.zoom /= scale;
                }
            }
        }
    }

    pub fn get_camera(&self, world: &World) -> Option<Camera> {
        let player = world.players.get(self.player_id)?;
        Some(Camera::new(player.pos, self.zoom))
    }
}

#[derive(Debug)]
pub enum SceneGraph {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Editor(EditorState),
    Free(FreeFlying),
    LockedToPlayer(LockedState),
}

impl SceneGraph {
    pub fn should_draw_world(&self) -> bool {
        match self {
            SceneGraph::MainMenu => true,
            SceneGraph::ListSaves => true,
            SceneGraph::Kicked => true,
            SceneGraph::Settings => true,
            SceneGraph::Editor(..) => true,
            SceneGraph::Free(..) => true,
            SceneGraph::LockedToPlayer(..) => true,
        }
    }

    pub fn handle_navigation(&mut self, nav: Scene) {
        *self = match (&self, nav) {
            (_, Scene::ListSaves) => Self::ListSaves,
            (_, Scene::Editor(id)) => Self::Editor(EditorState::new(id, Vec2::ZERO)),
            (Self::Editor(state), Scene::Free) => {
                Self::Free(FreeFlying::new(state.camera, Some(state.vehicle)))
            }
            (_, Scene::Pilot(id)) => Self::LockedToPlayer(LockedState::new(id)),
            (SceneGraph::LockedToPlayer(_pilot), Scene::Free) => {
                Self::Free(FreeFlying::new(Camera::default(), None))
            }
            (_, Scene::Free) => Self::Free(FreeFlying::at_zero()),
            (_, Scene::MainMenu) => Self::MainMenu,
            (_, Scene::Kicked) => Self::Kicked,
            (_, Scene::Settings) => Self::Settings,
        }
    }

    pub fn look_at(&mut self, id: Ent) -> bool {
        if let Self::Free(fly) = self {
            let ret = fly.follow_vehicle != Some(id);
            fly.follow_vehicle = Some(id);
            ret
        } else {
            false
        }
    }

    pub fn is_real_view(&self) -> bool {
        match self {
            Self::Free(_) => true,
            _ => false,
        }
    }

    pub fn free(&self) -> Option<&FreeFlying> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    pub fn free_mut(&mut self) -> Option<&mut FreeFlying> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    pub fn editor(&self) -> Option<&EditorState> {
        match self {
            Self::Editor(e) => Some(e),
            _ => None,
        }
    }

    pub fn editor_mut(&mut self) -> Option<&mut EditorState> {
        match self {
            Self::Editor(e) => Some(e),
            _ => None,
        }
    }

    pub fn camera(&self, world: &World) -> Option<Camera> {
        match self {
            SceneGraph::MainMenu => None,
            SceneGraph::ListSaves => None,
            SceneGraph::Kicked => None,
            SceneGraph::Settings => None,
            SceneGraph::Editor(editor) => Some(editor.camera),
            SceneGraph::Free(free) => Some(free.camera),
            SceneGraph::LockedToPlayer(locked) => locked.get_camera(world),
        }
    }
}
