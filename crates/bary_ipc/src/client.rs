use crate::ClientId;
use crate::*;
use log::{info, trace, warn};
use std::io::{Read, Write};
use std::net::*;
use std::time::{Duration, SystemTime};

const CLIENT_BIND_ADDRESS: &'static str = "0.0.0.0:0";

#[derive(Clone, Copy, Debug)]
pub struct ClientStatistics {
    pub is_connected: bool,
    pub rtt: f64,
    pub packet_loss: f64,
    pub bytes_sent_per_second: f64,
    pub bytes_received_per_second: f64,
    pub rx_count: usize,
    pub tx_count: usize,
}

#[derive(Debug)]
pub struct ClientNode {
    id: ClientId,
    errors: usize,
    server_addr: SocketAddr,
    framer: Framer,
    socket: Option<TcpStream>,
}

fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

impl ClientNode {
    pub fn localhost(server_port: u16) -> Self {
        Self::new(127, 0, 0, 1, server_port)
    }

    fn from_addr(server_addr: SocketAddr) -> Self {
        info!("Connecting on {server_addr}");

        let new_socket = TcpStream::connect_timeout(&server_addr, Duration::from_millis(100)).ok();

        if let Some(socket) = new_socket.as_ref() {
            socket.set_nonblocking(true).unwrap();
        }

        info!("New socket: {new_socket:?}");

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;

        Self {
            id: ClientId(client_id),
            errors: 0,
            server_addr,
            socket: new_socket,
            framer: Framer::new(),
        }
    }

    pub fn with_str_addr(addr: &str) -> Result<Self, AddrParseError> {
        let addr: SocketAddr = addr.parse()?;
        Ok(Self::from_addr(addr))
    }

    pub fn new(a: u8, b: u8, c: u8, d: u8, server_port: u16) -> Self {
        let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), server_port);
        Self::from_addr(server_addr)
    }

    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn errors(&self) -> usize {
        self.errors
    }

    pub fn is_connected(&self) -> bool {
        self.socket.is_some()
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.server_addr
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<ServerResponse> {
        let mut messages = Vec::new();

        let mut buf = [0; 4096];

        if let Some(socket) = self.socket.as_mut() {
            loop {
                match socket.read(&mut buf) {
                    Ok(0) => {
                        // disconnect
                        break;
                    }
                    Ok(n) => {
                        self.framer.push(&buf[..n]);
                    }
                    Err(e) => {
                        // I don't care
                        break;
                    }
                }
            }
        }

        for packet in self.framer.parse() {
            if let Ok(response) = bincode::deserialize::<ServerResponse>(packet.payload()) {
                messages.push(response);
            } else {
                warn!("Unable to parse packet as response");
            }
        }

        messages
    }

    pub fn send(&mut self, msg: impl Into<ServerRequest>) {
        let msg = msg.into();
        let bytes = bincode::serialize(&msg).unwrap();
        let packet = Packet::new(bytes.clone());
        trace!("Sending: {packet:?}");
        self.socket.as_mut().map(|s| s.write(&packet.to_bytes()));
    }
}
