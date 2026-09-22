use crate::{
    event_bus::EventBus,
    tweens::{AnimationStates, Tween},
};
use bary_core::prelude::{AABB, Components, Ent, lerp};
use bary_input::InputState;
use glam::DVec2;
use rend::*;

pub struct Ui<'a> {
    events: EventBus<UiEvent>,
    mouse_pos: DVec2,
    input: InputState,
    cmd: &'a mut RenderCommands,
    anim: &'a AnimationStates,
    id: usize,
    pos: DVec2,
    padding: f64,
    font_size: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    ButtonClicked,
    CheckboxClicked,
}

pub struct ButtonResponse {
    pub is_hovered: bool,
    pub is_clicked: bool,
    pub is_pressed: bool,
    pub extent: DVec2,
}

impl ButtonResponse {
    pub fn clicked(&self) -> bool {
        self.is_clicked
    }
}

impl<'a> Ui<'a> {
    pub fn new(
        mouse_pos: DVec2,
        input: InputState,
        cmd: &'a mut RenderCommands,
        anim: &'a AnimationStates,
    ) -> Self {
        Self {
            events: EventBus::new(),
            mouse_pos,
            input,
            cmd,
            anim,
            id: 0,
            pos: DVec2::new(30.0, 50.0),
            padding: 6.0,
            font_size: 23.0,
        }
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn events(&self) -> impl Iterator<Item = &UiEvent> {
        self.events.iter()
    }

    pub fn mouse_pos(&self) -> DVec2 {
        self.mouse_pos
    }

    pub fn fonts(&self) -> &Components<FontInfo> {
        &self.cmd.fonts
    }

    pub fn separator(&mut self) {
        let height = 5.0;
        let dims = DVec2::new(300.0, height);
        self.cmd.rect(self.pos).dims(dims).color(Color::WHITE);
        self.pos.y += height + self.padding;
    }

    fn text_box(
        &mut self,
        text: impl Into<String>,
        color: Color,
        on_click: Option<UiEvent>,
    ) -> ButtonResponse {
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
        let is_hovered = aabb.contains(self.mouse_pos.as_vec2());

        let is_clicked = is_hovered && self.input.just_pressed(rdev::Button::Left);
        let is_pressed = is_hovered && self.input.is_key_pressed(rdev::Button::Left);

        let mix_param = if is_pressed {
            0.5
        } else if is_hovered {
            0.2
        } else {
            0.0
        };

        let color = color.mix(Color::BLACK, mix_param);

        self.cmd
            .rect(self.pos)
            .dims(full_extent)
            .color(color)
            .z(0.52);

        self.pos.y += full_extent.y + self.padding;

        if is_clicked && let Some(on_click) = on_click {
            self.events.enqueue(on_click);
        }

        ButtonResponse {
            is_hovered,
            is_clicked,
            is_pressed,
            extent: full_extent,
        }
    }

    pub fn label(&mut self, text: impl Into<String>) -> ButtonResponse {
        self.text_box(text, Color::hsl(0.3, 0.5, 0.2, 1.0), None)
    }

    pub fn button(&mut self, text: impl Into<String>, color: Color) -> ButtonResponse {
        self.text_box(text, color, Some(UiEvent::ButtonClicked))
    }

    pub fn checkbox(&mut self, text: impl Into<String>, state: bool) -> ButtonResponse {
        self.text_box(
            text,
            Color::hsl(0.7, 0.5, 0.2, 1.0),
            Some(UiEvent::CheckboxClicked),
        )
    }
}
