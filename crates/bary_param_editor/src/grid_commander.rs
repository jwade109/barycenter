use std::collections::BTreeMap;

use bary_core::prelude::*;
use bary_ipc::GridTelemetry;
use bary_sim::{Instruction, Sender, WorldDelta};

pub struct GridCommander {
    selected: Option<Ent>,
    heading: f32,
    pos: DVec2,
    vel: DVec2,
    delta_v: DVec2,
    kp: f64,
    kd: f64,
}

impl GridCommander {
    pub fn new() -> Self {
        Self {
            selected: None,
            heading: 0.0,
            pos: DVec2::ZERO,
            vel: DVec2::ZERO,
            delta_v: DVec2::ZERO,
            kp: 30.0,
            kd: 50.0,
        }
    }

    fn grid_not_selected(&mut self, ui: &mut egui::Ui, grids: &BTreeMap<Ent, GridTelemetry>) {
        for (id, tlm) in grids {
            if ui
                .button(format!(
                    "{} {} {}",
                    id,
                    tlm.name,
                    distance_str_v(tlm.pos.tr())
                ))
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
        grids: &BTreeMap<Ent, GridTelemetry>,
        deltas: &mut Sender<WorldDelta>,
    ) {
        if ui.button("Select another ship").clicked() {
            self.selected = None;
            return;
        }

        let Some(tlm) = grids.get(&selected) else {
            self.selected = None;
            return;
        };

        ui.heading(&tlm.name);
        ui.label(format!("Type: {}", tlm.model));
        ui.label(distance_str_v(tlm.pos.tr()));
        ui.label(velocity_str_v(tlm.vel.tr()));

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

        ui.heading("Velocity");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.vel.x).speed(0.1));
            ui.add(egui::DragValue::new(&mut self.vel.y).speed(0.1));
            if ui.button("Velocity Hold").clicked() {
                let instr = Instruction::HoldVelocity(self.vel);
                let delta = WorldDelta::SetGridInstruction(selected, instr.into());
                deltas.send(delta.into());
            }
        });

        ui.separator();

        ui.heading("Delta-V");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.delta_v.x).speed(0.1));
            ui.add(egui::DragValue::new(&mut self.delta_v.y).speed(0.1));
            if ui.button("Position Hold").clicked() {
                let instr = Instruction::DeltaV(self.delta_v);
                let delta = WorldDelta::SetGridInstruction(selected, instr.into());
                deltas.send(delta.into());
            }
        });

        ui.separator();

        ui.heading("Position");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.pos.x).speed(0.1));
            ui.add(egui::DragValue::new(&mut self.pos.y).speed(0.1));
            if ui.button("Position Hold").clicked() {
                let iso = (self.pos, self.heading as f64).into();
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
        grids: &BTreeMap<Ent, GridTelemetry>,
        deltas: &mut Sender<WorldDelta>,
    ) {
        if let Some(selected) = self.selected {
            self.grid_control_panel(ui, selected, grids, deltas);
        } else {
            self.grid_not_selected(ui, grids);
        }
    }
}
