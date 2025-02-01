//! Processing unit management implementation
//!
//! This module handles the management and state tracking of individual
//! processing units in the accelerator.

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;
use futures::future::join_all;

use crate::types::{UnitId, Operation, Status, VectorBlock};
use crate::error::{Result, UnitError, HardwareError};
use crate::hw::fpga::{FpgaInterface, CommandPacket};

/// Unit state information
#[derive(Debug, Clone, PartialEq)]
pub struct UnitState {
    /// Current operation status
    pub status: Status,
    /// Current operation (if any)
    pub current_op: Option<Operation>,
    /// Error information (if any)
    pub error: Option<String>,
}

impl Default for UnitState {
    fn default() -> Self {
        Self {
            status: Status::Success,
            current_op: None,
            error: None,
        }
    }
}

/// Processing unit manager
#[derive(Clone)]
pub struct UnitManager {
    /// FPGA interface
    fpga: Arc<Mutex<Box<dyn FpgaInterface>>>,
    /// Unit states
    states: Arc<RwLock<Vec<UnitState>>>,
}

impl UnitManager {
    /// Create a new unit manager
    pub fn new(fpga: Box<dyn FpgaInterface>) -> Self {
        let states = (0..crate::types::UNIT_COUNT)
            .map(|_| UnitState::default())
            .collect();

        Self {
            fpga: Arc::new(Mutex::new(fpga)),
            states: Arc::new(RwLock::new(states)),
        }
    }

    /// Initialize all units
    pub async fn initialize(&self) -> Result<()> {
        let mut fpga = self.fpga.lock().await;
        fpga.initialize().await?;
        Ok(())
    }

    /// Get state of specific unit
    pub async fn get_state(&self, unit_id: UnitId) -> Result<UnitState> {
        let states = self.states.read().await;
        states.get(unit_id.raw())
            .cloned()
            .ok_or_else(|| UnitError::InvalidId(unit_id.raw()).into())
    }

    /// Execute operation on specific unit
    pub async fn execute(&self, unit_id: UnitId, op: Operation) -> Result<()> {
        // Check unit state
        {
            let states = self.states.read().await;
            if let Some(state) = states.get(unit_id.raw()) {
                if state.status == Status::InProgress {
                    return Err(UnitError::Busy(unit_id).into());
                }
            }
        }

        // Update state
        {
            let mut states = self.states.write().await;
            if let Some(state) = states.get_mut(unit_id.raw()) {
                state.status = Status::InProgress;
                state.current_op = Some(op);
                state.error = None;
            }
        }

        // Send command to FPGA
        let cmd = CommandPacket {
            unit_id,
            source_id: match op {
                Operation::Copy { from } | Operation::Add { from } => Some(from),
                _ => None,
            },
            operation: op,
            config: vec![],
        };

        let mut fpga = self.fpga.lock().await;
        fpga.send_command(cmd).await?;

        // Wait for and process response
        let response = fpga.receive_response().await?;
        
        // Update state with response
        {
            let mut states = self.states.write().await;
            if let Some(state) = states.get_mut(unit_id.raw()) {
                state.status = response.status;
                if response.status == Status::Failed {
                    state.error = Some("Operation failed".to_string());
                }
            }
        }

        Ok(())
    }

    /// Execute operations on multiple units in parallel
    pub async fn execute_parallel(&self, operations: Vec<(UnitId, Operation)>) -> Result<()> {
        let futures: Vec<_> = operations.into_iter()
            .map(|(unit_id, op)| self.execute(unit_id, op))
            .collect();

        // Execute all operations and collect results
        let results = join_all(futures).await;
        
        // Check for any errors
        for result in results {
            if let Err(e) = result {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Reset specific unit
    pub async fn reset_unit(&self, unit_id: UnitId) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(unit_id.raw()) {
            *state = UnitState::default();
        }
        Ok(())
    }

    /// Reset all units
    pub async fn reset_all(&self) -> Result<()> {
        let mut states = self.states.write().await;
        for state in states.iter_mut() {
            *state = UnitState::default();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fpga::MockFpga;

    #[tokio::test]
    async fn test_unit_manager() {
        let manager = UnitManager::new(Box::new(MockFpga::default()));
        
        // Test initialization
        assert!(manager.initialize().await.is_ok());

        // Test unit state
        let unit_id = UnitId::new(0).unwrap();
        let state = manager.get_state(unit_id).await.unwrap();
        assert_eq!(state.status, Status::Success);

        // Test operation execution
        let op = Operation::Nop;
        assert!(manager.execute(unit_id, op).await.is_ok());

        // Test parallel execution
        let ops = vec![
            (UnitId::new(0).unwrap(), Operation::Nop),
            (UnitId::new(1).unwrap(), Operation::Nop),
        ];
        assert!(manager.execute_parallel(ops).await.is_ok());
    }

    #[tokio::test]
    async fn test_unit_reset() {
        let manager = UnitManager::new(Box::new(MockFpga::default()));
        assert!(manager.initialize().await.is_ok());

        let unit_id = UnitId::new(0).unwrap();
        assert!(manager.reset_unit(unit_id).await.is_ok());
        assert!(manager.reset_all().await.is_ok());
    }
}