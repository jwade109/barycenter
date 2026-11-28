pub use rdev::Button;
pub use rdev::Key;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum KB {
    Key(Key),
    Button(Button),
}

#[derive(Debug, Default, Clone)]
pub struct InputState {
    currently_pressed: HashSet<KB>,
    just_pressed: HashSet<KB>,
    just_pressed_debounced: HashSet<KB>,
    just_released: HashSet<KB>,
    events: Vec<rdev::Event>,
}

impl From<Key> for KB {
    fn from(value: Key) -> Self {
        KB::Key(value)
    }
}

impl From<Button> for KB {
    fn from(value: Button) -> Self {
        KB::Button(value)
    }
}

impl InputState {
    pub fn process_rdev_event(&mut self, event: &rdev::Event, focused: bool) {
        if let rdev::EventType::KeyPress(k) = event.event_type {
            if focused {
                self.set_pressed(k);
                self.events.push(event.clone());
            }
        } else if let rdev::EventType::KeyRelease(k) = event.event_type {
            self.set_released(k);
            self.events.push(event.clone());
        } else if let rdev::EventType::ButtonPress(mb) = event.event_type {
            if focused {
                self.set_pressed(mb);
                self.events.push(event.clone());
            }
        } else if let rdev::EventType::ButtonRelease(mb) = event.event_type {
            self.set_released(mb);
            self.events.push(event.clone());
        } else {
            if focused {
                self.events.push(event.clone());
            }
        }
    }

    pub fn is_key_pressed(&self, key: impl Into<KB>) -> bool {
        let key = key.into();
        self.currently_pressed.contains(&key)
    }

    pub fn just_pressed_debounced(&self, key: impl Into<KB>) -> bool {
        let key = key.into();
        self.just_pressed_debounced.contains(&key)
    }

    pub fn just_released(&self, key: impl Into<KB>) -> bool {
        let key = key.into();
        self.just_released.contains(&key)
    }

    pub fn just_pressed(&self, key: impl Into<KB>) -> bool {
        let key = key.into();
        self.just_pressed.contains(&key)
    }

    pub fn set_pressed(&mut self, key: impl Into<KB>) {
        let key = key.into();
        if !self.currently_pressed.contains(&key) {
            self.just_pressed_debounced.insert(key);
        }
        self.currently_pressed.insert(key);
        self.just_pressed.insert(key);
    }

    pub fn set_released(&mut self, key: impl Into<KB>) {
        let key = key.into();
        if self.currently_pressed.contains(&key) {
            self.currently_pressed.remove(&key);
            self.just_released.insert(key);
        }
    }

    pub fn on_frame_boundary(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_pressed_debounced.clear();
        self.events.clear();
    }

    pub fn get_currently_pressed(&self) -> impl Iterator<Item = KB> {
        self.currently_pressed.iter().copied()
    }

    pub fn get_just_pressed(&self) -> impl Iterator<Item = KB> {
        self.just_pressed.iter().copied()
    }

    pub fn get_just_pressed_debounced(&self) -> impl Iterator<Item = KB> {
        self.just_pressed_debounced.iter().copied()
    }

    pub fn get_just_released(&self) -> impl Iterator<Item = KB> {
        self.just_released.iter().copied()
    }

    pub fn events(&self) -> impl Iterator<Item = &rdev::Event> {
        self.events.iter()
    }
}
