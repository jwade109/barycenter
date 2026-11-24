use crate::{Scene, UiBuilder, UiMessage};
use bary_core::prelude::Vec2;
use bary_sim::list_saves_in_dir;

pub fn list_saves_scene_ui(ui: &mut UiBuilder) {
    ui.window_at("list_saves", Vec2::splat(250.0), |ui| {
        let saves = list_saves_in_dir("saves/");
        let mut saves: Vec<_> = saves.into_iter().map(|s| s.display().to_string()).collect();
        saves.sort();

        for s in saves {
            ui.button(UiMessage::LoadSaveFile(s.clone()), s.clone());
        }

        ui.button(Scene::MainMenu, "Return to Main Menu");
    });
}
