use crate::Scene;
use bary_core::prelude::*;
use bary_input::InputState;
use bary_ui::Tree;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiMessage {
    Drag(DragHandle),
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
    SetMachineState(Ent, bool),
    SetThrusterState(Ent, bool),
    SetComputerOnOff(Ent, bool),
    SetComputerDrift(Ent),
    Salt(usize),
}

impl From<Scene> for UiMessage {
    fn from(value: Scene) -> Self {
        Self::Navigate(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragHandle {
    MainMenu,
    SaveSelect,
    Docking,
    PartInfo,
}

impl From<DragHandle> for UiMessage {
    fn from(value: DragHandle) -> Self {
        Self::Drag(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct UiInteractionState {
    is_on_gui: bool,
    screen_pos: Option<Vec2>,
    hot: Option<(UiMessage, Vec2)>,
    active: Option<(UiMessage, Vec2)>,
}

#[derive(Debug, Clone, Copy)]
pub enum UiEventKind {
    Click,
    Release,
    Drag(Vec2),
}

#[derive(Debug, Clone)]
pub struct UiEvent {
    pub msg: UiMessage,
    pub pos: Vec2,
    pub kind: UiEventKind,
}

impl UiEvent {
    fn click(msg: UiMessage, pos: Vec2) -> Self {
        Self {
            msg,
            pos,
            kind: UiEventKind::Click,
        }
    }

    fn release(msg: UiMessage, pos: Vec2) -> Self {
        Self {
            msg,
            pos,
            kind: UiEventKind::Click,
        }
    }

    fn drag(msg: UiMessage, pos: Vec2, delta: Vec2) -> Self {
        Self {
            msg,
            pos,
            kind: UiEventKind::Drag(delta),
        }
    }
}

impl UiInteractionState {
    pub fn is_on_gui(&self) -> bool {
        self.is_on_gui
    }

    pub fn active(&self) -> Option<&UiMessage> {
        self.active.as_ref().map(|e| &e.0)
    }

    pub fn hot(&self) -> Option<&UiMessage> {
        self.hot.as_ref().map(|e| &e.0)
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

                let Some(onclick) = node.on_click() else {
                    continue;
                };

                let relative = p - aabb.lower();

                if contains {
                    self.hot = Some((onclick.clone(), relative));

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
                    ret = Some(UiEvent::drag(active.0.clone(), active.1, delta));
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
                if active.0 == hot.0 {
                    ret = Some(UiEvent::release(active.0.clone(), active.1));
                }
            }
            self.active = None;
        }

        ret
    }
}
