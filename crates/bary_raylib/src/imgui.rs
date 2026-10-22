use crate::render::*;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::*;
use bary_input::*;
use bary_sim::*;
use early_returns::*;
use raylib::prelude::*;

#[derive(Debug, Copy, Clone)]
pub enum LayoutDirection {
    Down,
    Right,
}

#[derive(Clone, Debug)]
pub struct TextArea {
    pub id: i64,
    pub origin: IVec2,
    pub dims: IVec2,
    pub text: String,
    pub is_hot: bool,
    pub is_pressed: bool,
    pub is_just_pressed: bool,
    pub is_just_released: bool,
}

pub struct ImGui {
    pub id_counter: i64,
    pub screen: Vec2,
    pub mouse_pos: Option<Vec2>,
    pub input: InputState,
    pub active: i64,
    pub layouts: Vec<Layout>,
}

pub struct Layout {
    pub id: i64,
    pub origin: IVec2,
    pub dims: IVec2,
    pub text_areas: Vec<TextArea>,
}

pub struct LayoutHandle<'a> {
    id: i64,
    text_areas: Vec<TextArea>,
    origin: IVec2,
    dir: LayoutDirection,
    dims: IVec2,
    padding: i32,
    child_gap: i32,
    next_pos: IVec2,
    gui: &'a mut ImGui,
}

fn ui_bounds(origin: IVec2, dims: IVec2) -> AABB {
    let br = origin + dims;
    AABB::from_arbitrary(origin.as_vec2(), br.as_vec2())
}

impl<'a> LayoutHandle<'a> {
    pub fn button(&mut self, text: impl Into<String>) {
        let id = self.gui.next_id_counter();
        let dims = IVec2::new(155, 40);
        let origin = self.next_pos;
        let aabb = ui_bounds(origin, dims);
        let is_hot = self
            .gui
            .mouse_pos
            .map(|p| aabb.contains(p))
            .unwrap_or(false);
        let is_pressed = is_hot && self.gui.input.is_key_pressed(rdev::Button::Left);
        let is_just_pressed = is_hot && self.gui.input.just_pressed_debounced(rdev::Button::Left);
        let is_just_released = is_hot && self.gui.input.just_released(rdev::Button::Left);

        match self.dir {
            LayoutDirection::Down => {
                self.dims.x = self.dims.x.max(dims.x + self.padding * 2);
                self.dims.y += dims.y;
                self.next_pos += IVec2::new(0, dims.y + self.child_gap);
            }
            LayoutDirection::Right => {
                self.dims.y = self.dims.y.max(dims.y + self.padding * 2);
                self.dims.x += dims.x;
                self.next_pos += IVec2::new(dims.x + self.child_gap, 0);
            }
        }

        if !self.text_areas.is_empty() {
            match self.dir {
                LayoutDirection::Down => self.dims.y += self.child_gap,
                LayoutDirection::Right => self.dims.x += self.child_gap,
            }
        }

        if is_hot {
            self.gui.active = id;
        }

        let button = TextArea {
            id,
            origin,
            dims,
            text: text.into(),
            is_hot,
            is_pressed,
            is_just_pressed,
            is_just_released,
        };

        self.text_areas.push(button);
    }
}

impl<'a> std::ops::Drop for LayoutHandle<'a> {
    fn drop(&mut self) {
        let aabb = ui_bounds(self.origin, self.dims);
        let mp = self.gui.mouse_pos.unwrap_or(Vec2::NAN);
        let is_hot = aabb.contains(mp);

        if is_hot && self.gui.active < self.id {
            self.gui.active = self.id;
        }

        self.gui.layouts.push(Layout {
            id: self.id,
            origin: self.origin,
            dims: self.dims,
            text_areas: self.text_areas.clone(),
        });
    }
}

impl ImGui {
    pub fn new(screen: Vec2, mouse_pos: Option<Vec2>, input: InputState) -> Self {
        let is_on_screen = if let Some(mp) = mouse_pos {
            mp.x >= 0.0 && mp.y >= 0.0 && mp.x <= screen.x && mp.y <= screen.y
        } else {
            false
        };

        let active = if is_on_screen { 0 } else { -1 };

        Self {
            id_counter: 1,
            screen,
            mouse_pos,
            input,
            active,
            layouts: Vec::new(),
        }
    }

