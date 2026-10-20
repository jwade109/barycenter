use std::sync::mpsc::{Receiver, Sender};

fn rdev_worker_thread(sender: Sender<rdev::Event>) {
    if let Err(error) = rdev::listen(move |e| {
        sender.send(e).expect("Expected sender to be connected");
    }) {
        println!("Error: {:?}", error)
    }
}

pub struct InputHandler {
    receiver: Receiver<rdev::Event>,
}

impl InputHandler {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(|| rdev_worker_thread(sender));
        Self { receiver }
    }

    pub fn get_events(&self) -> Vec<rdev::Event> {
        self.receiver.try_iter().collect()
    }
}
