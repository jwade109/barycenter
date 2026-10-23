use crate::{Scene, UiBuilder};

pub fn settings_scene_ui(ui: &mut UiBuilder) {
    ui.begin_window("settings_window");
    ui.text("The settings.");
    ui.button(Scene::MainMenu, "Return to Main Menu");
    ui.end_window();
}
