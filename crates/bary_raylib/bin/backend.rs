use bary_ipc::{ClientNode, ServerMessage};
use bary_sim::{ExternalControls, World, WorldDelta};

pub trait Backend {
    fn world(&self) -> &World;

    fn request_delta(&mut self, delta: WorldDelta);

    fn load_save(&mut self, path: String);

    fn set_sim_speed(&mut self, speed: u32);

    fn request_world_download(&mut self);

    fn network(&self) -> Option<&ClientNode>;

    fn username(&self) -> Option<&str>;

    fn update(&mut self, controls: &ExternalControls) -> Vec<ServerMessage>;
}
