use crate::*;
use bary_core::prelude::{Ent, Vec2};
use bary_factory::InvSlot;
use bary_sim::*;
use bary_ui::*;
use early_returns::*;

pub fn part_info_ui(
    part_id: Ent,
    ui: &UiBuilder,
    world: &World,
    assets: &Assets,
) -> Option<Node<UiMessage>> {
    let part = world.parts.try_get(part_id).ok()?;
    let proto = world.prototypes.try_get(part.prototype).ok()?;

    let mut children = Vec::new();

    children.push(ui.text(proto.name.clone()));

    if let Some(texture) = assets.part_textures.get(&proto.name) {
        children.push(ui.sprite(proto.name.clone(), texture));
    }

    if let Ok(cpu) = world.computers.try_get(part_id) {
        children.push(ui.button(
            UiMessage::SetComputerOnOff(part_id, !cpu.on),
            if cpu.on { "Turn Off" } else { "Turn On" },
        ));
        children.push(ui.button(UiMessage::SetComputerDrift(part_id), "Set to Drift"));
        let info = computer_info_str(cpu);
        children.push(ui.text(info));
    }
    if let Ok(thruster) = world.thrusters.try_get(part_id) {
        let s = format!("{:#?}", thruster);
        children.push(ui.button(
            UiMessage::SetThrusterState(part_id, !thruster.is_on()),
            if thruster.is_on() {
                "Turn Off"
            } else {
                "Turn On"
            },
        ));
        children.push(ui.text(s));
    }
    if let Ok(light) = world.lights.try_get(part_id) {
        let s = format!("{:#?}", light);
        children.push(ui.text(s));
    }
    if let Ok(mac) = world.machines.try_get(part_id) {
        children.push(ui.progress_bar(mac.progress()));
        children.push(ui.button(
            UiMessage::SetMachineState(part_id, !mac.enabled),
            if mac.enabled { "Turn Off" } else { "Turn On" },
        ));
    }
    if let Ok(inv) = world.inventories.try_get(part_id) {
        for (idx, slot) in inv.slots().enumerate() {
            let line = slot_info_str(idx, slot);
            children.push(ui.text(line));
            children.push(ui.progress_bar(slot.fill_percentage()));
        }
    }

    Some(Node::column(600.0, children))
}

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
            SceneNavigation::Part(part_id),
            format!("Go to Part {}", part_id),
        ));
        if let Some(node) = part_info_ui(part_id, ui, world, assets) {
            children.push(node);
        }
    }

    let node = Node::column(600, children);
    tree.add_layout(node, Vec2::splat(100.0));
}

pub fn slot_info_str(idx: usize, slot: &InvSlot) -> String {
    if let Some(contents) = slot.contents() {
        format!(
            "{}. {:?} ({}) ({:0.1}%) {}",
            idx + 1,
            contents.0,
            contents.1,
            100.0 * slot.fill_percentage(),
            slot.mass(),
        )
    } else {
        format!("{}. Empty - can store [{:?}]", idx + 1, slot.filter())
    }
}

pub fn computer_info_str(cpu: &Computer) -> String {
    let mut lines = vec![
        format!("On: {}", cpu.on),
        format!("\nStatus: {:?}", cpu.status),
        format!("\nTicks: {}", cpu.ticks_this_cycle),
        format!("\nFired: {}", cpu.fired_this_tick),
        format!("\nIters: {}", cpu.iters),
    ];

    for cmd in &cpu.command_queue {
        let line = format!("\n  - {}", cmd);
        lines.push(line);
    }

    lines.into_iter().collect()
}

pub fn part_focus_ui(tree: &mut Tree<UiMessage>, ui: &UiBuilder) {
    tree.add_layout(ui.text("Hello!"), Vec2::splat(300.0));
}
