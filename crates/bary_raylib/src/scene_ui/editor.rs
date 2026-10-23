use crate::{
    Scene, UiBuilder,
    UiMessage::{self, Navigate},
};
use bary_core::prelude::Vec2;
use bary_ui::*;

pub fn make_editor_ui(ui: &UiBuilder, tree: &mut Tree<UiMessage>) {
    tree.add_layout(
        Node::column(
            Size::Fit,
            vec![
                ui.button(Navigate(Scene::Free), "Editor!"),
                ui.button(Navigate(Scene::Free), "Editor!"),
                ui.button(Navigate(Scene::Free), "Editor!"),
                ui.button(Navigate(Scene::Free), "Editor!"),
            ],
        ),
        Vec2::splat(40.0),
    );
}
