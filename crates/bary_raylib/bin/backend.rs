use bary_core::prelude::Ent;
use bary_ipc::{ClientNode, ServerResponse};
use bary_sim::{ExternalControls, World, WorldDelta};

#[derive(Debug)]
pub enum BackendEvent {
    FinishedWorldDownload(String),
    Server(ServerResponse),
}

impl From<ServerResponse> for BackendEvent {
    fn from(value: ServerResponse) -> Self {
        Self::Server(value)
    }
}

pub trait Backend {
    fn world(&self) -> &World;

    fn request_delta(&mut self, delta: WorldDelta);

    fn load_save(&mut self, path: String);

    fn set_sim_speed(&mut self, speed: u32);

    fn request_world_download(&mut self);

    fn network(&self) -> Option<&ClientNode>;

    fn username(&self) -> Option<&str>;

    fn allocated_player_id(&self) -> Option<Ent>;

    fn update(&mut self, controls: &ExternalControls) -> Vec<BackendEvent>;
}
