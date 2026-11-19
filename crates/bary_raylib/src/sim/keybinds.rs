use EventTrigger::*;
use bary_factory::Item;
use bary_input::InputState;
use log::warn;
use rdev::Key;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum EventTrigger {
    OnClick,
    IsPressed,
    OnRelease,
}

fn is_subset_of(a: &[Key], b: &[Key]) -> bool {
    let a: HashSet<Key> = a.iter().cloned().collect();
    let b: HashSet<Key> = b.iter().cloned().collect();
    a.is_subset(&b)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Keybind {
    primary: Key,
    secondary: Vec<Key>,
    trigger: EventTrigger,
}

impl Keybind {
    fn new(primary: Key, secondary: impl IntoIterator<Item = Key>, trigger: EventTrigger) -> Self {
        Self {
            primary,
            secondary: secondary.into_iter().collect(),
            trigger,
        }
    }

    pub fn is_conflicted_with(&self, other: &Self) -> bool {
        self.primary == other.primary
            && self.trigger == other.trigger
            && (is_subset_of(&self.secondary, &other.secondary)
                || is_subset_of(&other.secondary, &self.secondary))
    }

    pub fn is_satisfied(&self, input: &InputState) -> bool {
        match self.trigger {
            OnClick => self.is_just_pressed(input),
            IsPressed => self.is_pressed(input),
            OnRelease => self.is_released(input),
        }
    }

    pub fn all_secondary_clicked(&self, input: &InputState) -> bool {
        self.secondary.iter().all(|k| input.is_key_pressed(*k))
    }

    pub fn is_pressed(&self, input: &InputState) -> bool {
        input.is_key_pressed(self.primary) && self.all_secondary_clicked(input)
    }

    pub fn is_just_pressed(&self, input: &InputState) -> bool {
        input.just_pressed_debounced(self.primary) && self.all_secondary_clicked(input)
    }

    pub fn is_released(&self, input: &InputState) -> bool {
        input.just_released(self.primary) && self.all_secondary_clicked(input)
    }
}

impl From<Key> for Keybind {
    fn from(value: Key) -> Self {
        Keybind::new(value, [], OnClick)
    }
}

impl From<(Key, EventTrigger)> for Keybind {
    fn from(value: (Key, EventTrigger)) -> Self {
        Keybind::new(value.0, [], value.1)
    }
}

impl std::fmt::Display for Keybind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut keys = self.secondary.clone();
        keys.push(self.primary);
        let prefix = keys
            .into_iter()
            .map(|a| format!("{a:?}"))
            .collect::<Vec<_>>()
            .join(" + ");
        write!(f, "{}", prefix)
    }
}

#[derive(Deserialize, Serialize)]
pub struct KeybindsSet {
    active_groups: BTreeSet<String>,
    bindings: BTreeMap<String, Keybind>,
}

impl KeybindsSet {
    pub fn is_satisfied(&self, input: &InputState, key: &str) -> bool {
        if let Some(kb) = self.bindings.get(key) {
            kb.is_satisfied(input)
        } else {
            warn!("{key} is not an active keybind!");
            false
        }
    }

    pub fn active_groups(&self) -> impl Iterator<Item = &String> {
        self.active_groups.iter()
    }

    pub fn set_active(&mut self, group_id: impl Into<String>) {
        self.active_groups.insert(group_id.into());
    }

    pub fn clear_active(&mut self) {
        self.active_groups.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Keybind)> {
        self.bindings.iter()
    }
}

pub fn load_keybinds(path: impl AsRef<Path>) -> KeybindsSet {
    let str = std::fs::read_to_string(path).unwrap();
    let bindings: BTreeMap<String, Keybind> = serde_yaml::from_str(&str).unwrap();
    KeybindsSet {
        active_groups: BTreeSet::new(),
        bindings,
    }
}
