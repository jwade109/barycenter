use std::collections::BTreeMap;

use crate::{EditorCommand, Scene};
use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::{TimedInstruction, WorldDelta};
use bary_ui::{NodePath, Tree};

#[derive(Debug, Clone)]
pub enum UiMessage {
    Drag(String),
    Navigate(Scene),
    Exit,
    AltMode,
    DebugText,
    SimSpeed(u32),
    DockingShiftX,
    DockingShiftY,
    DockingRotate,
    DockingActivate,
    LoadSave,
    JoinMultiplayer,
    HostMultiplayer,
    WriteSaveToDisk,
    LoadSaveFile(String),
    Salt(usize),
    Editor(EditorCommand),
    Delta(WorldDelta),
}

impl From<Scene> for UiMessage {
    fn from(value: Scene) -> Self {
        Self::Navigate(value)
    }
}

impl From<WorldDelta> for UiMessage {
    fn from(value: WorldDelta) -> Self {
        Self::Delta(value)
    }
}

impl From<EditorCommand> for UiMessage {
    fn from(value: EditorCommand) -> Self {
        Self::Editor(value)
    }
}

#[derive(Debug, Clone)]
pub struct UiRef {
    pub path: NodePath,
    pub offset: Vec2,
    pub onclick: Option<UiMessage>,
}

#[derive(Debug, Clone, Default)]
pub struct UiInteractionState {
    is_on_gui: bool,
    screen_pos: Option<Vec2>,
    hot: Option<UiRef>,
    active: Option<UiRef>,
    offsets: BTreeMap<String, Vec2>,
}

impl UiInteractionState {
    pub fn window_pos(&mut self, salt: impl Into<String>, offset: Vec2) -> Vec2 {
        *self.offsets.entry(salt.into()).or_insert(offset)
    }

    pub fn shift_offset(&mut self, salt: &str, delta: Vec2) {
        if let Some(pos) = self.offsets.get_mut(salt) {
            *pos += delta;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UiEventKind {
    Click,
    Release,
    Drag(Vec2),
}

#[derive(Debug, Clone)]
pub struct UiEvent {
    pub reference: UiRef,
    pub kind: UiEventKind,
}

impl UiEvent {
    fn click(rf: UiRef) -> Self {
        Self {
            reference: rf,
            kind: UiEventKind::Click,
        }
    }

    fn release(rf: UiRef) -> Self {
        Self {
            reference: rf,
            kind: UiEventKind::Click,
        }
    }

    fn drag(rf: UiRef, delta: Vec2) -> Self {
        Self {
            reference: rf,
            kind: UiEventKind::Drag(delta),
        }
    }
}

impl UiInteractionState {
    pub fn is_on_gui(&self) -> bool {
        self.is_on_gui
    }

    pub fn active(&self) -> Option<&UiRef> {
        self.active.as_ref()
    }

    pub fn hot(&self) -> Option<&UiRef> {
        self.hot.as_ref()
    }

    pub fn active_path(&self) -> Option<&NodePath> {
        self.active.as_ref().map(|a| &a.path)
    }

    pub fn hot_path(&self) -> Option<&NodePath> {
        self.hot.as_ref().map(|a| &a.path)
    }

    pub fn update(
        &mut self,
        ui: &Tree<UiMessage>,
        screen_pos: Option<Vec2>,
        input: &InputState,
    ) -> Option<UiEvent> {
        self.is_on_gui = false;
        self.hot = None;
        let last_pos = self.screen_pos;
        self.screen_pos = screen_pos;

        if let Some(p) = screen_pos {
            for node in ui.iter() {
                let aabb = node.aabb();
                let contains = aabb.contains(p);

                self.is_on_gui |= contains;

                let path = node.path();

                let relative = p - aabb.lower();

                if contains {
                    self.hot = Some(UiRef {
                        path: path.clone(),
                        offset: relative,
                        onclick: node.on_click().cloned(),
                    });

                    if input.just_pressed(rdev::Button::Left) {
                        self.active = self.hot.clone();
                    }
                }
            }
        }

        let mut ret = None;

        if input.is_key_pressed(rdev::Button::Left) {
            if let Some((last, (cur, active))) =
                last_pos.zip(self.screen_pos.zip(self.active.as_ref()))
            {
                if last != cur {
                    let delta = cur - last;
                    ret = Some(UiEvent::drag(active.clone(), delta));
                }
            }
        }

        // if input.just_pressed(rdev::Button::Left) {
        //     if let Some(active) = self.active {
        //         ret = Some(UiEvent::click(active.0, active.1));
        //     }
        // }

        if input.just_released(rdev::Button::Left) {
            if let Some((active, hot)) = self.active.as_ref().zip(self.hot.as_ref()) {
                if active.path == hot.path {
                    ret = Some(UiEvent::release(active.clone()));
                }
            }
            self.active = None;
        }

        ret
    }
}
