//! Memory management implementation
//!
//! This module handles memory allocation and management for the FPGA.

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use thiserror::Error;

use crate::domain::{
    operation::UnitId,
    error::{Result, DomainError},
};

/// Memory block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u64);

impl BlockId {
    /// Create new block ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw block ID
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// Allocate contiguous blocks
    Contiguous,
    /// Allow fragmented allocation
    Fragmented,
}

/// Memory block status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// Block is free
    Free,
    /// Block is allocated
    Allocated {
        /// Owner unit
        unit: UnitId,
    },
    /// Block is locked
    Locked {
        /// Owner unit
        unit: UnitId,
        /// Lock reason
        reason: LockReason,
    },
}

/// Lock reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockReason {
    /// Block is being written
    Writing,
    /// Block is being read
    Reading,
    /// Block is being transferred
    Transferring,
}

/// Memory-specific errors
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Insufficient memory: requested {requested} bytes, available {available} bytes")]
    InsufficientMemory {
        requested: usize,
        available: usize,
    },

    #[error("Block {0} not found")]
    BlockNotFound(BlockId),

    #[error("Block {0} already allocated")]
    BlockAlreadyAllocated(BlockId),

    #[error("Block {0} is locked")]
    BlockLocked(BlockId),

    #[error("Invalid block size: {0} bytes")]
    InvalidBlockSize(usize),
}

/// Memory manager for FPGA memory
pub struct MemoryManager {
    /// Total memory size
    total_size: usize,
    /// Block size
    block_size: usize,
    /// Block status map
    blocks: Arc<RwLock<HashMap<BlockId, BlockStatus>>>,
    /// Allocation counter
    next_block_id: Arc<Mutex<u64>>,
}

