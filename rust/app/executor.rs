//! Operation execution engine
//!
//! This module provides the execution engine for running operations on the FPGA.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, sleep};
use async_trait::async_trait;
use tracing::{info, warn, error};

use crate::domain::{
    operation::{Operation, UnitId, OperationStatus},
    error::{Result, DomainError},
};
use crate::infra::{
    fpga::{FpgaInterface, Command, Response},
    memory::{MemoryManager, BlockId, LockReason},
};

/// Maximum retry attempts for operations
const MAX_RETRIES: u32 = 3;

/// Retry delay in milliseconds
const RETRY_DELAY_MS: u64 = 100;

/// Operation context containing execution details
#[derive(Debug)]
pub struct OperationContext {
    /// Operation to execute
    pub operation: Operation,
    /// Target unit
    pub unit: UnitId,
    /// Associated memory block
    pub block: Option<BlockId>,
    /// Number of retry attempts
    pub retries: u32,
    /// Start timestamp
    pub start_time: std::time::Instant,
}

impl OperationContext {
    /// Create new operation context
    pub fn new(operation: Operation, unit: UnitId) -> Self {
        Self {
            operation,
            unit,
            block: None,
            retries: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Check if operation has exceeded retry limit
    pub fn exceeded_retries(&self) -> bool {
        self.retries >= MAX_RETRIES
    }

    /// Get operation duration
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Operation execution trait
#[async_trait]
pub trait OperationExecutor: Send + Sync {
    /// Execute operation
    async fn execute(&self, context: OperationContext) -> Result<OperationStatus>;
    
    /// Cancel operation
    async fn cancel(&self, unit: UnitId) -> Result<()>;
}

/// Main executor implementation
pub struct Executor {
    fpga: Arc<Mutex<Box<dyn FpgaInterface>>>,
    memory: Arc<MemoryManager>,
    active_operations: Arc<RwLock<Vec<OperationContext>>>,
}

impl Executor {
    /// Create new executor
    pub fn new(
        fpga: Box<dyn FpgaInterface>,
        memory: Arc<MemoryManager>,
    ) -> Self {
        Self {
            fpga: Arc::new(Mutex::new(fpga)),
            memory,
            active_operations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Handle operation preparation
    async fn prepare_operation(&self, context: &mut OperationContext) -> Result<()> {
        // Lock required memory blocks
        if let Some(block_id) = context.block {
            self.memory.lock(
                block_id,
                context.unit,
                LockReason::Writing
            ).await?;
        }

        // Record operation start
        let mut active_ops = self.active_operations.write().await;
        active_ops.push(context.clone());

        Ok(())
    }

    /// Handle operation completion
    async fn complete_operation(
        &self,
        context: &OperationContext,
        status: OperationStatus
    ) -> Result<()> {
        // Unlock memory blocks
        if let Some(block_id) = context.block {
            self.memory.unlock(block_id).await?;
        }

        // Remove from active operations
        let mut active_ops = self.active_operations.write().await;
        active_ops.retain(|op| op.unit != context.unit);

        // Log completion
        match status {
            OperationStatus::Success => {
                info!(
                    "Operation completed successfully: {:?} on unit {}",
                    context.operation,
                    context.unit.raw()
                );
            }
            OperationStatus::Failed { code } => {
                error!(
                    "Operation failed: {:?} on unit {}, error code {}",
                    context.operation,
                    context.unit.raw(),
                    code
                );
            }
        }

        Ok(())
    }

    /// Retry failed operation
    async fn retry_operation(&self, mut context: OperationContext) -> Result<OperationStatus> {
        context.retries += 1;
        warn!(
            "Retrying operation {:?} on unit {}, attempt {}/{}",
            context.operation,
            context.unit.raw(),
            context.retries,
            MAX_RETRIES
        );

        sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
        self.execute(context).await
    }
}

#[async_trait]
impl OperationExecutor for Executor {
    async fn execute(&self, mut context: OperationContext) -> Result<OperationStatus> {
        // Prepare operation
        self.prepare_operation(&mut context).await?;

        // Send command to FPGA
        let mut fpga = self.fpga.lock().await;
        fpga.send_command(Command::Execute {
            unit_id: context.unit,
            operation: context.operation.clone(),
        }).await?;

        // Wait for response
        let response = fpga.receive_response().await?;
        
        match response {
            Response::Status { status, .. } => {
                match status {
                    OperationStatus::Success => {
                        self.complete_operation(&context, status).await?;
                        Ok(status)
                    }
                    OperationStatus::Failed { .. } => {
                        if context.exceeded_retries() {
                            self.complete_operation(&context, status).await?;
                            Ok(status)
                        } else {
                            self.retry_operation(context).await
                        }
                    }
                }
            }
            Response::Error { code, message, .. } => {
                error!(
                    "FPGA error: {} (code: {})",
                    message,
                    code
                );
                Err(DomainError::OperationFailed {
                    operation: format!("{:?}", context.operation),
                    reason: message,
                }.into())
            }
        }
    }

    async fn cancel(&self, unit: UnitId) -> Result<()> {
        // Send cancel command
        let mut fpga = self.fpga.lock().await;
        fpga.send_command(Command::Reset { unit_id: unit }).await?;

        // Clean up any active operations
        let mut active_ops = self.active_operations.write().await;
        if let Some(op) = active_ops.iter().find(|op| op.unit == unit) {
            if let Some(block_id) = op.block {
                self.memory.unlock(block_id).await?;
            }
        }
        active_ops.retain(|op| op.unit != unit);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::fpga::MockFpga;

    #[tokio::test]
    async fn test_operation_execution() {
        let memory = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let executor = Executor::new(
            Box::new(MockFpga::default()),
            memory.clone(),
        );

        // Test successful execution
        let context = OperationContext::new(
            Operation::Copy {
                source: UnitId::new(0).unwrap(),
            },
            UnitId::new(1).unwrap(),
        );

        let status = executor.execute(context).await.unwrap();
        assert!(matches!(status, OperationStatus::Success));

        // Test cancellation
        let unit = UnitId::new(1).unwrap();
        assert!(executor.cancel(unit).await.is_ok());
    }

    #[tokio::test]
    async fn test_operation_retry() {
        let memory = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let executor = Executor::new(
            Box::new(MockFpga::default()),
            memory.clone(),
        );

        let mut context = OperationContext::new(
            Operation::Copy {
                source: UnitId::new(0).unwrap(),
            },
            UnitId::new(1).unwrap(),
        );

        // Simulate retries
        context.retries = MAX_RETRIES - 1;
        let status = executor.execute(context).await.unwrap();
        
        // Even mock FPGA should succeed eventually
        assert!(matches!(status, OperationStatus::Success));
    }
}