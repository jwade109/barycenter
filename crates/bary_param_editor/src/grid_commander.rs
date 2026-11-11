use std::collections::BTreeMap;

use bary_core::prelude::{Angle, Ent, Isometry2d, distance_str_v, randvec};
use bary_sim::{Instruction, Sender, WorldDelta};

pub struct GridCommander {
    selected: Option<Ent>,
    heading: f32,
    x: f64,
    y: f64,
    kp: f64,
    kd: f64,
}

impl GridCommander {
    pub fn new() -> Self {
        Self {
            selected: None,
            heading: 0.0,
            x: 0.0,
            y: 0.0,
            kp: 30.0,
            kd: 50.0,
        }
    }

    fn grid_not_selected(
        &mut self,
        ui: &mut egui::Ui,
        grids: &BTreeMap<Ent, (String, String, Isometry2d)>,
    ) {
        for (id, (name, _, pos)) in grids {
            if ui
                .button(format!("{} {} {}", id, name, distance_str_v(pos.tr())))
                .clicked()
            {
                self.selected = Some(*id);
            }
        }
    }

    fn grid_control_panel(
        &mut self,
        ui: &mut egui::Ui,
        selected: Ent,
        grids: &BTreeMap<Ent, (String, String, Isometry2d)>,
        deltas: &mut Sender<WorldDelta>,
    ) {
        if ui.button("Select another ship").clicked() {
            self.selected = None;
            return;
        }

        let Some((name, model, iso)) = grids.get(&selected) else {
            self.selected = None;
            return;
        };

        ui.heading(name);
        ui.label(format!("Type: {model}"));
        ui.label(distance_str_v(iso.tr()));

        ui.heading("Heading");
        ui.horizontal(|ui| {
            ui.drag_angle(&mut self.heading);
            if ui.button("Attitude Hold").clicked() {
                let instr = Instruction::HoldAttitude(Angle::radians(self.heading));
                let delta = WorldDelta::SetGridInstruction(selected, instr.into());
                deltas.send(delta.into());
            }
        });
        ui.separator();

        ui.heading("Position");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.x).speed(0.1));
            ui.add(egui::DragValue::new(&mut self.y).speed(0.1));
            if ui.button("Position Hold").clicked() {
                let iso = (self.x as f64, self.y as f64, self.heading as f64).into();
                let instr = Instruction::HoldPosition(iso);
                let delta = WorldDelta::SetGridInstruction(selected, instr.into());
                deltas.send(delta.into());
            }
        });

        ui.separator();

        if ui.button("Drift").clicked() {
            let instr = Instruction::Drift;
            let delta = WorldDelta::SetGridInstruction(selected, instr.into());
            deltas.send(delta.into());
        }

        ui.separator();

        ui.heading("Attitude PD Coefficients");

        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.kp).speed(0.1));
            ui.add(egui::DragValue::new(&mut self.kd).speed(0.1));
            if ui.button("Set Coefficients").clicked() {
                let delta = WorldDelta::SetGridAttitudePD(selected, self.kp, self.kd);
                deltas.send(delta.into());
            }
        });
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        grids: &BTreeMap<Ent, (String, String, Isometry2d)>,
        deltas: &mut Sender<WorldDelta>,
    ) {
        if let Some(selected) = self.selected {
            self.grid_control_panel(ui, selected, grids, deltas);
        } else {
            self.grid_not_selected(ui, grids);
        }
    }
}
