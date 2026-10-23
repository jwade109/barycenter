use crate::*;
use bary_core::prelude::*;
use bary_sim::*;
use bary_ui::*;

pub fn make_free_flying_ui(ui: &UiBuilder, free: &FreeFlying, world: &World, tree: &mut Tree<UiMessage>) {
    let mut root: Node<UiMessage> = Node::root(Size::Fit, Size::Fit).with_children(
        [
            ui.button(UiMessage::Navigate(Scene::MainMenu), "Return to Main Menu"),
            ui.button(UiMessage::WriteSaveToDisk, "Save Game"),
            ui.button(UiMessage::AltMode, "Toggle Alt Mode"),
            ui.button(UiMessage::DebugText, "Toggle Debug Text"),
            ui.button(UiMessage::SimSpeed(0), "Toggle Pause"),
            ui.button(UiMessage::SimSpeed(1), "Sim 1x"),
            ui.button(UiMessage::SimSpeed(10), "Sim 10x"),
            ui.button(UiMessage::SimSpeed(100), "Sim 100x"),
            ui.button(UiMessage::SimSpeed(1000), "Sim 1000x"),
        ]
        .into_iter(),
    );

    if let Some(loc) = free.selected_vehicles.first() {
        let msg = UiMessage::Navigate(Scene::Editor(loc.grid_id));
        let b = ui.button(msg, "Open Ship Editor");
        root.add_child(b);
    }

    tree.add_layout(root, None);

    if let Some(docking) = free.docking_interface() {
        if let Some(ui) = make_docking_ui(ui, &docking, world) {
            tree.add_layout(ui, Vec2::splat(400.0))
        }
    }
}

fn make_docking_ui(
    builder: &UiBuilder,
    docking: &DockingInterface,
    world: &World,
) -> Option<Node<UiMessage>> {
    let parent = world.grids.try_get(docking.parent.grid_id).ok()?;
    let child = world.grids.try_get(docking.child.grid_id).ok()?;

    let sep = parent
        .particle_location
        .translation
        .distance(child.particle_location.translation);

    let sep = distance_str(sep as f64);

    let docking_ui = Node::column(
        Size::Fit,
        vec![
            builder.draghandle(DragHandle::Docking),
            builder.text("Docking Control Panel"),
            builder.text("Multiline string\nAnother line\nAnd another!"),
            builder.text(format!("{} <- {}", parent.name, child.name)),
            builder.text(format!("Separation: {}", sep)),
            builder.button(UiMessage::DockingShiftX, "Offset X"),
            builder.button(UiMessage::DockingShiftY, "Offset Y"),
            builder.button(UiMessage::DockingRotate, "Rotate"),
            builder.button(UiMessage::DockingActivate, "Dock"),
        ],
    );

    Some(docking_ui)
}
