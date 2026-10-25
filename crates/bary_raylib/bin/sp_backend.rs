use crate::backend::Backend;
use bary_ipc::*;
use bary_sim::{World, WorldDelta, apply_delta, load_world};
use log::*;

pub struct SinglePlayerBackend {
    world: World,
    sim_speed: u32,
    events: Vec<Message>,
}

impl SinglePlayerBackend {
    pub fn new(world: World) -> Self {
        Self {
            world,
            sim_speed: 1,
            events: Vec::new(),
        }
    }

    fn append_event(&mut self, msg: MessageKind) {
        let msg = Message::new(MessageSource::Server, msg);
        self.events.push(msg);
    }
}

impl Backend for SinglePlayerBackend {
    fn world(&self) -> &World {
        &self.world
    }

    fn request_delta(&mut self, delta: WorldDelta) {
        _ = apply_delta(&mut self.world, delta);
    }

    fn load_save(&mut self, path: String) {
        match load_world(path) {
            Ok(world) => {
                self.world = world;
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
        self.append_event(MessageKind::HaveNewSave);
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.sim_speed = speed;
    }

    fn request_world_download(&mut self) {
        self.append_event(MessageKind::FinishWorldDownload);
    }

    fn network(&self) -> Option<&ClientNode> {
        None
    }

    fn username(&self) -> Option<&str> {
        None
    }

    fn update(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.events)
    }
}
