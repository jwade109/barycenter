use bary_core::prelude::BaryResult;
use bary_input::*;
use early_returns::some_or_continue;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Hash, PartialOrd, Ord)]
pub enum InputAction {
    // piloting
    DriveForward,
    DriveLeft,
    DriveBackward,
    DriveRight,
    ContextDependent,
    // free
    EnterEditor,
    ToggleFollow,
    // camera
    CameraForward,
    CameraLeft,
    CameraBackward,
    CameraRight,
    CameraRotateLeft,
    CameraRotateRight,
    CameraZoomIn,
    CameraZoomOut,
    // time controls
    SpeedUp,
    SlowDown,
    TogglePause,
    // editor
    EditorDelete,
    EditorContext,
    EditorLeave,
    ShowDebugInfo,
    ShowInventoryInfo,
    // console
    ConsoleCloseMenu,
    // main menu
    CloseMainMenu,
    // debug
    DebugRecord,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct KeyMapping(HashMap<rdev::Key, InputAction>);

pub fn load_keymap_from_file(path: &str) -> BaryResult<KeyMapping> {
    let contents = std::fs::read_to_string(path)?;
    load_keymap_from_string(&contents)
}

pub fn load_keymap_from_string(contents: &str) -> BaryResult<KeyMapping> {
    let mut km = KeyMapping::default();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        let (a, b) = some_or_continue!(line.split_once(" "));
        println!("{} {}", a, b);
        let key: rdev::Key = serde_yaml::from_str(a)?;
        let action: InputAction = serde_yaml::from_str(b)?;
        km.set_mapping(key, action);
    }
    Ok(km)
}

impl KeyMapping {
    pub fn new(map: impl Into<HashMap<rdev::Key, InputAction>>) -> Self {
        Self(map.into())
    }

    pub fn map(&self, key: rdev::Key) -> Option<InputAction> {
        self.0.get(&key).copied()
    }

    pub fn set_mapping(&mut self, key: rdev::Key, action: InputAction) {
        self.0.insert(key, action);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&rdev::Key, &InputAction)> {
        self.0.iter()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ActionSet {
    currently_pressed: BTreeSet<InputAction>,
    // just_pressed: HashSet<T>,
    just_pressed_debounced: BTreeSet<InputAction>,
    just_released: BTreeSet<InputAction>,
}

fn apply_keymapping(mapping: &KeyMapping, keys: impl Iterator<Item = KB>) -> BTreeSet<InputAction> {
    keys.filter_map(|e| {
        if let KB::Key(e) = e {
            mapping.map(e)
        } else {
            None
        }
    })
    .collect()
}

impl ActionSet {
    pub fn new(input: &InputState, map: &KeyMapping) -> Self {
        Self {
            currently_pressed: apply_keymapping(map, input.get_currently_pressed()),
            // just_pressed: apply_keymapping(map, input.get_just_pressed()),
            just_pressed_debounced: apply_keymapping(map, input.get_just_pressed_debounced()),
            just_released: apply_keymapping(map, input.get_just_released()),
        }
    }

    pub fn is_active(&self, key: InputAction) -> bool {
        let key = key;
        self.currently_pressed.contains(&key)
    }

    pub fn just_triggered(&self, key: InputAction) -> bool {
        let key = key;
        self.just_pressed_debounced.contains(&key)
    }

    pub fn just_released(&self, key: InputAction) -> bool {
        let key = key;
        self.just_released.contains(&key)
    }

    // pub fn just_triggered(&self, key: T) -> bool {
    //     let key = key.into();
    //     self.just_pressed.contains(&key)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_sets() {
        use rdev::Key::*;

        // define a driving action set -
        // this would be in a config file somewhere
        let driving = KeyMapping::new([
            (KeyW, InputAction::DriveForward),
            (KeyA, InputAction::DriveLeft),
            (KeyS, InputAction::DriveBackward),
            (KeyD, InputAction::DriveRight),
        ]);

        let mut input = InputState::default();

        // ==================================================
        // frame 0 -- KeyD is just pushed
        // ==================================================

        input.set_pressed(KeyD);

        let actions = ActionSet::new(&input, &driving);

        assert!(input.just_pressed(KeyD));

        assert!(actions.just_triggered(InputAction::DriveRight));
        assert!(!actions.just_triggered(InputAction::DriveBackward));
        assert!(!actions.just_triggered(InputAction::DriveLeft));
        assert!(!actions.just_triggered(InputAction::DriveForward));

        assert!(input.is_key_pressed(KeyD));

        assert!(actions.is_active(InputAction::DriveRight));
        assert!(!actions.is_active(InputAction::DriveBackward));
        assert!(!actions.is_active(InputAction::DriveLeft));
        assert!(!actions.is_active(InputAction::DriveForward));

        input.on_frame_boundary();

        // ==================================================
        // frame 1 -- nothing happens. KeyD is still held down
        // ==================================================

        let actions = ActionSet::new(&input, &driving);

        assert!(!input.just_pressed(KeyD));
        assert!(input.is_key_pressed(KeyD));

        // nothing has just been triggered
        assert!(!actions.just_triggered(InputAction::DriveRight));
        assert!(!actions.just_triggered(InputAction::DriveBackward));
        assert!(!actions.just_triggered(InputAction::DriveLeft));
        assert!(!actions.just_triggered(InputAction::DriveForward));

        // only drive right is still active
        assert!(actions.is_active(InputAction::DriveRight));
        assert!(!actions.is_active(InputAction::DriveBackward));
        assert!(!actions.is_active(InputAction::DriveLeft));
        assert!(!actions.is_active(InputAction::DriveForward));

        input.on_frame_boundary();

        // ==================================================
        // frame 2 -- KeyD released, F and S pressed
        // ==================================================

        input.set_released(KeyD);
        input.set_pressed(KeyF);
        input.set_pressed(KeyS);
        input.set_pressed(KeyW);

        assert!(input.just_released(KeyD));

        assert!(input.just_pressed(KeyF));
        assert!(input.just_pressed(KeyS));
        assert!(input.just_pressed(KeyW));

        let actions = ActionSet::new(&input, &driving);

        // S and W trigger Forwards and Backwards
        assert!(!actions.just_triggered(InputAction::DriveRight));
        assert!(actions.just_triggered(InputAction::DriveBackward));
        assert!(!actions.just_triggered(InputAction::DriveLeft));
        assert!(actions.just_triggered(InputAction::DriveForward));

        // only those two are active
        assert!(!actions.is_active(InputAction::DriveRight));
        assert!(actions.is_active(InputAction::DriveBackward));
        assert!(!actions.is_active(InputAction::DriveLeft));
        assert!(actions.is_active(InputAction::DriveForward));

        // releasing D releases Right
        assert!(actions.just_released(InputAction::DriveRight));
        assert!(!actions.just_released(InputAction::DriveBackward));
        assert!(!actions.just_released(InputAction::DriveLeft));
        assert!(!actions.just_released(InputAction::DriveForward));
    }

    #[test]
    fn load_keymap() {
        let contents = "\
            KeyW DriveForward\n\
            KeyA DriveLeft\n\
            KeyS DriveBackward\n\
            KeyD DriveRight\n";

        let km = load_keymap_from_string(contents);

        dbg!(km);
    }
}