impl MemoryManager {
    /// Create new memory manager
    pub fn new(total_size: usize, block_size: usize) -> Result<Self> {
        if block_size == 0 || block_size % 16 != 0 {
            return Err(MemoryError::InvalidBlockSize(block_size).into());
        }

        Ok(Self {
            total_size,
            block_size,
            blocks: Arc::new(RwLock::new(HashMap::new())),
            next_block_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Allocate memory block
    pub async fn allocate(
        &self,
        size: usize,
        unit: UnitId,
        strategy: AllocationStrategy,
    ) -> Result<BlockId> {
        if size == 0 || size % self.block_size != 0 {
            return Err(MemoryError::InvalidBlockSize(size).into());
        }

        let num_blocks = size / self.block_size;
        let mut blocks = self.blocks.write().await;

        // Check available memory
        let used_blocks = blocks.len();
        let max_blocks = self.total_size / self.block_size;
        if used_blocks + num_blocks > max_blocks {
            return Err(MemoryError::InsufficientMemory {
                requested: size,
                available: (max_blocks - used_blocks) * self.block_size,
            }.into());
        }

        // Allocate new block
        let mut next_id = self.next_block_id.lock().await;
        let block_id = BlockId::new(*next_id);
        *next_id += 1;

        blocks.insert(block_id, BlockStatus::Allocated { unit });

        Ok(block_id)
    }

    /// Free memory block
    pub async fn free(&self, block_id: BlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.get(&block_id) {
            None => Err(MemoryError::BlockNotFound(block_id).into()),
            Some(BlockStatus::Locked { .. }) => {
                Err(MemoryError::BlockLocked(block_id).into())
            },
            Some(_) => {
                blocks.remove(&block_id);
                Ok(())
            }
        }
    }

    /// Lock memory block
    pub async fn lock(
        &self,
        block_id: BlockId,
        unit: UnitId,
        reason: LockReason,
    ) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.get(&block_id) {
            None => Err(MemoryError::BlockNotFound(block_id).into()),
            Some(BlockStatus::Locked { .. }) => {
                Err(MemoryError::BlockLocked(block_id).into())
            },
            Some(_) => {
                blocks.insert(block_id, BlockStatus::Locked { unit, reason });
                Ok(())
            }
        }
    }

    /// Unlock memory block
    pub async fn unlock(&self, block_id: BlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.get(&block_id) {
            None => Err(MemoryError::BlockNotFound(block_id).into()),
            Some(BlockStatus::Locked { unit, .. }) => {
                blocks.insert(block_id, BlockStatus::Allocated { unit: *unit });
                Ok(())
            },
            Some(_) => Ok(()),
        }
    }

    /// Get block status
    pub async fn status(&self, block_id: BlockId) -> Result<BlockStatus> {
        let blocks = self.blocks.read().await;
        blocks.get(&block_id)
            .copied()
            .ok_or_else(|| MemoryError::BlockNotFound(block_id).into())
    }

    /// Get memory usage statistics
    pub async fn usage(&self) -> MemoryUsage {
        let blocks = self.blocks.read().await;
        let total_blocks = self.total_size / self.block_size;
        let used_blocks = blocks.len();
        
        MemoryUsage {
            total_size: self.total_size,
            used_size: used_blocks * self.block_size,
            block_size: self.block_size,
            total_blocks,
            used_blocks,
            locked_blocks: blocks.values()
                .filter(|status| matches!(status, BlockStatus::Locked { .. }))
                .count(),
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryUsage {
    /// Total memory size in bytes
    pub total_size: usize,
    /// Used memory size in bytes
    pub used_size: usize,
    /// Block size in bytes
    pub block_size: usize,
    /// Total number of blocks
    pub total_blocks: usize,
    /// Number of used blocks
    pub used_blocks: usize,
    /// Number of locked blocks
    pub locked_blocks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_allocation() {
        let manager = MemoryManager::new(1024, 16).unwrap();
        let unit = UnitId::new(0).unwrap();

        // Test successful allocation
        let block_id = manager.allocate(32, unit, AllocationStrategy::Contiguous)
            .await
            .unwrap();

        // Verify block status
        let status = manager.status(block_id).await.unwrap();
        assert!(matches!(status, BlockStatus::Allocated { unit: u } if u == unit));

        // Test block locking
        manager.lock(block_id, unit, LockReason::Writing).await.unwrap();
        let status = manager.status(block_id).await.unwrap();
        assert!(matches!(
            status,
            BlockStatus::Locked { unit: u, reason: LockReason::Writing } if u == unit
        ));

        // Test unlocking and freeing
        manager.unlock(block_id).await.unwrap();
        manager.free(block_id).await.unwrap();

        // Verify block is gone
        assert!(manager.status(block_id).await.is_err());
    }

    #[tokio::test]
    async fn test_memory_limits() {
        let manager = MemoryManager::new(32, 16).unwrap();
        let unit = UnitId::new(0).unwrap();

        // Test allocation exceeding total memory
        let result = manager.allocate(64, unit, AllocationStrategy::Contiguous).await;
        assert!(matches!(
            result.unwrap_err().downcast_ref::<MemoryError>(),
            Some(MemoryError::InsufficientMemory { .. })
        ));

        // Test invalid block size
        let result = manager.allocate(10, unit, AllocationStrategy::Contiguous).await;
        assert!(matches!(
            result.unwrap_err().downcast_ref::<MemoryError>(),
            Some(MemoryError::InvalidBlockSize(10))
        ));
    }

    #[tokio::test]
    async fn test_memory_usage() {
        let manager = MemoryManager::new(1024, 16).unwrap();
        let unit = UnitId::new(0).unwrap();

        let usage = manager.usage().await;
        assert_eq!(usage.total_size, 1024);
        assert_eq!(usage.used_size, 0);
        assert_eq!(usage.block_size, 16);

        // Allocate some memory
        let block_id = manager.allocate(32, unit, AllocationStrategy::Contiguous)
            .await
            .unwrap();
        manager.lock(block_id, unit, LockReason::Reading).await.unwrap();

        let usage = manager.usage().await;
        assert_eq!(usage.used_blocks, 1);
        assert_eq!(usage.locked_blocks, 1);
    }
}