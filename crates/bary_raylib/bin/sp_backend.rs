use crate::backend::Backend;
use bary_core::prelude::WallTimer;
use bary_ipc::*;
use bary_sim::{World, WorldDelta, apply_delta, load_world, update_world};
use log::*;

pub struct SinglePlayerBackend {
    world: World,
    sim_speed: u32,
    events: Vec<ServerMessage>,
    step_timer: WallTimer,
}

impl SinglePlayerBackend {
    pub fn new(world: World) -> Self {
        Self {
            world,
            sim_speed: 1,
            events: Vec::new(),
            step_timer: WallTimer::hz(50),
        }
    }

    fn append_event(&mut self, msg: impl Into<ServerMessage>) {
        self.events.push(msg.into());
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
        self.append_event(ServerMessage::HaveNewSave);
        self.request_world_download();
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.sim_speed = speed;
    }

    fn request_world_download(&mut self) {
        self.append_event(ServerMessage::TransactionComplete);
    }

    fn network(&self) -> Option<&ClientNode> {
        None
    }

    fn username(&self) -> Option<&str> {
        None
    }

    fn update(&mut self) -> Vec<ServerMessage> {
        if self.step_timer.tick() {
            for _ in 0..self.sim_speed {
                update_world(&mut self.world);
            }
        }

        std::mem::take(&mut self.events)
    }
}
