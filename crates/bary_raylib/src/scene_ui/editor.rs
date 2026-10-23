use crate::{EditorState, Scene, UiBuilder, UiMessage};

pub fn make_editor_ui(ui: &mut UiBuilder, editor: &EditorState) {
    ui.begin_window("editor_window");
    ui.text(format!("Editing {}", editor.vehicle));
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
    ui.button(UiMessage::Navigate(Scene::Free), "Editor!");
}
