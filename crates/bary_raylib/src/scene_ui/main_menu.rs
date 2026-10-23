use crate::{DragHandle, Scene, UiBuilder, UiMessage};
use bary_core::prelude::Vec2;
use bary_ui::*;

pub fn main_menu_ui(tree: &mut Tree<UiMessage>, builder: &UiBuilder, offset: Vec2) {
    tree.add_layout(make_main_menu(&builder), offset);
}

fn make_main_menu(ui: &UiBuilder) -> Node<UiMessage> {
    Node::column(
        Size::Fit,
        vec![
            ui.draghandle(DragHandle::MainMenu),
            ui.button(Scene::ListSaves, "Load Single Player"),
            ui.button(UiMessage::JoinMultiplayer, "Join Server"),
            ui.button(Scene::Settings, "Settings"),
            ui.button(Scene::Kicked, "Go to Kicked Menu"),
            ui.button(UiMessage::Exit, "Exit to Desktop"),
        ],
    )
}
