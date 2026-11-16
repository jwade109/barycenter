use std::collections::BTreeMap;

use bary_ipc::{GridsTelemetry, ServerResponse, WorldTelemetry};
use bary_server::{make_grid_telemetry, make_world_telemetry};
use bary_sim::World;

#[derive(Debug, Default, Clone)]
struct TelemetryFrame {
    world: Option<WorldTelemetry>,
    grids: Option<GridsTelemetry>,
}

pub struct WorldComparator {
    server: BTreeMap<u64, TelemetryFrame>,
    local: BTreeMap<u64, TelemetryFrame>,
}

const HISTORY_AGE_TICKS: u64 = 10;

fn limit_history(frames: &mut BTreeMap<u64, TelemetryFrame>, latest_tick: u64) {
    frames.retain(|tick, _| *tick + HISTORY_AGE_TICKS > latest_tick && *tick <= latest_tick);
}

impl WorldComparator {
    pub fn new() -> Self {
        Self {
            server: BTreeMap::new(),
            local: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.server.clear();
        self.local.clear();
    }

    pub fn on_server_telemetry(&mut self, tlm: &ServerResponse) {
        match tlm {
            ServerResponse::Summary(tlm) => {
                let frame = self.server.entry(tlm.tick).or_default();
                frame.world = Some(*tlm);
                limit_history(&mut self.server, tlm.tick);
            }
            ServerResponse::Grids(tlm) => {
                let frame = self.server.entry(tlm.tick).or_default();
                frame.grids = Some(tlm.clone());
                limit_history(&mut self.server, tlm.tick);
            }
            _ => (),
        }
    }

    pub fn update_local(&mut self, world: &World) {
        let frame = self.local.entry(world.ticks).or_default();
        frame.world = Some(make_world_telemetry(world));
        frame.grids = Some(make_grid_telemetry(world));
        limit_history(&mut self.local, world.ticks);
    }

    pub fn diff(&self, ui: &mut egui::Ui, server: &GridsTelemetry, local: &GridsTelemetry) {
        for (id, server_grid) in &server.grids {
            let Some(local_grid) = local.grids.get(id) else {
                continue;
            };

            ui.label(server.tick.to_string());
            ui.label(id.to_string());
            ui.label(format!("{:0.3}", server_grid.pos.tr()));
            ui.label(format!("{:0.3}", local_grid.pos.tr()));
            ui.label(format!(
                "{:0.3}",
                server_grid.pos.tr() - local_grid.pos.tr()
            ));
            ui.end_row();
        }
    }

    fn grid_inner(&self, ui: &mut egui::Ui) {
        for (server_tick, server_frame) in &self.server {
            let Some(local_frame) = self.local.get(server_tick) else {
                continue;
            };

            if let (Some(s), Some(l)) = (&server_frame.grids, &local_frame.grids) {
                self.diff(ui, s, l);
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                egui::Grid::new("comparison_grid").show(ui, |ui| {
                    self.grid_inner(ui);
                });
            });
    }
}
