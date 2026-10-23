use crate::{UI_FONT_SIZE, UiInteractionState, UiMessage};
use bary_core::prelude::Vec2;
use bary_input::InputState;
use bary_ui::*;
use raylib::prelude::*;
use std::rc::Rc;

pub struct UiBuilder {
    font: Rc<Font>,
    pub font_size: f32,
    active_node: Option<(Node<UiMessage>, Vec2)>,
    input: InputState,
    tree: Tree<UiMessage>,
    state: UiInteractionState,
}

impl UiBuilder {
    pub fn new(font: Rc<Font>, input: InputState, state: UiInteractionState) -> Self {
        Self {
            font,
            font_size: UI_FONT_SIZE as f32,
            active_node: None,
            input,
            tree: Tree::new(),
            state,
        }
    }

    pub fn hot_element(&self) -> Option<&UiMessage> {
        self.state.hot()
    }

    pub fn active_element(&self) -> Option<&UiMessage> {
        self.state.active()
    }

    pub fn begin_window(&mut self, id_salt: impl Into<String>) {
        let id_salt = id_salt.into();
        let offset = self.window_pos(id_salt.clone(), Vec2::splat(400.0));
        let node = Node::column(Size::Fit, Vec::new());
        self.active_node = Some((node, offset));
        self.draghandle(id_salt);
    }

    pub fn begin_window_at(&mut self, origin: Vec2) {
        self.end_window();
        let node = Node::column(Size::Fit, Vec::new());
        self.active_node = Some((node, origin));
    }

    pub fn end_window(&mut self) {
        if let Some((node, origin)) = self.active_node.take() {
            self.tree.add_layout(node, origin)
        }
    }

    pub fn window_pos(&mut self, salt: impl Into<String>, offset: Vec2) -> Vec2 {
        self.state.window_pos(salt, offset)
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn add_layout(&mut self, node: Node<UiMessage>, origin: impl Into<Option<Vec2>>) {
        self.tree.add_layout(node, origin);
    }

    pub fn take(self) -> (Tree<UiMessage>, UiInteractionState) {
        (self.tree, self.state)
    }

    fn add_to_active(&mut self, n: Node<UiMessage>) {
        if self.active_node.is_none() {
            self.begin_window("anonymous_window");
        }
        if let Some((node, _)) = &mut self.active_node {
            node.add_child(n);
        }
    }

    pub fn draghandle(&mut self, salt: impl Into<String>) {
        let msg = UiMessage::Drag(salt.into());
        let n = Node::handle(Size::Grow, 20, msg);
        self.add_to_active(n);
    }

    pub fn sep(&mut self) {
        self.add_to_active(Node::structural(Size::Grow, 2));
    }

    pub fn text(&mut self, s: impl Into<String>) {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x;
        let h = dims.y;
        self.add_to_active(Node::text(w, h, s))
    }

    pub fn button(&mut self, msg: impl Into<UiMessage>, s: impl Into<String>) {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x + 36.0;
        let h = dims.y + 18.0;
        self.add_to_active(Node::button(s, msg, w, h));
    }

    pub fn sprite(&mut self, s: impl Into<String>, texture: &Texture2D) {
        let h = 70;
        let actual_w = texture.width;
        let actual_h = texture.height;
        let scale = actual_h as f32 / h as f32;
        let w = actual_w as f32 / scale;
        self.add_to_active(Node::image(s, w as u32, h));
    }

    pub fn progress_bar(&mut self, val: f32) {
        self.add_to_active(Node::progress_bar(Size::Grow, 11, val));
    }
}