    fn next_id_counter(&mut self) -> i64 {
        let ret = self.id_counter;
        self.id_counter += 1;
        ret
    }

    pub fn layout<'a>(&'a mut self, pos: IVec2, dir: LayoutDirection) -> LayoutHandle<'a> {
        let padding = 7;
        LayoutHandle {
            id: self.next_id_counter(),
            dir,
            padding,
            child_gap: 4,
            origin: pos,
            dims: IVec2::splat(padding * 2),
            next_pos: pos + IVec2::splat(padding),
            gui: self,
            text_areas: Vec::new(),
        }
    }

    pub fn active_id(&self) -> i64 {
        self.active
    }

    pub fn is_hovering_gui(&self) -> bool {
        self.active > 0
    }
}

pub fn imgui_all_parts_in_layer(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    world: &World,
    sounds: &mut SoundEffects,
) {
    let hovered_proto = get_hovered_prototype(client, world);
    let editor = some_or_return!(client.scene_graph.editor_mut());
    let layer = some_or_return!(editor.layer);
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let height = d.get_render_height();

    let bottom_left = IVec2::new(50, height - 50);
    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;

    let font = d.get_font_default();

    let mut y = bottom_left.y;

    for (proto_id, proto) in world.prototypes.iter() {
        if proto.layer != layer {
            continue;
        }
        let is_hovered = Some(*proto_id) == hovered_proto;
        let is_selected = Some(*proto_id) == editor.prototype_id;
        y -= box_height + padding;

        let xc = bottom_left.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);

        let alpha = 0.96;

        let color = if is_selected {
            Color::ORANGE
        } else if is_hovered {
            Color::TEAL
        } else {
            Color::DARKSLATEGRAY
        };

        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        d.draw_rectangle(bottom_left.x, y, box_width, box_height, color.alpha(alpha));
        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            if editor.prototype_id != Some(*proto_id) {
                editor.prototype_id = Some(*proto_id);
                sounds.push(SoundEffect::PickLayer);
            }
        }
        draw_text_centered_weak(
            d,
            &font,
            &proto.name,
            glam_to_raylib(center),
            font_size,
            Color::WHITE,
        );
    }
}

pub fn imgui_editor_layer_indicator(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let editor = some_or_return!(client.scene_graph.editor_mut());
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let boxes = [
        (PartLayer::Exterior, Color::WHITE),
        (PartLayer::Structural, Color::GRAY),
        (PartLayer::Plumbing, Color::PURPLE),
        (PartLayer::Internal, Color::ORANGE),
    ];

    let width = d.get_render_width();
    let height = d.get_render_height();

    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;
    let bottom_right = IVec2::new(width - 50, height - 50);
    let dims = IVec2::new(
        box_width,
        boxes.len() as i32 * box_height + padding * (boxes.len() as i32 - 1),
    );
    let origin = bottom_right - dims;
    let bottom_left = bottom_right - IVec2::X * dims.x;

    let mut y = origin.y;

    let font = d.get_font_default();

    for (layer, color) in boxes {
        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);
        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let text = format!("{:?}", layer);
        let is_focused = Some(layer) == editor.layer || editor.layer.is_none();
        let alpha = if is_focused { 1.0 } else { 0.2 };
        d.draw_rectangle(origin.x, y, box_width, box_height, color.alpha(alpha));

        draw_text_centered_weak(
            d,
            &font,
            &text,
            glam_to_raylib(center),
            font_size,
            Color::WHITE,
        );

        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);

            if client.input.just_pressed_debounced(rdev::Button::Left) {
                if editor.layer != Some(layer) {
                    editor.layer = Some(layer);
                    sounds.push(SoundEffect::PickLayer);
                }
            }
        }

        y += box_height + padding;
    }
}

pub fn lame_old_imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    world: &World,
    sounds: &mut SoundEffects,
) {
    imgui_editor_layer_indicator(d, client, sounds);
    imgui_all_parts_in_layer(d, client, world, sounds);
}
