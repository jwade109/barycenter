use anyhow::Result;
use bary_ipc::{Framer, Packet};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    net::UdpSocket,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

#[derive(Parser)]
struct Args {
    local_addr: String,
    remote_addr: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Message {
    username: String,
    message: String,
}

impl Message {
    fn new(username: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            message: message.into(),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let s = serde_json::to_string(self).unwrap();
        s.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = String::from_utf8_lossy(bytes).to_string();
        serde_json::from_str(&s).ok()
    }
}

fn socket_thread(socket: UdpSocket, responses: Sender<Packet>, requests: Receiver<Message>) {
    let mut buf = [0; 4096];
    let mut framer = Framer::new();
    loop {
        for req in requests.try_iter() {
            let payload = req.to_bytes();
            let packet = Packet::new(payload.to_vec());
            let bytes = packet.to_bytes();
            if let Err(e) = socket.send(&bytes) {
                println!("Failed to send: {e}");
            }
        }

        while let Ok(n) = socket.recv(&mut buf) {
            framer.push(&buf[..n]);
        }

        for packet in framer.parse() {
            responses.send(packet).unwrap();
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn receiver_thread(responses: Receiver<Packet>) -> Result<()> {
    loop {
        for resp in responses.try_iter() {
            let message = Message::from_bytes(resp.payload());
            println!("Got message: {message:?}");
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let socket = UdpSocket::bind(args.local_addr)?;
    socket.connect(args.remote_addr)?;
    socket.set_nonblocking(true)?;

    let (requests_sender, requests_receiver) = channel();
    let (responses_sender, responses_receiver) = channel();

    std::thread::spawn(|| socket_thread(socket, responses_sender, requests_receiver));
    std::thread::spawn(|| receiver_thread(responses_receiver));

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let s = input.trim().to_string();

        let message = Message::new("username", s);

        println!("Got: {:?}", message);

        requests_sender.send(message)?;
    }
}
