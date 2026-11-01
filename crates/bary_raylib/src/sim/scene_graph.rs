use crate::{EditorState, FreeFlying};
use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::Camera;
use rdev::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scene {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Pilot(Ent),
    Editor(Ent),
    Free,
    Part(Ent),
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
pub struct PilotingState {
    pub ownship: Ent,
    pub target: Option<Ent>,
    pub camera: Camera,
    pub target_rotation: f64,
    pub target_zoom: f32,
}

impl PilotingState {
    pub fn new(ownship: Ent) -> Self {
        Self {
            ownship,
            target: None,
            camera: Camera {
                isometry: Isometry2d::ZERO,
                zoom: 0.03,
            },
            target_rotation: 0.0,
            target_zoom: 0.03,
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
                    self.target_zoom *= scale;
                } else if delta_y < 0 {
                    self.target_zoom /= scale;
                }
            }
        }

        let angular_speed = 2.5f64.to_radians();

        if input.is_key_pressed(Key::KeyQ) {
            self.target_rotation += angular_speed;
        }
        if input.is_key_pressed(Key::KeyE) {
            self.target_rotation -= angular_speed;
        }

        self.camera.isometry.rotation +=
            (self.target_rotation - self.camera.isometry.rotation) * 0.25;
        self.camera.zoom += (self.target_zoom - self.camera.zoom) * 0.25;
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
    Piloting(PilotingState),
    Part(PartFocusState),
}

impl SceneGraph {
    pub fn should_draw_world(&self) -> bool {
        match self {
            SceneGraph::MainMenu => false,
            SceneGraph::ListSaves => false,
            SceneGraph::Kicked => false,
            SceneGraph::Settings => false,
            SceneGraph::Editor(..) => true,
            SceneGraph::Free(..) => true,
            SceneGraph::Piloting(..) => true,
            SceneGraph::Part(..) => true,
        }
    }

    pub fn handle_navigation(&mut self, nav: Scene) {
        *self = match (&self, nav) {
            (_, Scene::ListSaves) => Self::ListSaves,
            (_, Scene::Editor(id)) => Self::Editor(EditorState::new(id, Vec2::ZERO)),
            (Self::Editor(state), Scene::Free) => {
                Self::Free(FreeFlying::new(state.camera, Some(state.vehicle)))
            }
            (_, Scene::Pilot(id)) => Self::Piloting(PilotingState::new(id)),
            (SceneGraph::Piloting(pilot), Scene::Free) => {
                Self::Free(FreeFlying::new(pilot.camera, Some(pilot.ownship)))
            }
            (_, Scene::Free) => Self::Free(FreeFlying::at_zero()),
            (_, Scene::MainMenu) => Self::MainMenu,
            (_, Scene::Kicked) => Self::Kicked,
            (_, Scene::Settings) => Self::Settings,
            (_, Scene::Part(id)) => Self::Part(PartFocusState::new(id)),
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

    pub fn camera(&self) -> Option<Camera> {
        match self {
            SceneGraph::MainMenu => None,
            SceneGraph::ListSaves => None,
            SceneGraph::Kicked => None,
            SceneGraph::Settings => None,
            SceneGraph::Editor(editor) => Some(editor.camera),
            SceneGraph::Free(free) => Some(free.camera),
            SceneGraph::Piloting(piloting) => Some(piloting.camera),
            SceneGraph::Part(..) => None,
        }
    }
}
