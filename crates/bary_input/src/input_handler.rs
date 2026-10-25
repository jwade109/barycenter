use crate::InputState;
use std::sync::mpsc::{Receiver, Sender};

fn rdev_worker_thread(sender: Sender<rdev::Event>) {
    if let Err(error) = rdev::listen(move |e| {
        _ = sender.send(e);
    }) {
        println!("Error: {:?}", error)
    }
}

pub struct InputHandler {
    receiver: Receiver<rdev::Event>,
    input: InputState,
}

impl InputHandler {
    pub fn new() -> Self {
        let input = InputState::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(|| rdev_worker_thread(sender));
        Self { receiver, input }
    }

    pub fn get_events(&self) -> Vec<rdev::Event> {
        self.receiver.try_iter().collect()
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn update(&mut self, is_focused: bool) {
        for event in self.get_events() {
            self.input.process_rdev_event(&event, is_focused);
        }
    }

    pub fn on_frame_boundary(&mut self) {
        self.input.on_frame_boundary();
    }
}
