use crate::{Assets, PilotingState, Scene, UiBuilder, get_part_info_modal};
use bary_core::prelude::{Rotation, Vec2};
use bary_sim::{Instruction, World, WorldDelta};

pub fn make_piloting_ui(
    ui: &mut UiBuilder,
    piloting: &mut PilotingState,
    world: &World,
    assets: &Assets,
) {
    let grid_id = piloting.ownship;

    ui.window_at("piloting", Vec2::splat(40.0), |ui| {
        let Some(grid) = world.grids.get(grid_id) else {
            ui.heading(format!("Failed to find vehicle {grid_id}"));
            return;
        };

        ui.heading(format!("Piloting {}", grid.name));

        ui.button(Scene::Free, "Return to Map View");

        let make_instr = |instr: Instruction| WorldDelta::SetGridInstruction(grid_id, instr.into());

        let adrift = make_instr(Instruction::Drift);

        let point_east = make_instr(Instruction::HoldAttitude(Rotation::East.into()));
        let point_north = make_instr(Instruction::HoldAttitude(Rotation::North.into()));
        let point_west = make_instr(Instruction::HoldAttitude(Rotation::West.into()));
        let point_south = make_instr(Instruction::HoldAttitude(Rotation::South.into()));

        ui.sep();

        ui.heading("Orientation Commands");

        ui.button(point_east, "point_east");
        ui.button(point_north, "point_north");
        ui.button(point_west, "point_west");
        ui.button(point_south, "point_south");

        let rcs_forward = make_instr(Instruction::rcs_forward());
        let rcs_backward = make_instr(Instruction::rcs_backward());
        let rcs_right = make_instr(Instruction::rcs_right());
        let rcs_left = make_instr(Instruction::rcs_left());

        ui.sep();

        ui.heading("RCS Commands");

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
    })
}
