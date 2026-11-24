use bary_core::prelude::Vec2;

use crate::{Scene, UiBuilder, UiMessage};

pub fn main_menu_ui(ui: &mut UiBuilder) {
    ui.window_at("main_menu", Vec2::splat(250.0), |ui| {
        ui.button(Scene::ListSaves, "Load Single Player");
        ui.button(UiMessage::JoinMultiplayer, "Join Server");
        ui.button(Scene::Settings, "Settings");
        ui.button(Scene::Kicked, "Go to Kicked Menu");
        ui.button(UiMessage::Exit, "Exit to Desktop");
    });
}
