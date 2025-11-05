use crate::client::constants::PACKET_SIZE;
use crate::client::error::{ClientError, Result};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceNumber(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub sequence: SequenceNumber,
}

impl Packet {
    pub fn new(sequence: SequenceNumber) -> Self {
        Self { sequence }
    }

    pub fn encode(&self) -> [u8; PACKET_SIZE] {
        self.sequence.0.to_le_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < PACKET_SIZE {
            debug!(
                expected = PACKET_SIZE,
                actual = bytes.len(),
                "Invalid packet size"
            );
            return Err(ClientError::Protocol(format!(
                "Invalid packet size: expected {}, got {}",
                PACKET_SIZE,
                bytes.len()
            )));
        }

        let mut buf = [0u8; PACKET_SIZE];
        buf.copy_from_slice(&bytes[..PACKET_SIZE]);
        let seq = u64::from_le_bytes(buf);

        debug!(sequence = seq, "Packet decoded successfully");

        Ok(Packet {
            sequence: SequenceNumber(seq),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_encode_decode() {
        let original = Packet::new(SequenceNumber(12345));
        let encoded = original.encode();
        let decoded = Packet::decode(&encoded).unwrap();
        assert_eq!(original.sequence, decoded.sequence);
    }

    #[test]
    fn test_packet_invalid_size() {
        let buf = [0u8; 4];
        assert!(Packet::decode(&buf).is_err());
    }

    #[test]
    fn test_packet_roundtrip() {
        let seq = SequenceNumber(12345);
        let packet = Packet::new(seq);
        let encoded = packet.encode();
        let decoded = Packet::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, seq);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_packet_encode_decode_property(seq in 0u64..u64::MAX) {
            let sequence = SequenceNumber(seq);
            let packet = Packet::new(sequence);
            let encoded = packet.encode();
            let decoded = Packet::decode(&encoded).unwrap();
            prop_assert_eq!(decoded.sequence, sequence);
        }

        #[test]
        fn test_packet_encode_decode_roundtrip(seq in 0u64..u64::MAX) {
            let original = Packet::new(SequenceNumber(seq));
            let encoded = original.encode();
            let decoded = Packet::decode(&encoded).unwrap();
            prop_assert_eq!(original, decoded);
        }
    }
}
