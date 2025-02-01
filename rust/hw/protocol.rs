//! FPGA communication protocol implementation
//!
//! This module defines the protocol for communicating with the FPGA hardware,
//! including packet formats and serialization.

use std::io::{self, Read, Write};
use byteorder::{ByteOrder, BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Serialize, Deserialize};

use crate::types::{UnitId, Operation, Status, VectorBlock};
use crate::error::{Result, HardwareError};

/// Protocol version
const PROTOCOL_VERSION: u8 = 1;

/// Maximum packet size
const MAX_PACKET_SIZE: usize = 1024;

/// Packet type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Command = 0x01,
    Response = 0x02,
    Error = 0xFF,
}

/// Packet header structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    /// Protocol version
    version: u8,
    /// Packet type
    packet_type: u8,
    /// Unit ID
    unit_id: u16,
    /// Packet sequence number
    sequence: u32,
    /// Payload length
    length: u16,
}

impl PacketHeader {
    /// Create a new packet header
    pub fn new(packet_type: PacketType, unit_id: UnitId, sequence: u32, length: u16) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type: packet_type as u8,
            unit_id: unit_id.raw() as u16,
            sequence,
            length,
        }
    }

    /// Serialize header to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(10);
        buffer.write_u8(self.version)?;
        buffer.write_u8(self.packet_type)?;
        buffer.write_u16::<BigEndian>(self.unit_id)?;
        buffer.write_u32::<BigEndian>(self.sequence)?;
        buffer.write_u16::<BigEndian>(self.length)?;
        Ok(buffer)
    }

    /// Deserialize header from bytes
    pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            version: reader.read_u8()?,
            packet_type: reader.read_u8()?,
            unit_id: reader.read_u16::<BigEndian>()?,
            sequence: reader.read_u32::<BigEndian>()?,
            length: reader.read_u16::<BigEndian>()?,
        })
    }
}

/// Command packet payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    /// Operation to execute
    pub operation: Operation,
    /// Source unit ID (if applicable)
    pub source_unit: Option<u16>,
    /// Configuration data
    pub config: Vec<u8>,
}

/// Response packet payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    /// Operation status
    pub status: Status,
    /// Response data (if any)
    pub data: Option<VectorBlock>,
}

/// Protocol encoder/decoder
pub struct ProtocolCodec {
    sequence: u32,
}

impl ProtocolCodec {
    /// Create new protocol codec
    pub fn new() -> Self {
        Self { sequence: 0 }
    }

    /// Encode command into packet
    pub fn encode_command(
        &mut self,
        unit_id: UnitId,
        payload: CommandPayload
    ) -> Result<Vec<u8>> {
        let payload_bytes = bincode::serialize(&payload)
            .map_err(|e| HardwareError::Protocol(e.to_string()))?;

        if payload_bytes.len() > MAX_PACKET_SIZE {
            return Err(HardwareError::Protocol(
                "Payload too large".to_string()
            ).into());
        }

        let header = PacketHeader::new(
            PacketType::Command,
            unit_id,
            self.sequence,
            payload_bytes.len() as u16
        );
        self.sequence += 1;

        let mut packet = header.serialize()?;
        packet.extend(payload_bytes);
        Ok(packet)
    }

    /// Decode response from packet
    pub fn decode_response(&self, bytes: &[u8]) -> Result<(PacketHeader, ResponsePayload)> {
        let mut cursor = io::Cursor::new(bytes);
        
        let header = PacketHeader::deserialize(&mut cursor)?;
        if header.version != PROTOCOL_VERSION {
            return Err(HardwareError::Protocol(
                format!("Unsupported protocol version: {}", header.version)
            ).into());
        }

        let payload: ResponsePayload = bincode::deserialize(&bytes[cursor.position() as usize..])
            .map_err(|e| HardwareError::Protocol(e.to_string()))?;

        Ok((header, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header() {
        let unit_id = UnitId::new(1).unwrap();
        let header = PacketHeader::new(PacketType::Command, unit_id, 42, 100);
        
        // Test serialization
        let bytes = header.serialize().unwrap();
        
        // Test deserialization
        let mut cursor = io::Cursor::new(bytes);
        let decoded = PacketHeader::deserialize(&mut cursor).unwrap();
        
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.packet_type, PacketType::Command as u8);
        assert_eq!(decoded.unit_id, 1);
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.length, 100);
    }

    #[test]
    fn test_protocol_codec() {
        let mut codec = ProtocolCodec::new();
        let unit_id = UnitId::new(0).unwrap();
        
        // Create command payload
        let payload = CommandPayload {
            operation: Operation::Nop,
            source_unit: None,
            config: vec![],
        };
        
        // Test encoding
        let packet = codec.encode_command(unit_id, payload).unwrap();
        
        // Create response payload
        let response = ResponsePayload {
            status: Status::Success,
            data: None,
        };
        
        // Serialize response
        let response_bytes = bincode::serialize(&response).unwrap();
        let mut response_packet = PacketHeader::new(
            PacketType::Response,
            unit_id,
            0,
            response_bytes.len() as u16
        ).serialize().unwrap();
        response_packet.extend(response_bytes);
        
        // Test decoding
        let (header, decoded_response) = codec.decode_response(&response_packet).unwrap();
        assert_eq!(header.packet_type, PacketType::Response as u8);
        assert!(matches!(decoded_response.status, Status::Success));
    }
}