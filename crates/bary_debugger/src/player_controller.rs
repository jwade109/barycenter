use std::collections::BTreeMap;

use bary_core::prelude::*;
use bary_ipc::ServerRequest;
use bary_sim::{JoystickPosition, PlayerControls, Sender, World, WorldDelta};

pub struct PlayerController {
    selected_player: Option<Ent>,
    controls: BTreeMap<Ent, PlayerControls>,
}

fn grid_row(ui: &mut egui::Ui, y: JoystickPosition, ctrl: &mut PlayerControls) {
    if ui.button("*").clicked() {
        ctrl.x = JoystickPosition::Negative;
        ctrl.y = y;
    }
    if ui.button("*").clicked() {
        ctrl.x = JoystickPosition::Zero;
        ctrl.y = y;
    }
    if ui.button("*").clicked() {
        ctrl.x = JoystickPosition::Positive;
        ctrl.y = y;
    }
    ui.end_row();
}

fn joystick_pos_ui(ui: &mut egui::Ui, ctrl: &mut PlayerControls) -> bool {
    let old_ctrl = *ctrl;
    egui::Grid::new("joystick_pos")
        .num_columns(3)
        .show(ui, |ui| {
            grid_row(ui, JoystickPosition::Positive, ctrl);
            grid_row(ui, JoystickPosition::Zero, ctrl);
            grid_row(ui, JoystickPosition::Negative, ctrl);
        });
    old_ctrl != *ctrl
}

impl PlayerController {
    pub fn new() -> Self {
        Self {
            selected_player: None,
            controls: BTreeMap::new(),
        }
    }

    fn player_specific_ui(
        &mut self,
        ui: &mut egui::Ui,
        id: Ent,
        world: &World,
        req: &mut Sender<ServerRequest>,
    ) {
        if ui.button("Choose a different player").clicked() {
            self.selected_player = None;
            return;
        }

        ui.label(format!("Selected player is {id}"));

        let Some(player) = world.players.get(id) else {
            self.selected_player = None;
            return;
        };

        if ui.button("Disconnect").clicked() {
            req.send(WorldDelta::DisconnectPlayer(id).into());
        }

        let ctrl = self.controls.entry(id).or_default();

        ui.label(format!("{:?}", player));
        ui.label(format!("{:?}", ctrl));

        let send_anyway = ui.button("Send Input Vector").clicked();

        if joystick_pos_ui(ui, ctrl) || send_anyway {
            let delta = WorldDelta::SetPlayerControls(id, *ctrl);
            req.send(delta.into());
        }
    }

    fn select_a_player_ui(&mut self, ui: &mut egui::Ui, world: &World) {
        for (id, player) in world.players.iter() {
            let label = format!(
                "{} {} {}",
                id,
                player.username,
                distance_str_v(player.iso.tr())
            );
            if ui.button(label).clicked() {
                self.selected_player = Some(*id);
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, world: &World, req: &mut Sender<ServerRequest>) {
        ui.heading("CONTROL DA PLAYERS");

        if ui.button("Request Player ID").clicked() {
            req.send(ServerRequest::RequestPlayerId)
        }

        if let Some(id) = self.selected_player {
            self.player_specific_ui(ui, id, world, req);
        } else {
            self.select_a_player_ui(ui, world);
        }
    }
}
