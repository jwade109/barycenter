use crate::{DragHandle, Scene, UiBuilder, UiMessage};
use bary_core::prelude::Vec2;
use bary_ui::*;

pub fn kicked_scene_ui(tree: &mut Tree<UiMessage>, ui: &UiBuilder, origin: Vec2) {
    tree.add_layout(
        Node::column(
            Size::Fit,
            vec![
                ui.draghandle(DragHandle::MainMenu),
                ui.text("You've been kicked."),
                ui.button(Scene::MainMenu, "Load Single Player"),
            ],
        ),
        origin,
    );
}
