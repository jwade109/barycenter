use crate::{Scene, UiBuilder};

pub fn kicked_scene_ui(ui: &mut UiBuilder) {
    ui.begin_window("kicked_dialogue");
    ui.text("You've been kicked.");
    ui.button(Scene::MainMenu, "Load Single Player");
    ui.end_window();
}
