use crate::*;
use bary_core::prelude::*;
use bary_sim::*;
use bary_ui::*;
use early_returns::*;

pub fn make_part_info_gui(
    ui: &UiBuilder,
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
    tree: &mut Tree<UiMessage>,
) {
    let gridloc = some_or_return!(client.selected_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let mut children = Vec::new();

    let s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    children.push(ui.draghandle(DragHandle::PartInfo));
    children.push(ui.text(s));

    for (_, part_id) in occ.iter() {
        children.push(ui.sep());
        children.push(ui.button(
            Scene::Part(part_id),
            format!("Go to Part {}", part_id),
        ));
        if let Some(node) = part_info_modal(part_id, ui, world, assets, "") {
            children.push(node);
        }
    }

    let node = Node::column(600, children);
    tree.add_layout(node, Vec2::splat(100.0));
}
