//! Core types for the neural network accelerator
//! 
//! This module defines the fundamental types and constants used throughout
//! the accelerator implementation.

use serde::{Serialize, Deserialize};
use std::fmt;

/// Size of computation blocks
pub const BLOCK_SIZE: usize = 16;
/// Number of processing units
pub const UNIT_COUNT: usize = 256;
/// Vector width in bits
pub const VECTOR_WIDTH: usize = 32;

/// Identifies a specific processing unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(pub usize);

impl UnitId {
    /// Create a new UnitId, returning None if out of range
    pub fn new(id: usize) -> Option<Self> {
        if id < UNIT_COUNT {
            Some(Self(id))
        } else {
            None
        }
    }

    /// Get the raw unit ID value
    pub fn raw(&self) -> usize {
        self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unit#{}", self.0)
    }
}

/// Vector computation operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// No operation
    Nop,
    /// Load data from memory
    Load { address: usize },
    /// Store data to memory
    Store { address: usize },
    /// Copy data from another unit
    Copy { from: UnitId },
    /// Add data from another unit
    Add { from: UnitId },
    /// Apply activation function
    Activate { function: Activation },
}

/// Activation functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activation {
    /// Hyperbolic tangent
    Tanh,
    /// Rectified Linear Unit
    ReLU,
}

/// A block of vector data
#[derive(Debug, Clone, PartialEq)]
pub struct VectorBlock {
    data: [f32; BLOCK_SIZE],
}

impl VectorBlock {
    /// Create a new zero-initialized block
    pub fn new() -> Self {
        Self {
            data: [0.0; BLOCK_SIZE]
        }
    }

    /// Get data at index
    pub fn get(&self, index: usize) -> Option<f32> {
        self.data.get(index).copied()
    }

    /// Set data at index
    pub fn set(&mut self, index: usize, value: f32) -> bool {
        if let Some(elem) = self.data.get_mut(index) {
            *elem = value;
            true
        } else {
            false
        }
    }

    /// Get raw data slice
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }
}

impl Default for VectorBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Computation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Operation completed successfully
    Success,
    /// Operation is in progress
    InProgress,
    /// Operation failed
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_id_validation() {
        assert!(UnitId::new(0).is_some());
        assert!(UnitId::new(UNIT_COUNT - 1).is_some());
        assert!(UnitId::new(UNIT_COUNT).is_none());
    }

    #[test]
    fn test_vector_block() {
        let mut block = VectorBlock::new();
        
        // Test initial state
        assert_eq!(block.get(0), Some(0.0));
        
        // Test valid operations
        assert!(block.set(0, 1.0));
        assert_eq!(block.get(0), Some(1.0));
        
        // Test bounds
        assert!(!block.set(BLOCK_SIZE, 0.0));
        assert_eq!(block.get(BLOCK_SIZE), None);
    }
}