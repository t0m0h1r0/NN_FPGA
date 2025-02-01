//! Asynchronous API interface
//!
//! This module provides a high-level async API for interacting with
//! the accelerator.

use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::types::{UnitId, Operation, Status, VectorBlock, Activation};
use crate::error::{Result, AccelError};
use crate::hw::unit::UnitManager;
use crate::core::compute::Vector;

/// Async accelerator interface
#[async_trait]
pub trait AsyncAccelerator: Send + Sync {
    /// Initialize the accelerator
    async fn initialize(&self) -> Result<()>;
    
    /// Create a new vector
    async fn create_vector(&self, size: usize) -> Result<Vector>;
    
    /// Execute operation on vector
    async fn execute(&self, vector: &mut Vector, op: Operation) -> Result<()>;
    
    /// Wait for operation to complete
    async fn wait_completion(&self, vector: &Vector) -> Result<Status>;
}

/// Accelerator implementation
pub struct Accelerator {
    unit_manager: Arc<UnitManager>,
    initialized: Arc<Mutex<bool>>,
}

impl Accelerator {
    /// Create new accelerator instance
    pub fn new(unit_manager: UnitManager) -> Self {
        Self {
            unit_manager: Arc::new(unit_manager),
            initialized: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl AsyncAccelerator for Accelerator {
    async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Ok(());
        }

        self.unit_manager.initialize().await?;
        *initialized = true;
        Ok(())
    }

    async fn create_vector(&self, size: usize) -> Result<Vector> {
        if !*self.initialized.lock().await {
            return Err(AccelError::Config(
                "Accelerator not initialized".to_string()
            ));
        }
        Vector::new(size)
    }

    async fn execute(&self, vector: &mut Vector, op: Operation) -> Result<()> {
        if !*self.initialized.lock().await {
            return Err(AccelError::Config(
                "Accelerator not initialized".to_string()
            ));
        }

        match op {
            Operation::Copy { from } => {
                self.unit_manager.execute(
                    vector.unit_id().ok_or_else(|| AccelError::Config(
                        "Vector not bound to unit".to_string()
                    ))?,
                    op
                ).await
            },
            Operation::Add { from } => {
                self.unit_manager.execute(
                    vector.unit_id().ok_or_else(|| AccelError::Config(
                        "Vector not bound to unit".to_string()
                    ))?,
                    op
                ).await
            },
            Operation::Activate { function } => {
                self.unit_manager.execute(
                    vector.unit_id().ok_or_else(|| AccelError::Config(
                        "Vector not bound to unit".to_string()
                    ))?,
                    op
                ).await
            },
            _ => Err(AccelError::Config(
                format!("Unsupported operation: {:?}", op)
            )),
        }
    }

    async fn wait_completion(&self, vector: &Vector) -> Result<Status> {
        let unit_id = vector.unit_id().ok_or_else(|| AccelError::Config(
            "Vector not bound to unit".to_string()
        ))?;
        
        let state = self.unit_manager.get_state(unit_id).await?;
        Ok(state.status)
    }
}

/// Helper functions for common operations
impl Accelerator {
    /// Copy data between vectors
    pub async fn copy(&self, src: &Vector, dst: &mut Vector) -> Result<()> {
        let src_id = src.unit_id().ok_or_else(|| AccelError::Config(
            "Source vector not bound to unit".to_string()
        ))?;
        
        self.execute(dst, Operation::Copy { from: src_id }).await
    }

    /// Add vectors
    pub async fn add(&self, src: &Vector, dst: &mut Vector) -> Result<()> {
        let src_id = src.unit_id().ok_or_else(|| AccelError::Config(
            "Source vector not bound to unit".to_string()
        ))?;
        
        self.execute(dst, Operation::Add { from: src_id }).await
    }

    /// Apply activation function
    pub async fn activate(&self, vector: &mut Vector, function: Activation) -> Result<()> {
        self.execute(vector, Operation::Activate { function }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::fpga::MockFpga;

    #[tokio::test]
    async fn test_accelerator() {
        let unit_manager = UnitManager::new(Box::new(MockFpga::default()));
        let accelerator = Accelerator::new(unit_manager);

        // Test initialization
        assert!(accelerator.initialize().await.is_ok());

        // Test vector creation
        let mut vec1 = accelerator.create_vector(32).await.unwrap();
        let mut vec2 = accelerator.create_vector(32).await.unwrap();

        // Test vector operations
        vec1.bind_to_unit(UnitId::new(0).unwrap()).await.unwrap();
        vec2.bind_to_unit(UnitId::new(1).unwrap()).await.unwrap();

        assert!(accelerator.copy(&vec1, &mut vec2).await.is_ok());
        assert!(accelerator.add(&vec1, &mut vec2).await.is_ok());
        assert!(accelerator.activate(&mut vec2, Activation::ReLU).await.is_ok());

        // Test completion waiting
        assert!(matches!(
            accelerator.wait_completion(&vec2).await.unwrap(),
            Status::Success
        ));
    }
}