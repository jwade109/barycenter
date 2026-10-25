use crate::ClientId;
use crate::*;
use renet::*;
use renet_netcode::*;
use std::net::*;
use std::time::Instant;

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
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_updated: Instant,
    rx_count: usize,
    tx_count: usize,
}

impl ClientNode {
    pub fn localhost(server_port: u16) -> Self {
        Self::new(127, 0, 0, 1, server_port)
    }

    fn from_addr(server_addr: SocketAddr) -> Self {
        let client = RenetClient::new(ConnectionConfig::default());

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;

        let socket = UdpSocket::bind(CLIENT_BIND_ADDRESS).unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        Self {
            id: ClientId(client_id),
            errors: 0,
            server_addr,
            client,
            transport,
            last_updated,
            tx_count: 0,
            rx_count: 0,
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

    pub fn rx_count(&self) -> usize {
        self.rx_count
    }

    pub fn tx_count(&self) -> usize {
        self.tx_count
    }

    pub fn reconnect(&mut self) {
        if self.is_connected() {
            return;
        }

        let client = RenetClient::new(ConnectionConfig::default());

        let socket = UdpSocket::bind(CLIENT_BIND_ADDRESS).unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr: self.server_addr,
            client_id: self.id.0,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        *self = Self {
            id: self.id,
            errors: self.errors,
            server_addr: self.server_addr,
            client,
            transport,
            last_updated,
            tx_count: self.tx_count,
            rx_count: self.rx_count,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.server_addr
    }

    pub fn stats(&self) -> ClientStatistics {
        let is_connected = self.client.is_connected();
        let info = self.client.network_info();

        ClientStatistics {
            is_connected,
            rtt: info.rtt,
            packet_loss: info.packet_loss,
            bytes_sent_per_second: info.bytes_sent_per_second,
            bytes_received_per_second: info.bytes_received_per_second,
            tx_count: self.tx_count,
            rx_count: self.rx_count,
        }
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<ServerMessage> {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;

        self.client.update(duration);

        if self.transport.update(duration, &mut self.client).is_err() {
            self.errors += 1;
            return Vec::new();
        }

        let mut messages = Vec::new();

        while let Some(bytes) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
            let msg: ServerMessage = bincode::deserialize(&bytes).unwrap();
            self.rx_count += 1;
            messages.push(msg);
        }

        if let Err(_) = self.transport.send_packets(&mut self.client) {
            self.errors += 1;
        }

        messages
    }

    pub fn send(&mut self, msg: impl Into<ClientMessage>) {
        let msg = msg.into();
        let bytes = bincode::serialize(&msg).unwrap();
        self.client
            .send_message(DefaultChannel::ReliableOrdered, bytes);
        self.tx_count += 1;
    }

    pub fn disconnect(&mut self) {
        self.client.disconnect();
    }
}
