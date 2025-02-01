//! FPGA hardware interface and communication implementation
//!
//! This module provides the low-level interface for communicating with the FPGA
//! hardware, including protocol handling and command execution.

use std::sync::Arc;
use tokio::sync::Mutex;
use bytes::{Buf, BufMut, BytesMut};
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::domain::{
    operation::{Operation, UnitId, OperationStatus},
    error::{Result, DomainError},
};

/// Protocol version
const PROTOCOL_VERSION: u8 = 1;
/// Maximum packet size
const MAX_PACKET_SIZE: usize = 1024;

/// Commands that can be sent to FPGA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Execute operation
    Execute {
        unit_id: UnitId,
        operation: Operation,
    },
    /// Query status
    Query {
        unit_id: UnitId,
    },
    /// Reset unit
    Reset {
        unit_id: UnitId,
    },
}

/// Responses from FPGA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Operation status
    Status {
        unit_id: UnitId,
        status: OperationStatus,
    },
    /// Error response
    Error {
        unit_id: UnitId,
        code: u8,
        message: String,
    },
}

/// FPGA communication configuration
#[derive(Debug, Clone)]
pub struct FpgaConfig {
    /// Device path
    pub device: String,
    /// Communication timeout
    pub timeout: std::time::Duration,
}

impl Default for FpgaConfig {
    fn default() -> Self {
        Self {
            device: "/dev/fpga0".to_string(),
            timeout: std::time::Duration::from_secs(1),
        }
    }
}

/// Hardware interface trait
#[async_trait::async_trait]
pub trait FpgaInterface: Send + Sync {
    /// Initialize FPGA connection
    async fn initialize(&mut self, config: &FpgaConfig) -> Result<()>;
    
    /// Send command to FPGA
    async fn send_command(&mut self, cmd: Command) -> Result<()>;
    
    /// Receive response from FPGA
    async fn receive_response(&mut self) -> Result<Response>;
    
    /// Check if FPGA is ready
    async fn is_ready(&self) -> bool;
}

/// Real FPGA implementation
pub struct RealFpga {
    config: FpgaConfig,
    device: Option<String>,
    sequence: u32,
    transport: BytesMut,
}

impl RealFpga {
    /// Create new FPGA interface
    pub fn new() -> Self {
        Self {
            config: FpgaConfig::default(),
            device: None,
            sequence: 0,
            transport: BytesMut::with_capacity(MAX_PACKET_SIZE),
        }
    }

    /// Pack command into packet
    fn pack_command(&mut self, cmd: &Command) -> Result<()> {
        self.transport.clear();
        
        // Write header
        self.transport.put_u8(PROTOCOL_VERSION);
        self.transport.put_u32(self.sequence);
        
        // Serialize command
        let cmd_bytes = bincode::serialize(cmd)
            .map_err(|e| DomainError::OperationFailed {
                operation: "serialize command".into(),
                reason: e.to_string(),
            })?;
            
        if cmd_bytes.len() > MAX_PACKET_SIZE - 5 {
            return Err(DomainError::OperationFailed {
                operation: "pack command".into(),
                reason: "command too large".into(),
            });
        }
        
        self.transport.put_slice(&cmd_bytes);
        self.sequence += 1;
        
        Ok(())
    }

    /// Unpack response from packet
    fn unpack_response(&mut self) -> Result<Response> {
        if self.transport.len() < 5 {
            return Err(DomainError::OperationFailed {
                operation: "unpack response".into(),
                reason: "packet too short".into(),
            });
        }

        let version = self.transport.get_u8();
        if version != PROTOCOL_VERSION {
            return Err(DomainError::OperationFailed {
                operation: "unpack response".into(),
                reason: format!("invalid protocol version: {}", version),
            });
        }

        let _sequence = self.transport.get_u32();
        
        bincode::deserialize(&self.transport)
            .map_err(|e| DomainError::OperationFailed {
                operation: "deserialize response".into(),
                reason: e.to_string(),
            })
    }
}

#[async_trait::async_trait]
impl FpgaInterface for RealFpga {
    async fn initialize(&mut self, config: &FpgaConfig) -> Result<()> {
        self.config = config.clone();
        self.device = Some(config.device.clone());
        Ok(())
    }

    async fn send_command(&mut self, cmd: Command) -> Result<()> {
        self.pack_command(&cmd)?;
        // Actual device communication would happen here
        Ok(())
    }

    async fn receive_response(&mut self) -> Result<Response> {
        // Actual device communication would happen here
        // For now just return a mock response
        Ok(Response::Status {
            unit_id: UnitId::new(0).unwrap(),
            status: OperationStatus::Success,
        })
    }

    async fn is_ready(&self) -> bool {
        self.device.is_some()
    }
}

/// Mock FPGA implementation for testing
pub struct MockFpga {
    ready: bool,
    last_command: Arc<Mutex<Option<Command>>>,
}

impl Default for MockFpga {
    fn default() -> Self {
        Self {
            ready: false,
            last_command: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl FpgaInterface for MockFpga {
    async fn initialize(&mut self, _config: &FpgaConfig) -> Result<()> {
        self.ready = true;
        Ok(())
    }

    async fn send_command(&mut self, cmd: Command) -> Result<()> {
        if !self.ready {
            return Err(DomainError::OperationFailed {
                operation: "send command".into(),
                reason: "FPGA not initialized".into(),
            });
        }
        let mut last_cmd = self.last_command.lock().await;
        *last_cmd = Some(cmd);
        Ok(())
    }

    async fn receive_response(&mut self) -> Result<Response> {
        if !self.ready {
            return Err(DomainError::OperationFailed {
                operation: "receive response".into(),
                reason: "FPGA not initialized".into(),
            });
        }
        Ok(Response::Status {
            unit_id: UnitId::new(0).unwrap(),
            status: OperationStatus::Success,
        })
    }

    async fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_fpga_protocol() {
        let mut fpga = RealFpga::new();
        let config = FpgaConfig::default();

        // Test initialization
        assert!(fpga.initialize(&config).await.is_ok());
        assert!(fpga.is_ready().await);

        // Test command packing
        let cmd = Command::Execute {
            unit_id: UnitId::new(0).unwrap(),
            operation: Operation::Copy {
                source: UnitId::new(1).unwrap(),
            },
        };
        assert!(fpga.pack_command(&cmd).is_ok());

        // Test response handling
        let response = fpga.receive_response().await.unwrap();
        match response {
            Response::Status { status, .. } => {
                assert!(matches!(status, OperationStatus::Success));
            },
            _ => panic!("Unexpected response type"),
        }
    }

    #[tokio::test]
    async fn test_mock_fpga() {
        let mut fpga = MockFpga::default();
        let config = FpgaConfig::default();

        assert!(fpga.initialize(&config).await.is_ok());
        assert!(fpga.is_ready().await);

        let cmd = Command::Query {
            unit_id: UnitId::new(0).unwrap(),
        };
        assert!(fpga.send_command(cmd).await.is_ok());

        let response = fpga.receive_response().await.unwrap();
        match response {
            Response::Status { status, .. } => {
                assert!(matches!(status, OperationStatus::Success));
            },
            _ => panic!("Unexpected response type"),
        }
    }
}