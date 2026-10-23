use crate::*;
use bary_core::prelude::{Rotation, Vec2, distance_str, velocity_str};
use bary_sim::*;
use early_returns::*;

pub fn make_part_info_gui(
    ui: &mut UiBuilder,
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
) {
    let gridloc = some_or_return!(client.selected_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    ui.begin_window("part_info_window");
    ui.text(s);

    for (_, part_id) in occ.iter() {
        ui.sep();
        ui.button(Scene::Part(part_id), format!("Go to Part {}", part_id));
        get_part_info_modal(ui, part_id, world, assets, None);
    }

    ui.end_window();
}

pub fn make_free_flying_ui(ui: &mut UiBuilder, free: &FreeFlying, world: &World, assets: &Assets) {
    ui.begin_window_at(Vec2::splat(40.0), "free_flying_window");
    ui.button(UiMessage::Navigate(Scene::MainMenu), "Return to Main Menu");
    ui.button(UiMessage::WriteSaveToDisk, "Save Game");
    ui.button(UiMessage::AltMode, "Toggle Alt Mode");
    ui.button(UiMessage::DebugText, "Toggle Debug Text");
    ui.button(UiMessage::SimSpeed(0), "Toggle Pause");
    ui.button(UiMessage::SimSpeed(1), "Sim 1x");
    ui.button(UiMessage::SimSpeed(10), "Sim 10x");
    ui.button(UiMessage::SimSpeed(100), "Sim 100x");
    ui.button(UiMessage::SimSpeed(1000), "Sim 1000x");

    if let Some(loc) = free.selected_vehicles.first() {
        if let Some(grid) = world.grids.get(loc.grid_id) {
            let msg = UiMessage::Navigate(Scene::Editor(loc.grid_id));
            ui.sep();
            ui.text(grid.name.clone());
            ui.button(msg, "Open Ship Editor");

            ui.sep();

            let make_instr = |instr: Instruction| UiMessage::TimedInstruction {
                grid_id: loc.grid_id,
                ti: TimedInstruction::perp(instr),
            };

            let adrift = make_instr(Instruction::Drift);

            let point_east = make_instr(Instruction::HoldAttitude(Rotation::East.into()));
            let point_north = make_instr(Instruction::HoldAttitude(Rotation::North.into()));
            let point_west = make_instr(Instruction::HoldAttitude(Rotation::West.into()));
            let point_south = make_instr(Instruction::HoldAttitude(Rotation::South.into()));

            ui.sep();

            ui.text("Orientation Commands");

            ui.button(point_east, "point_east");
            ui.button(point_north, "point_north");
            ui.button(point_west, "point_west");
            ui.button(point_south, "point_south");

            let rcs_forward = make_instr(Instruction::rcs_forward());
            let rcs_backward = make_instr(Instruction::rcs_backward());
            let rcs_right = make_instr(Instruction::rcs_right());
            let rcs_left = make_instr(Instruction::rcs_left());

            ui.sep();

            ui.text("RCS Commands");

            ui.button(rcs_forward, "Forward");
            ui.button(rcs_backward, "Backward");
            ui.button(rcs_right, "Right");
            ui.button(rcs_left, "Left");

            ui.sep();

            ui.button(adrift, "Set Adrift");

            ui.sep();

            if let Some(cpu_id) = grid.computers.first() {
                get_part_info_modal(ui, *cpu_id, world, assets, None);
            }
        }
    }

    ui.end_window();

    if let Some(docking) = free.docking_interface() {
        make_docking_ui(ui, &docking, world);
    }
}

fn make_docking_ui(ui: &mut UiBuilder, docking: &DockingInterface, world: &World) -> Option<()> {
    let parent = world.grids.try_get(docking.parent.grid_id).ok()?;
    let child = world.grids.try_get(docking.child.grid_id).ok()?;

    let u = parent.particle_location.translation;
    let v = child.particle_location.translation;

    let sep = u.distance(v);

    let u = parent.velocity.translation;
    let v = child.velocity.translation;

    let vel = u.distance(v);

    let sep = distance_str(sep);
    let vel = velocity_str(vel);

    ui.begin_window("docking_dialogue");
    ui.text("Docking Control Panel");
    ui.text(format!("{} <- {}", parent.name, child.name));
    ui.text(format!("Separation: {}", sep));
    ui.text(format!("Rel Velocity: {}", vel));
    ui.button(UiMessage::DockingShiftX, "Offset X");
    ui.button(UiMessage::DockingShiftY, "Offset Y");
    ui.button(UiMessage::DockingRotate, "Rotate");
    ui.button(UiMessage::DockingActivate, "Dock");
    ui.end_window();

    Some(())
}
