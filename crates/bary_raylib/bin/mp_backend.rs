use crate::*;
use bary_ipc::{ClientNode, ClientTelemetry, ServerResponse};
use bary_sim::{World, WorldDelta, apply_delta, update_world};
use log::error;

pub struct MultiplayerBackend {
    world: World,
    node: ClientNode,
    async_world_download: WorldDownloader,
    username: String,
    server_handshake_complete: bool,
}

impl MultiplayerBackend {
    pub fn new(world: World, node: ClientNode, username: String) -> Self {
        Self {
            world,
            node,
            async_world_download: WorldDownloader::new(),
            username,
            server_handshake_complete: false,
        }
    }

    fn request_delta(&mut self, delta: WorldDelta) {
        self.node.send(delta);
    }

    fn report_telemetry(&mut self) {
        let grids = make_grid_telemetry(&self.world);
        let world = make_world_telemetry(&self.world);
        let tlm = ClientTelemetry { grids, world };
        self.node.send(ServerRequest::ClientTelemetry(tlm));
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

        if self.world.ticks % 25 == 0 {
            self.report_telemetry();
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
        self.node.send(ServerRequest::BeginWorldDownload);
    }

    fn on_rcv_fragment(&mut self, fragment: DownloadFragment, sender: &mut Sender<BackendEvent>) {
        if let Some((world, name)) = self.async_world_download.on_fragment(fragment) {
            self.world = world;
            sender.send(BackendEvent::FinishedWorldDownload(name));
        }
    }

    fn request_world_download(&mut self) {
        self.node.send(ServerRequest::BeginWorldDownload);
    }

    fn on_rcv_take_your_hat_off(&mut self) {
        self.server_handshake_complete = true;
    }

    fn update(&mut self) -> Vec<BackendEvent> {
        let mut events = Sender::new();

        for msg in self.node.update() {
            let e = match msg.clone() {
                ServerResponse::Driver {
                    ticks,
                    speed: _,
                    deltas,
                } => {
                    self.on_driver_packet(ticks, deltas);
                    msg
                }
                ServerResponse::Kicked => {
                    self.on_rcv_kicked();
                    events.send(msg.into());
                    continue;
                }
                ServerResponse::PleaseSendUsername => {
                    self.on_rcv_who_goes_there();
                    continue;
                }
                ServerResponse::HaveNewSave => {
                    self.on_rcv_have_new_save();
                    continue;
                }
                ServerResponse::SuccessfullyConnected => {
                    self.on_rcv_take_your_hat_off();
                    continue;
                }
                ServerResponse::Ack => {
                    continue;
                }
                ServerResponse::Pong => {
                    continue;
                }
                ServerResponse::Download(fragment) => {
                    self.on_rcv_fragment(fragment, &mut events);
                    continue;
                }
                e => e,
            };

            events.send(e.into());
        }

        events.take()
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

    fn update(&mut self, _controls: &ExternalControls) -> Vec<BackendEvent> {
        self.update()
    }
}
