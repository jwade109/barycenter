use crate::{ClientId, Message, MessageKind, MessageSource};
use log::{debug, info};
pub use renet::DisconnectReason;
use renet::*;
use renet_netcode::*;
use std::collections::HashMap;
use std::net::*;
use std::time::{Instant, SystemTime};

#[derive(Debug, Clone, Copy)]
pub struct ClientInfo {
    pub rx_count: usize,
    pub tx_count: usize,
}

pub enum ServerEvent {
    Connected(ClientId),
    Disconnected(ClientId, DisconnectReason),
    Message(Message),
}

pub struct ServerNode {
    renet: RenetServer,
    transport: NetcodeServerTransport,
    last_updated: Instant,
    client_info: HashMap<ClientId, ClientInfo>,
}

impl ServerNode {
    pub fn new(host_port: u16) -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), host_port);
        let socket: UdpSocket = UdpSocket::bind(addr).unwrap();

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), host_port),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), host_port),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 252)), host_port),
            ],
            authentication: ServerAuthentication::Unsecure,
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        let renet = RenetServer::new(ConnectionConfig::default());

        Self {
            renet,
            transport,
            last_updated: Instant::now(),
            client_info: HashMap::new(),
        }
    }

    pub fn renet(&self) -> &RenetServer {
        &self.renet
    }

    pub fn client_info(&self) -> impl Iterator<Item = (&ClientId, &ClientInfo)> {
        self.client_info.iter()
    }

    pub fn broadcast(&mut self, msg: MessageKind) {
        let msg = msg.with_source(MessageSource::Server);
        let message = bincode::serialize(&msg).unwrap();
        self.renet
            .broadcast_message(DefaultChannel::ReliableOrdered, message);

        for info in self.client_info.values_mut() {
            info.tx_count += 1;
        }
    }

    pub fn send(&mut self, id: ClientId, kind: MessageKind) {
        let message = bincode::serialize(&kind.with_source(MessageSource::Server)).unwrap();
        self.renet
            .send_message(id.0, DefaultChannel::ReliableOrdered, message);

        self.client_info
            .entry(id)
            .and_modify(|c| c.tx_count += 1)
            .or_insert(ClientInfo {
                rx_count: 0,
                tx_count: 1,
            });
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<ServerEvent> {
        let mut messages = Vec::new();

        let now = Instant::now();
        let duration = now - self.last_updated;
        self.renet.update(duration);
        self.transport.update(duration, &mut self.renet).unwrap();

        while let Some(event) = self.renet.get_event() {
            match event {
                renet::ServerEvent::ClientConnected { client_id } => {
                    let event = ServerEvent::Connected(ClientId(client_id));
                    messages.push(event);
                }
                renet::ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!("Client disconnected: {}", client_id);
                    self.client_info.remove(&ClientId(client_id));

                    let event = ServerEvent::Disconnected(ClientId(client_id), reason);
                    messages.push(event);
                }
            }
        }

        for client_id in self.renet.clients_id() {
            while let Some(message) = self
                .renet
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                if let Ok(message) = bincode::deserialize::<Message>(&message) {
                    debug!("Received message from client {}: {:?}", client_id, message);
                    self.client_info
                        .entry(ClientId(client_id))
                        .and_modify(|c| c.rx_count += 1)
                        .or_insert(ClientInfo {
                            rx_count: 1,
                            tx_count: 0,
                        });

                    let event = ServerEvent::Message(message);
                    messages.push(event);
                }
            }
        }

        self.transport.send_packets(&mut self.renet);

        self.last_updated = now;

        messages
    }
}

#[cfg(test)]
mod tests {
    use bary_core::prelude::TableIdent;

    use super::super::*;
    use std::time::Duration;

    #[test]
    fn server_send_to() {
        let mut server = ServerNode::new(5000);
        let mut c1 = ClientNode::localhost(5000);
        let mut c2 = ClientNode::localhost(5000);

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update();
            _ = c1.update();
            _ = c2.update();
            std::thread::sleep(dur);
        }

        assert_eq!(server.renet().connected_clients(), 2);
        assert!(c1.is_connected());
        assert!(c2.is_connected());

        server.send(c1.id(), MessageKind::Ack);

        _ = server.update();

        // std::thread::sleep(dur);

        let m1 = c1.update();
        let m2 = c2.update();

        assert!(m2.is_empty());

        let msg = m1.first().unwrap();

        assert!(msg.is_ack());
    }

    #[test]
    fn multiple_messages() {
        let mut server = ServerNode::new(8000);
        let mut client = ClientNode::localhost(8000);

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update();
            _ = client.update();
            std::thread::sleep(dur);
        }

        assert_eq!(server.renet().connected_clients(), 1);
        assert!(client.is_connected());

        server.broadcast(MessageKind::Ack);
        server.broadcast(MessageKind::PrintEntityInfo(TableIdent::Grids));
        server.send(
            client.id(),
            MessageKind::PrintEntityInfo(TableIdent::Computers),
        );

        _ = server.update();

        let msgs = client.update();

        assert_eq!(msgs.len(), 3);

        assert_eq!(msgs[0].source, MessageSource::Server);
        assert_eq!(msgs[1].source, MessageSource::Server);
        assert_eq!(msgs[2].source, MessageSource::Server);
    }

    #[test]
    fn test_connection() {
        let mut server = ServerNode::new(7000);
        let mut client = ClientNode::localhost(7000);

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update();
            _ = client.update();
            std::thread::sleep(dur);
        }

        assert_eq!(server.renet().connected_clients(), 1);
        assert!(client.is_connected());

        server.broadcast(MessageKind::Ack);
        _ = server.update();

        let msgs = client.update();
        let msg = msgs.first().unwrap();
        assert!(msg.is_ack());
    }
}
