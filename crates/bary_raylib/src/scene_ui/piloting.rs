use crate::{PilotingState, Scene, UiBuilder};

pub fn make_piloting_ui(ui: &mut UiBuilder, piloting: &mut PilotingState) {
    ui.window("piloting", |ui| {
        ui.heading(format!("Piloting {piloting:?}"));
        ui.button(Scene::Free, "Return to Map View");
    })
}
