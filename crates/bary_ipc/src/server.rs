use crate::{ClientId, Framer, Packet, ServerRequest, ServerResponse};
use bary_core::prelude::randint;
use log::*;
use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Write};
use std::net::*;

pub enum ServerEvent {
    Connected(ClientId),
    Disconnected(ClientId),
    Message(ClientId, ServerRequest),
}

pub struct ServerNode {
    listener: TcpListener,
    clients: BTreeMap<ClientId, Client>,
}

#[derive(Debug)]
struct Client {
    socket: TcpStream,
    addr: SocketAddr,
    framer: Framer,
}

impl Client {
    pub fn send(&mut self, response: ServerResponse) -> std::io::Result<usize> {
        let payload = bincode::serialize(&response).unwrap();
        let packet = Packet::new(payload);
        self.socket.write(&packet.to_bytes())
    }

    pub fn update(&mut self) -> Result<Vec<ServerRequest>, ()> {
        let mut buf = [0; 4096];

        let mut requests = Vec::new();

        'read: loop {
            match self.socket.read(&mut buf) {
                Ok(0) => {
                    warn!("Client {} has disconnected", self.addr);
                    return Err(());
                }
                Ok(n) => {
                    let bytes = &buf[0..n];
                    self.framer.push(bytes);
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {
                        break 'read;
                    }
                    ErrorKind::ConnectionReset => {
                        warn!("Client {} has disconnected", self.addr);
                        return Err(());
                    }
                    e => {
                        error!("Got error reading from socket: {e:?}");
                    }
                },
            }
        }

        for packet in self.framer.parse() {
            if let Ok(request) = bincode::deserialize::<ServerRequest>(packet.payload()) {
                info!("Got a request: {request:?}");
                requests.push(request);
            } else {
                warn!("Unable to parse packet as request");
            }
        }

        Ok(requests)
    }
}

impl ServerNode {
    pub fn new(host_port: u16) -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), host_port);
        info!("Hosting server on {addr}");
        let listener = TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();

        Self {
            listener,
            clients: BTreeMap::new(),
        }
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    pub fn broadcast(&mut self, msg: ServerResponse) {
        for client in self.clients.values_mut() {
            _ = client.send(msg.clone());
        }
    }

    pub fn send(&mut self, id: ClientId, response: ServerResponse) {
        if let Some(client) = self.clients.get_mut(&id) {
            _ = client.send(response);
        }
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<ServerEvent> {
        let mut messages = Vec::new();

        while let Ok((stream, addr)) = self.listener.accept() {
            let client = Client {
                socket: stream,
                addr,
                framer: Framer::new(),
            };
            warn!("Got a client: {client:?}");
            let id = ClientId(randint(1, 1000000) as u64);
            self.clients.insert(id, client);
            messages.push(ServerEvent::Connected(id));
        }

        self.clients.retain(|id, c| match c.update() {
            Ok(msgs) => {
                for msg in msgs {
                    messages.push(ServerEvent::Message(*id, msg));
                }
                true
            }
            Err(()) => {
                messages.push(ServerEvent::Disconnected(*id));
                false
            }
        });

        messages
    }
}
