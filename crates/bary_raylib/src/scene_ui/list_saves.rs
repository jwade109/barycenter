use crate::{DragHandle, Scene, UiBuilder, UiMessage};
use bary_core::prelude::Vec2;
use bary_sim::list_saves_in_dir;
use bary_ui::*;

pub fn list_saves_scene_ui(tree: &mut Tree<UiMessage>, builder: &UiBuilder, origin: Vec2) {
    let mut saves = list_saves_in_dir("saves/");
    saves.sort();
    let saves: Vec<_> = saves.into_iter().map(|s| s.display().to_string()).collect();
    tree.add_layout(make_savefiles_menu(&builder, &saves), origin);
}

fn make_savefiles_menu(ui: &UiBuilder, saves: &[String]) -> Node<UiMessage> {
    let mut buttons = vec![ui.draghandle(DragHandle::SaveSelect)];

    for s in saves {
        let b = ui.button(UiMessage::LoadSaveFile(s.clone()), s.clone());
        buttons.push(b);
    }

    buttons.push(ui.button(Scene::MainMenu, "Return to Main Menu"));

    Node::column(Size::Fit, buttons)
}
