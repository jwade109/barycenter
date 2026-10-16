use anyhow::Result;
use bary_ipc::{Framer, Packet};
use clap::Parser;
use log::*;
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

fn socket_thread(socket: UdpSocket, responses: Sender<Packet>, requests: Receiver<String>) {
    let mut buf = [0; 4096];
    let mut framer = Framer::new();
    loop {
        for req in requests.try_iter() {
            let payload = req.as_bytes();
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
            let message = String::from_utf8_lossy(resp.payload()).to_string();
            println!("Got message: {message}");
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("foo.txt")?;
            file.write(&resp.to_bytes())?;
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

    let mut input = String::new();

    loop {
        std::io::stdin().read_line(&mut input)?;

        let s = input.trim().to_string();

        println!("Got: {}", s);

        requests_sender.send(s)?;
    }
}
