use log::{info, warn};

use crate::{Packet, PacketParseError};

#[derive(Debug, Clone, Default)]
pub struct Framer {
    data: Vec<u8>,
    next_expected_packet_id: u32,
    next_outgoing_packet_id: u32,
}

impl Framer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            next_expected_packet_id: 0,
            next_outgoing_packet_id: 0,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.data.extend(data);
    }

    pub fn frame(&mut self, payload: Vec<u8>) -> Option<Packet> {
        let packet = Packet::new(self.next_outgoing_packet_id, payload)?;
        self.next_outgoing_packet_id = self.next_outgoing_packet_id.wrapping_add(1);
        Some(packet)
    }

    fn pop_front(&mut self) -> Result<Packet, bool> {
        use PacketParseError::*;
        match Packet::from_bytes(&self.data) {
            Ok((packet, n)) => {
                self.data.drain(..n);
                Ok(packet)
            }
            Err(UnexpectedEof) => Err(false),
            Err(BadChecksum {
                expected,
                calculated,
            }) => {
                warn!(
                    "Bad checksum: expected {}, calculated {}",
                    expected, calculated
                );
                self.data.drain(..1);
                Err(true)
            }
            Err(e) => {
                warn!("Unspecified error: {e:?}, {} bytes remain", self.data.len());
                self.data.drain(..1);
                Err(true)
            }
        }
    }

    pub fn parse(&mut self) -> Vec<Packet> {
        let mut ret = Vec::new();
        loop {
            match self.pop_front() {
                Ok(packet) => {
                    let sno = packet.sequence_no();
                    if self.next_expected_packet_id != sno {
                        warn!(
                            "Bad sequence number; expected {}, got {}",
                            self.next_expected_packet_id, sno
                        );
                        warn!("{} bytes remain", self.data.len());
                    }

                    self.next_expected_packet_id = sno.wrapping_add(1);
                    ret.push(packet)
                }
                Err(true) => continue,
                Err(false) => break,
            }
        }
        ret
    }
}
