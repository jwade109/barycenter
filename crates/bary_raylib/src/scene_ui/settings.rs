use crate::{DragHandle, Scene, UiBuilder, UiMessage};
use bary_core::prelude::Vec2;
use bary_ui::*;

pub fn settings_scene_ui(tree: &mut Tree<UiMessage>, ui: &UiBuilder, origin: Vec2) {
    tree.add_layout(
        Node::column(
            Size::Fit,
            vec![
                ui.draghandle(DragHandle::MainMenu),
                Node::text(600, 100, "The settings."),
                ui.button(Scene::MainMenu, "Return to Main Menu"),
            ],
        ),
        origin,
    );
}
