use crate::backend::{Backend, BackendEvent};
use bary_core::prelude::WallTimer;
use bary_ipc::*;
use bary_sim::*;
use log::*;

pub struct SinglePlayerBackend {
    world: World,
    sim_speed: u32,
    events: Vec<BackendEvent>,
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

    fn append_event(&mut self, msg: impl Into<BackendEvent>) {
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
        match load_world(&path) {
            Ok(world) => {
                self.world = world;
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
        self.append_event(ServerResponse::HaveNewSave);
        self.append_event(BackendEvent::FinishedWorldDownload(path));
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.sim_speed = speed;
    }

    fn request_world_download(&mut self) {}

    fn network(&self) -> Option<&ClientNode> {
        None
    }

    fn username(&self) -> Option<&str> {
        None
    }

    fn update(&mut self, controls: &ExternalControls) -> Vec<BackendEvent> {
        if self.step_timer.tick() {
            for _ in 0..self.sim_speed {
                let response = update_world_with_user_controls(&mut self.world, controls);
                let events = response.events.take();
                if !events.is_empty() {
                    self.events.extend(
                        events
                            .into_iter()
                            .map(|s| ServerResponse::Simulation(s).into()),
                    );
                }
            }
        }

        std::mem::take(&mut self.events)
    }
}
