use crate::{EditorState, FreeFlying};
use bary_core::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneNavigation {
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Editor(Ent),
    Free,
    Part(Ent),
}

#[derive(Debug)]
pub enum SceneGraph {
    Loading(u32, u32, String),
    MainMenu,
    ListSaves,
    Kicked,
    Settings,
    Editor(EditorState),
    Free(FreeFlying),
    Part(Ent),
}

impl SceneGraph {
    pub fn handle_navigation(&mut self, nav: SceneNavigation) {
        *self = match nav {
            SceneNavigation::ListSaves => Self::ListSaves,
            SceneNavigation::Editor(id) => Self::Editor(EditorState::new(id, Vec2::ZERO)),
            SceneNavigation::Free => Self::Free(FreeFlying::new()),
            SceneNavigation::MainMenu => Self::MainMenu,
            SceneNavigation::Kicked => Self::Kicked,
            SceneNavigation::Settings => Self::Settings,
            SceneNavigation::Part(id) => Self::Part(id),
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
