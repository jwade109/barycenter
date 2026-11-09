use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Duration;

use bary_input::{InputHandler, InputState};
use rdev::Key;
use rdev::Key::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
enum EventTrigger {
    OnClick,
    IsPressed,
    OnRelease,
}

use EventTrigger::*;

fn is_subset_of(a: &[Key], b: &[Key]) -> bool {
    let a: HashSet<Key> = a.iter().cloned().collect();
    let b: HashSet<Key> = b.iter().cloned().collect();
    a.is_subset(&b)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Keybind {
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

    fn is_conflicted_with(&self, other: &Self) -> bool {
        self.primary == other.primary
            && self.trigger == other.trigger
            && (is_subset_of(&self.secondary, &other.secondary)
                || is_subset_of(&other.secondary, &self.secondary))
    }

    fn is_satisfied(&self, input: &InputState) -> bool {
        match self.trigger {
            OnClick => self.is_just_pressed(input),
            IsPressed => self.is_pressed(input),
            OnRelease => self.is_released(input),
        }
    }

    fn all_secondary_clicked(&self, input: &InputState) -> bool {
        self.secondary.iter().all(|k| input.is_key_pressed(*k))
    }

    fn is_pressed(&self, input: &InputState) -> bool {
        input.is_key_pressed(self.primary) && self.all_secondary_clicked(input)
    }

    fn is_just_pressed(&self, input: &InputState) -> bool {
        input.just_pressed_debounced(self.primary) && self.all_secondary_clicked(input)
    }

    fn is_released(&self, input: &InputState) -> bool {
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
struct KeybindsSet<T: Ord> {
    bindings: BTreeMap<T, Keybind>,
}

fn load_keybinds(path: impl AsRef<Path>) -> KeybindsSet<String> {
    let str = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&str).unwrap()
}

use serde::{Deserialize, Serialize};

fn main() {
    let binds = load_keybinds("assets/new_keybinds.yaml");

    let mut input_handler = InputHandler::new();

    for (action, keybind) in &binds.bindings {
        println!("{action:?} -> {keybind}");

        for (a, ka) in &binds.bindings {
            if action == a {
                continue;
            }
            if keybind.is_conflicted_with(ka) {
                println!("Keybind collision: {action:?} ({keybind}) and {a:?} ({ka})")
            }
        }
    }

    println!("=======");

    let mut frame_no = 0;

    loop {
        let input = input_handler.update(true);

        for (action, keybind) in &binds.bindings {
            if keybind.is_satisfied(&input) {
                println!("{frame_no}: {action:?} -> {keybind}");
            }
        }

        frame_no += 1;

        std::thread::sleep(Duration::from_millis(4));
    }
}
