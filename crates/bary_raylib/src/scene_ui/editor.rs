use bary_core::prelude::{PartLayer, Vec2};
use bary_sim::World;

use crate::{EditorState, Scene, UiBuilder, UiMessage};

pub fn make_editor_ui(ui: &mut UiBuilder, editor: &EditorState, world: &World) {
    ui.begin_window_at(Vec2::new(20.0, 20.0));
    ui.text(format!("Editing {}", editor.vehicle));
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.end_window();

    new_editor_layer_indicator(ui);

    if let Some(layer) = editor.layer {
        all_parts_in_layer_ui(ui, layer, world);
    }
}

fn new_editor_layer_indicator(ui: &mut UiBuilder) {
    ui.begin_window("editor_layer_selection");

    for layer in PartLayer::all() {
        ui.button(UiMessage::EditorSetLayer(layer), format!("{:?}", layer));
    }

    ui.end_window();
}

fn all_parts_in_layer_ui(ui: &mut UiBuilder, layer: PartLayer, world: &World) {
    ui.begin_window("editor_parts_selection");
    for (proto_id, proto) in world.prototypes.iter() {
        if proto.layer != layer {
            continue;
        }

        ui.button(UiMessage::EditorSetProto(*proto_id), &proto.name);
    }
    ui.end_window();
}
