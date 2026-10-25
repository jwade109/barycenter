use bary_core::prelude::*;
use bary_input::InputState;
use bary_parts::BlueprintId;
use bary_sim::{World, WorldDelta};
use early_returns::*;

#[derive(Debug, Clone, Copy)]
pub enum EditorCommand {
    SetLayer(PartLayer),
    SetProto(Ent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorState {
    pub vehicle: Ent,
    pub target_coord: PartCoord,
    pub actual_offset: Vec2,
    pub camera_rotation: Rotation,
    pub prototype_id: Option<Ent>,
    pub part_rotation: Rotation,
    pub layer: Option<PartLayer>,
    pub select_start: Option<PartCoord>,
    pub hovered: Option<PartCoord>,

    pub vehicle_name_field: Option<(EditableText, bool)>,
}

impl EditorState {
    pub fn handle_command(&mut self, cmd: EditorCommand) {
        match cmd {
            EditorCommand::SetLayer(layer) => {
                self.layer = Some(layer);
            }
            EditorCommand::SetProto(id) => {
                self.prototype_id = Some(id);
            }
        }
    }
}

fn editor_offset_moves_with_wasd(input: &InputState, offset: &mut PartCoord) {
    if input.is_key_pressed(rdev::Key::ControlLeft) {
        return;
    }

    let speed = 4;

    if input.just_pressed(rdev::Key::KeyS) {
        offset.0.y -= speed;
    }
    if input.just_pressed(rdev::Key::KeyW) {
        offset.0.y += speed;
    }
    if input.just_pressed(rdev::Key::KeyD) {
        offset.0.x += speed;
    }
    if input.just_pressed(rdev::Key::KeyA) {
        offset.0.x -= speed;
    }
}

impl EditorState {
    pub fn new(grid_id: Ent, offset: Vec2) -> Self {
        EditorState {
            vehicle: grid_id,
            target_coord: PartCoord::ZERO,
            actual_offset: offset,
            camera_rotation: Rotation::East,
            prototype_id: None,
            part_rotation: Rotation::East,
            layer: Some(PartLayer::Internal),
            select_start: None,
            hovered: None,
            vehicle_name_field: None,
        }
    }

    #[must_use]
    pub fn handle_keys(&mut self, input: &InputState, world: &World) -> Option<WorldDelta> {
        let mut ret = None;

        if input.just_pressed_debounced(rdev::Key::Return) && self.vehicle_name_field.is_some() {
            if let Some((field, is_name)) = self.vehicle_name_field.take() {
                let delta = if is_name {
                    WorldDelta::RenameGrid(self.vehicle, field.contents().to_string())
                } else {
                    let bp = BlueprintId(field.contents().to_string(), 0);
                    WorldDelta::SetGridBlueprint(self.vehicle, Some(bp))
                };
                ret = Some(delta);
                self.vehicle_name_field = None;
            }
        }

        if let Some((field, _is_name)) = &mut self.vehicle_name_field {
            field.handle_keys(input);
        } else {
            if input.just_pressed_debounced(rdev::Key::KeyR) {
                self.rotate_part();
            }

            if input.just_pressed_debounced(rdev::Key::KeyQ) {
                self.pipette(world);
            }

            if input.just_pressed_debounced(rdev::Key::KeyE) {
                self.next_layer(true);
            }

            editor_offset_moves_with_wasd(input, &mut self.target_coord);
        }

        if input.just_pressed_debounced(rdev::Key::KeyN) && self.vehicle_name_field.is_none() {
            self.vehicle_name_field = Some((EditableText::empty(), true));
        }

        if input.just_pressed_debounced(rdev::Key::KeyB) && self.vehicle_name_field.is_none() {
            self.vehicle_name_field = Some((EditableText::empty(), false));
        }

        if input.just_pressed_debounced(rdev::Key::Escape) && self.vehicle_name_field.is_some() {
            self.vehicle_name_field = None;
        }

        ret
    }

    pub fn next_layer(&mut self, is_up: bool) {
        self.layer = if is_up {
            enum_iterator::next_cycle(&self.layer)
        } else {
            enum_iterator::previous_cycle(&self.layer)
        };
    }

    pub fn rotate_part(&mut self) {
        if self.prototype_id.is_some() {
            self.part_rotation = self.part_rotation.next();
        } else {
            self.camera_rotation = self.camera_rotation.next();
        }
    }

    pub fn pipette(&mut self, world: &World) {
        if self.prototype_id.is_some() {
            self.prototype_id = None;
            return;
        }
        self.prototype_id = None;

        let coord = some_or_return!(self.hovered);

        let grid = ok_or_return!(world.grids.try_get(self.vehicle));
        let Some(occ) = grid.get_parts_at(coord) else {
            self.layer = None;
            return;
        };

        // use the focus layer to pipette if it's available; otherwise, use the top one
        let part_id = if let Some(layer) = self.layer {
            occ.at_layer(layer)
        } else {
            occ.top()
        };

        let part_id = some_or_return!(part_id);
        let part = ok_or_return!(world.parts.try_get(part_id));

        self.prototype_id = Some(part.prototype);
        self.part_rotation = part.region.rot();
        self.layer = Some(part.layer);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditableText {
    inner: String,
}

impl EditableText {
    pub fn empty() -> Self {
        Self {
            inner: String::new(),
        }
    }

    pub fn contents(&self) -> &str {
        &self.inner
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn on_backspace(&mut self) {
        self.inner.pop();
    }

    pub fn handle_keys(&mut self, input: &InputState) {
        for event in input.events() {
            if let rdev::EventType::KeyPress(k) = &event.event_type {
                match k {
                    rdev::Key::Backspace => self.on_backspace(),
                    rdev::Key::Return => self.clear(),
                    _ => {
                        if let Some(s) = &event.name {
                            if s.is_ascii() {
                                self.inner += s;
                            }
                        }
                    }
                }
            }
        }
    }
}
