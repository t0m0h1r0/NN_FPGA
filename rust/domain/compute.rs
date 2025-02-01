//! Core computation functionality for the neural network accelerator
//!
//! This module implements the fundamental computation operations and
//! abstractions for vector processing.

use std::sync::Arc;
use tokio::sync::RwLock;
use rayon::prelude::*;

use crate::types::{
    UnitId, Operation, Activation, VectorBlock,
    BLOCK_SIZE, UNIT_COUNT
};
use crate::error::{Result, AccelError, UnitError};

/// Represents a vector that can be processed by the accelerator
#[derive(Clone)]
pub struct Vector {
    /// Number of elements
    size: usize,
    /// Data blocks
    blocks: Vec<Arc<RwLock<VectorBlock>>>,
    /// Associated processing unit
    unit: Option<UnitId>,
}

impl Vector {
    /// Create a new vector with the specified size
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 || size % BLOCK_SIZE != 0 {
            return Err(AccelError::Dimension(
                format!("Vector size must be positive multiple of {}", BLOCK_SIZE)
            ));
        }

        let num_blocks = size / BLOCK_SIZE;
        let blocks = (0..num_blocks)
            .map(|_| Arc::new(RwLock::new(VectorBlock::new())))
            .collect();

        Ok(Self {
            size,
            blocks,
            unit: None,
        })
    }

    /// Bind vector to a specific processing unit
    pub async fn bind_to_unit(&mut self, unit_id: UnitId) -> Result<()> {
        if unit_id.raw() >= UNIT_COUNT {
            return Err(UnitError::InvalidId(unit_id.raw()).into());
        }
        self.unit = Some(unit_id);
        Ok(())
    }

    /// Get value at specific index
    pub async fn get(&self, index: usize) -> Result<f32> {
        let (block_idx, inner_idx) = self.validate_index(index)?;
        let block = self.blocks[block_idx].read().await;
        block.get(inner_idx)
            .ok_or_else(|| AccelError::Dimension(
                format!("Index {} out of bounds", index)
            ))
    }

    /// Set value at specific index
    pub async fn set(&mut self, index: usize, value: f32) -> Result<()> {
        let (block_idx, inner_idx) = self.validate_index(index)?;
        let mut block = self.blocks[block_idx].write().await;
        if !block.set(inner_idx, value) {
            return Err(AccelError::Dimension(
                format!("Failed to set value at index {}", index)
            ));
        }
        Ok(())
    }

    /// Copy data from another unit
    pub async fn copy_from_unit(&mut self, source: UnitId) -> Result<()> {
        let target = self.require_unit()?;
        self.execute_operation(Operation::Copy { from: source }).await
    }

    /// Add data from another unit
    pub async fn add_from_unit(&mut self, source: UnitId) -> Result<()> {
        let target = self.require_unit()?;
        self.execute_operation(Operation::Add { from: source }).await
    }

    /// Apply activation function
    pub async fn apply_activation(&mut self, function: Activation) -> Result<()> {
        self.require_unit()?;
        self.execute_operation(Operation::Activate { function }).await
    }

    // Private helper methods

    /// Validate index and return block and inner indices
    fn validate_index(&self, index: usize) -> Result<(usize, usize)> {
        if index >= self.size {
            return Err(AccelError::Dimension(
                format!("Index {} out of bounds for vector size {}", index, self.size)
            ));
        }
        Ok((index / BLOCK_SIZE, index % BLOCK_SIZE))
    }

    /// Ensure vector is bound to a unit and return the unit ID
    fn require_unit(&self) -> Result<UnitId> {
        self.unit.ok_or_else(|| AccelError::Config(
            "Vector is not bound to any unit".to_string()
        ))
    }

    /// Execute operation on the hardware
    async fn execute_operation(&mut self, op: Operation) -> Result<()> {
        // In a real implementation, this would communicate with the FPGA
        // For now, we'll just simulate the operation
        match op {
            Operation::Copy { from } => {
                println!("Copying from unit {} to unit {}", 
                        from.raw(), self.require_unit()?.raw());
            },
            Operation::Add { from } => {
                println!("Adding data from unit {} to unit {}", 
                        from.raw(), self.require_unit()?.raw());
            },
            Operation::Activate { function } => {
                println!("Applying {:?} activation on unit {}", 
                        function, self.require_unit()?.raw());
            },
            _ => return Err(UnitError::UnsupportedOperation(
                format!("Operation {:?} not implemented", op)
            ).into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_vector_creation() {
        let rt = Runtime::new().unwrap();
        
        assert!(rt.block_on(async {
            Vector::new(16).await.is_ok()
        }));
        assert!(rt.block_on(async {
            Vector::new(0).await.is_err()
        }));
        assert!(rt.block_on(async {
            Vector::new(15).await.is_err()
        }));
    }

    #[test]
    fn test_vector_operations() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut vec = Vector::new(32).await.unwrap();
            
            // Test unit binding
            let unit_id = UnitId::new(0).unwrap();
            assert!(vec.bind_to_unit(unit_id).await.is_ok());
            
            // Test value setting and getting
            assert!(vec.set(0, 1.0).await.is_ok());
            assert_eq!(vec.get(0).await.unwrap(), 1.0);
            
            // Test operations
            let source = UnitId::new(1).unwrap();
            assert!(vec.copy_from_unit(source).await.is_ok());
            assert!(vec.add_from_unit(source).await.is_ok());
            assert!(vec.apply_activation(Activation::ReLU).await.is_ok());
        });
    }
}