use crate::{
    event_bus::EventBus,
    sounds::SoundKind,
    tweens::{AnimationStates, Tween},
};
use bary_core::prelude::{AABB, Components, Ent, lerp};
use bary_input::InputState;
use glam::DVec2;
use rend::*;

pub struct Ui<'a> {
    sounds: EventBus<SoundKind>,
    mouse_pos: DVec2,
    input: InputState,
    cmd: &'a mut RenderCommands,
    anim: &'a AnimationStates,
    id: usize,
    pos: DVec2,
    padding: f64,
    font_size: f64,
}

pub struct ButtonResponse {
    pub is_hovered: bool,
    pub is_clicked: bool,
    pub extent: DVec2,
}

impl<'a> Ui<'a> {
    pub fn new(
        mouse_pos: DVec2,
        input: InputState,
        cmd: &'a mut RenderCommands,
        anim: &'a AnimationStates,
    ) -> Self {
        Self {
            sounds: EventBus::new(),
            mouse_pos,
            input,
            cmd,
            anim,
            id: 0,
            pos: DVec2::new(30.0, 300.0),
            padding: 15.0,
            font_size: 18.0,
        }
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn sounds(&self) -> impl Iterator<Item = &SoundKind> {
        self.sounds.iter()
    }

    pub fn mouse_pos(&self) -> DVec2 {
        self.mouse_pos
    }

    pub fn fonts(&self) -> &Components<FontInfo> {
        &self.cmd.fonts
    }

    pub fn label(&mut self, text: impl Into<String>) -> ButtonResponse {
        self.id += 1;

        let text = text.into();
        let padding = DVec2::splat(15.0);
        let text_offset = DVec2::Y * self.font_size;
        let text_origin = self.pos + padding + text_offset;

        let extent = self
            .cmd
            .text(text_origin, text.clone())
            .size(self.font_size)
            .color(Color::WHITE)
            .extent();

        let full_extent = extent + padding * 2.0;
        let rect_origin = self.pos - extent.y * DVec2::Y;
        let aabb = AABB::from_arbitrary(self.pos.as_vec2(), (self.pos + full_extent).as_vec2());
        let contains = aabb.contains(self.mouse_pos.as_vec2());

        let alpha = contains as u8 as f64 * 0.2 + 0.9;

        self.cmd
            .rect(self.pos)
            .dims(full_extent)
            .color(Color::hsl(0.1, 0.3, 0.2, 1.0))
            .z(0.52);

        self.cmd.text_with_shadow(
            text_origin,
            (-2.0, -2.0),
            text,
            self.font_size,
            Color::WHITE,
            Color::BLACK.alpha(0.7),
        );

        self.pos.y += full_extent.y + self.padding;

        let is_clicked = self.input.just_pressed(rdev::Button::Left) && contains;

        ButtonResponse {
            is_hovered: contains,
            is_clicked,
            extent: full_extent,
        }
    }

    pub fn button(&mut self, text: impl Into<String>, color: Color) -> ButtonResponse {
        self.id += 1;

        let text = text.into();
        let padding = DVec2::splat(15.0);
        let text_offset = DVec2::Y * self.font_size;
        let text_origin = self.pos + padding + text_offset;

        let extent = self
            .cmd
            .text(text_origin, text.clone())
            .size(self.font_size)
            .color(Color::WHITE)
            .extent();

        let full_extent = extent + padding * 2.0;
        let rect_origin = self.pos - extent.y * DVec2::Y;
        let aabb = AABB::from_arbitrary(self.pos.as_vec2(), (self.pos + full_extent).as_vec2());
        let contains = aabb.contains(self.mouse_pos.as_vec2());

        let t = self
            .anim
            .anim(("button", self.id), Tween::Exponential, 0.1, contains);
        let alpha = lerp(0.7, 1.0, t as f32) as f64;

        let extra_extent = DVec2::new(200.0 * t, 0.0);

        let expanded_extent = full_extent + extra_extent;
        let expanded_origin = self.pos.with_y(self.pos.y - extra_extent.y);

        let alpha = contains as u8 as f64 * 0.2 + 0.9;

        self.cmd
            .rect(self.pos)
            .dims(expanded_extent)
            .color(color.alpha(alpha))
            .z(0.52);

        self.cmd.text_with_shadow(
            text_origin,
            (-2.0, -2.0),
            text,
            self.font_size,
            Color::WHITE,
            Color::BLACK.alpha(0.7),
        );

        self.pos.y += full_extent.y + self.padding;

        let is_clicked = self.input.just_pressed(rdev::Button::Left) && contains;

        if is_clicked {
            self.sounds.enqueue(SoundKind::ButtonUp);
        }

        ButtonResponse {
            is_hovered: contains,
            is_clicked,
            extent: full_extent,
        }
    }

    pub fn checkbox(&mut self, text: impl Into<String>, state: &mut bool) -> ButtonResponse {
        let dims = DVec2::new(200.0, 50.0);
        let color = if *state { Color::RED } else { Color::BLUE };
        self.cmd.rect(self.pos).dims(dims);
        let aabb = AABB::from_arbitrary(self.pos.as_vec2(), (self.pos + dims).as_vec2());
        self.pos += (dims.y + self.padding) * DVec2::Y;

        let is_hovered = aabb.contains(self.mouse_pos.as_vec2());
        let is_clicked = is_hovered && self.input.just_pressed(rdev::Button::Left);

        if is_clicked {
            self.sounds.enqueue(SoundKind::Crossword);
            *state ^= true;
        }

        ButtonResponse {
            is_hovered,
            is_clicked,
            extent: dims,
        }
    }
}
