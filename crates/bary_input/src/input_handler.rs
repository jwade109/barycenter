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

    pub fn update(&mut self, is_focused: bool) -> InputState {
        self.input.on_frame_boundary();
        for event in self.receiver.try_iter() {
            self.input.process_rdev_event(&event, is_focused);
        }
        self.input.clone()
    }
}
