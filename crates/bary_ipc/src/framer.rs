use crate::{Packet, PacketParseError};

pub struct Framer {
    data: Vec<u8>,
}

impl Framer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.data.extend(data);
    }

    fn pop_front(&mut self) -> Result<Packet, bool> {
        use PacketParseError::*;
        match Packet::from_bytes(&self.data) {
            Ok((packet, n)) => {
                self.data.drain(..n);
                Ok(packet)
            }
            Err(UnexpectedEof) => Err(false),
            Err(e) => {
                self.data.drain(..1);
                Err(true)
            }
        }
    }

    pub fn parse(&mut self) -> Vec<Packet> {
        let mut ret = Vec::new();
        loop {
            match self.pop_front() {
                Ok(packet) => ret.push(packet),
                Err(true) => continue,
                Err(false) => break,
            }
        }
        ret
    }
}
