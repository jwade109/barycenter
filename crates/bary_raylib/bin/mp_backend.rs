use crate::*;
use bary_core::prelude::WallTimer;
use bary_ipc::{Blob, ClientNode, ClientTelemetry, ServerResponse};
use bary_raylib::AsyncWorldDownload;
use bary_sim::{World, WorldDelta, apply_delta, timedelta_from_delta_ticks, update_world};
use log::{error, warn};
use std::time::Duration;

pub struct MultiplayerBackend {
    world: World,
    node: ClientNode,
    async_world_download: Option<AsyncWorldDownload>,
    username: String,
    server_handshake_complete: bool,
    server_ping_timer: WallTimer,
    server_telemetry_timer: WallTimer,
}

impl MultiplayerBackend {
    pub fn new(world: World, node: ClientNode, username: String) -> Self {
        let server_ping_timer = WallTimer::with_dur(Duration::from_millis(1000));
        let server_telemetry_timer = WallTimer::with_dur(Duration::from_millis(100));

        Self {
            world,
            node,
            async_world_download: None,
            username,
            server_handshake_complete: false,
            server_ping_timer,
            server_telemetry_timer,
        }
    }

    fn request_delta(&mut self, delta: WorldDelta) {
        self.node.send(delta);
    }

    fn report_telemetry(&mut self) {
        let tlm = ClientTelemetry {
            ticks: self.world.ticks,
        };
        self.node.send(ServerRequest::ClientTelemetry(tlm));
    }

    fn send_ping(&mut self) {
        self.node.send(ServerRequest::Ping);
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.node.send(ServerRequest::SetSimSpeed(speed));
    }

    fn load_save(&mut self, path: impl Into<String>) {
        self.node.send(ServerRequest::LoadSave(path.into()));
    }

    fn on_driver_packet(&mut self, ticks: u64, deltas: Vec<WorldDelta>) {
        while self.world.ticks + 1 < ticks {
            update_world(&mut self.world);
        }

        for delta in deltas {
            if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
                error!("Failed to apply delta {:?}: {:?}", delta, e);
            }
        }

        if self.world.ticks < ticks {
            update_world(&mut self.world);
        }
    }

    fn on_rcv_who_goes_there(&mut self) {
        self.node.send(ServerRequest::Introduction {
            username: self.username.clone(),
        });
    }

    fn on_rcv_kicked(&mut self) {
        self.world = World::empty();
    }

    fn on_rcv_have_new_save(&mut self) {
        info!("Server has reported it has a new save file");
        self.node.send(ServerRequest::BeginAsyncWorldDownload);
        self.async_world_download = Some(AsyncWorldDownload::new());
    }

    fn on_rcv_entity(&mut self, ident: TableIdent, id: Ent, data: Vec<u8>) {
        info!("Received entity {} {} ({} bytes)", ident, id, data.len());
        if let Some(dl) = &mut self.async_world_download {
            dl.add_entity(ident, id, data);
        } else {
            error!("No download underway!");
        }
    }

    fn on_transaction_complete(&mut self) {
        match std::mem::take(&mut self.async_world_download) {
            Some(dl) => {
                info!("Successfully unpacked transaction.");
                self.world = dl.world();
            }
            None => {
                error!("No transaction was underway!");
            }
        }
    }

    fn request_world_download(&mut self) {
        self.node.send(ServerRequest::BeginAsyncWorldDownload);
        self.async_world_download = Some(AsyncWorldDownload::new());
    }

    fn on_rcv_take_your_hat_off(&mut self) {
        self.server_handshake_complete = true;
    }

    fn update(&mut self) -> Vec<ServerResponse> {
        // if self.server_ping_timer.tick() {
        //     if self.server_handshake_complete {
        //         self.send_ping();
        //     }
        // }

        // if self.server_telemetry_timer.tick() {
        //     self.report_telemetry();
        // }

        let mut propagated = Vec::new();
        for msg in self.node.update() {
            let e = match msg.clone() {
                ServerResponse::Driver {
                    ticks,
                    speed: _,
                    deltas,
                } => {
                    self.on_driver_packet(ticks, deltas);
                    continue;
                }
                ServerResponse::Kicked => {
                    self.on_rcv_kicked();
                    propagated.push(msg);
                    continue;
                }
                ServerResponse::WhoGoesThere => {
                    self.on_rcv_who_goes_there();
                    continue;
                }
                ServerResponse::HaveNewSave => {
                    self.on_rcv_have_new_save();
                    continue;
                }
                ServerResponse::TakeYourHatOff => {
                    self.on_rcv_take_your_hat_off();
                    continue;
                }
                ServerResponse::Ack => {
                    continue;
                }
                ServerResponse::Pong => {
                    continue;
                }
                ServerResponse::Entity(ident, id, data) => {
                    self.on_rcv_entity(ident, id, data);
                    continue;
                }
                ServerResponse::TransactionComplete(x) => {
                    self.on_transaction_complete();
                    ServerResponse::TransactionComplete(x)
                }
                e => e,
            };

            propagated.push(e);
        }
        propagated
    }
}

impl Backend for MultiplayerBackend {
    fn world(&self) -> &World {
        &self.world
    }

    fn request_delta(&mut self, delta: WorldDelta) {
        self.request_delta(delta);
    }

    fn load_save(&mut self, path: String) {
        self.load_save(path);
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.set_sim_speed(speed);
    }

    fn request_world_download(&mut self) {
        self.request_world_download();
    }

    fn network(&self) -> Option<&ClientNode> {
        Some(&self.node)
    }

    fn username(&self) -> Option<&str> {
        Some(&self.username)
    }

    fn update(&mut self, _controls: &ExternalControls) -> Vec<ServerResponse> {
        self.update()
    }
}
