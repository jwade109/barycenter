use crate::UI_FONT_SIZE;
use bary_ui::*;
use raylib::prelude::*;

pub struct UiBuilder<'a> {
    font: &'a Font,
    font_size: f32,
}

impl<'a> UiBuilder<'a> {
    pub fn new(font: &'a Font) -> Self {
        Self {
            font,
            font_size: UI_FONT_SIZE as f32,
        }
    }

    pub fn draghandle<T: UiMsg>(&self, msg: impl Into<T>) -> Node<T> {
        Node::<T>::handle(Size::Grow, 20, msg)
    }

    pub fn sep<T: UiMsg>(&self) -> Node<T> {
        Node::structural(Size::Grow, 2)
    }

    pub fn text<T: UiMsg>(&self, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x;
        let h = dims.y;
        Node::text(w, h, s)
    }

    pub fn button<T: UiMsg>(&self, msg: impl Into<T>, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x + 36.0;
        let h = dims.y + 18.0;
        Node::button(s, msg, w, h)
    }

    pub fn sprite<T: UiMsg>(&self, s: impl Into<String>, texture: &Texture2D) -> Node<T> {
        let h = 70;
        let actual_w = texture.width;
        let actual_h = texture.height;
        let scale = actual_h as f32 / h as f32;
        let w = actual_w as f32 / scale;
        Node::image(s, w as u32, h)
    }

    pub fn progress_bar<T: UiMsg>(&self, val: f32) -> Node<T> {
        Node::progress_bar(Size::Grow, 11, val)
    }
}
