use crate::{Scene, UiBuilder, UiMessage};

pub fn main_menu_ui(ui: &mut UiBuilder) {
    ui.begin_window("main_menu");
    ui.button(Scene::ListSaves, "Load Single Player");
    ui.button(UiMessage::JoinMultiplayer, "Join Server");
    ui.button(Scene::Settings, "Settings");
    ui.button(Scene::Kicked, "Go to Kicked Menu");
    ui.button(UiMessage::Exit, "Exit to Desktop");
    ui.end_window();
}
