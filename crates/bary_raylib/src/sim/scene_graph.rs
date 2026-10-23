use crate::{EditorState, FreeFlying};
use bary_core::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scene {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
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
pub enum SceneGraph {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Editor(EditorState),
    Free(FreeFlying),
    Part(PartFocusState),
}

impl SceneGraph {
    pub fn handle_navigation(&mut self, nav: Scene) {
        *self = match (&self, nav) {
            (_, Scene::ListSaves) => Self::ListSaves,
            (_, Scene::Editor(id)) => Self::Editor(EditorState::new(id, Vec2::ZERO)),
            (Self::Editor(state), Scene::Free) => {
                let free = FreeFlying {
                    follow_vehicle: Some(state.vehicle),
                    ..FreeFlying::new()
                };
                Self::Free(free)
            }
            (_, Scene::Free) => Self::Free(FreeFlying::new()),
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
}
