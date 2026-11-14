#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Packet {
    sequence_no: u32,
    payload: Vec<u8>,
}

const LEADING_BYTES: u16 = 0xDB11;
const TRAILING_BYTES: u16 = 0xDBEE;

#[derive(Debug, Clone, Copy)]
pub enum PacketParseError {
    UnexpectedEof,
    BadPrefix(u16),
    BadSuffix(u16),
    BadChecksum { expected: u16, calculated: u16 },
}

fn calculate_checksum(bytes: &[u8]) -> u16 {
    let mut sum_a = 0u8;
    let mut sum_b = 0u8;

    for i in (0..bytes.len()).step_by(2) {
        let a = *bytes.get(i).unwrap_or(&0);
        let b = *bytes.get(i + 1).unwrap_or(&0);

        sum_a = sum_a.wrapping_add(a);
        sum_b = sum_b.wrapping_add(b);
    }

    u16::from_be_bytes([sum_a, sum_b])
}

fn get_u16(bytes: &[u8], offset: usize) -> Result<u16, PacketParseError> {
    use PacketParseError::*;
    let a = *bytes.get(offset + 0).ok_or(UnexpectedEof)?;
    let b = *bytes.get(offset + 1).ok_or(UnexpectedEof)?;
    Ok(u16::from_be_bytes([a, b]))
}

fn get_u32(bytes: &[u8], offset: usize) -> Result<u32, PacketParseError> {
    use PacketParseError::*;
    let a = *bytes.get(offset + 0).ok_or(UnexpectedEof)?;
    let b = *bytes.get(offset + 1).ok_or(UnexpectedEof)?;
    let c = *bytes.get(offset + 2).ok_or(UnexpectedEof)?;
    let d = *bytes.get(offset + 3).ok_or(UnexpectedEof)?;
    Ok(u32::from_be_bytes([a, b, c, d]))
}

impl Packet {
    pub fn new(sequence_no: u32, payload: Vec<u8>) -> Option<Self> {
        if payload.len() > u16::MAX as usize - 150 {
            return None;
        }

        Some(Self {
            sequence_no,
            payload,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), PacketParseError> {
        use PacketParseError::*;

        let prefix = get_u16(bytes, 0)?;
        if prefix != LEADING_BYTES {
            return Err(BadPrefix(prefix));
        }

        const OFFSET_SEQUENCE_NO: usize = 2;
        const OFFSET_CHECKSUM: usize = 6;
        const OFFSET_PAYLOAD_LEN: usize = 8;
        const OFFSET_PAYLOAD: usize = 10;

        let sequence_no = get_u32(bytes, OFFSET_SEQUENCE_NO)?;
        let actual_cksm = get_u16(bytes, OFFSET_CHECKSUM)?;
        let payload_length = get_u16(bytes, OFFSET_PAYLOAD_LEN)? as usize;

        let suffix = get_u16(bytes, OFFSET_PAYLOAD + payload_length)?;
        if suffix != TRAILING_BYTES {
            return Err(BadSuffix(suffix));
        }

        let payload = bytes
            .get(OFFSET_PAYLOAD..OFFSET_PAYLOAD + payload_length)
            .ok_or(UnexpectedEof)?
            .to_vec();

        let calc_cksm = calculate_checksum(&payload);

        if actual_cksm != calc_cksm {
            return Err(BadChecksum {
                expected: actual_cksm,
                calculated: calc_cksm,
            });
        }

        let n_consumed = OFFSET_PAYLOAD + payload.len() + 2;

        Ok((
            Self {
                sequence_no,
                payload,
            },
            n_consumed,
        ))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        _ = self.write(&mut buf);
        buf
    }

    pub fn write(&self, sink: &mut impl std::io::Write) -> std::io::Result<usize> {
        let mut sum = 0;
        let checksum = calculate_checksum(&self.payload);
        let len = self.payload.len().min(u16::MAX as usize) as u16;
        sum += sink.write(&LEADING_BYTES.to_be_bytes())?;
        sum += sink.write(&self.sequence_no.to_be_bytes())?;
        sum += sink.write(&checksum.to_be_bytes())?;
        sum += sink.write(&len.to_be_bytes())?;
        sum += sink.write(&self.payload)?;
        sum += sink.write(&TRAILING_BYTES.to_be_bytes())?;
        Ok(sum)
    }

    pub fn unwrap(self) -> Vec<u8> {
        self.payload
    }

    pub fn sequence_no(&self) -> u32 {
        self.sequence_no
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = PacketParseError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Packet::from_bytes(value).map(|p| p.0)
    }
}

#[cfg(test)]
mod tests {

    use bary_core::prelude::randint;

    use super::*;

    #[test]
    fn packet_to_bytes() {
        let packet = Packet::new(12, vec![1, 2, 3, 4, 5]).unwrap();
        let bytes = packet.to_bytes();
        assert_eq!(
            vec![
                0xDB, 0x11, 0, 0, 0, 12, 9, 6, 0, 5, 1, 2, 3, 4, 5, 0xDB, 0xEE
            ],
            bytes
        );

        let parsed = Packet::try_from(bytes.as_slice()).unwrap();

        assert_eq!(parsed, packet);
    }

    #[test]
    fn random_packets() {
        for _ in 0..1000 {
            let len = randint(10, 200) as usize;
            let payload = (0..len).map(|_| randint(0, 255) as u8).collect();
            let packet = Packet::new(0, payload).unwrap();
            let bytes = packet.to_bytes();
            println!("{bytes:02x?}");
            let (parsed, n) = Packet::from_bytes(&bytes).unwrap();
            assert_eq!(packet, parsed);
            assert_eq!(n, len);
        }
    }
}
