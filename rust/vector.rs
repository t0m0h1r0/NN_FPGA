// vector.rs

use rayon::prelude::*;
use std::sync::Arc;
use crossbeam::channel::{bounded, Sender};
use crate::types::{BLOCK_SIZE, UNIT_COUNT, Activation, Operation};
use crate::block::VectorBlock;
use crate::error::{Result, NNError};

/// 並列処理用の設定
const MIN_PARALLEL_SIZE: usize = 4;

#[derive(Clone)]
pub struct Vector {
    pub(crate) size: usize,
    pub(crate) blocks: Vec<Arc<VectorBlock>>,
    pub(crate) unit_id: Option<usize>,  // 追加：ユニットID
}

impl Vector {
    pub fn new(size: usize) -> Result<Self> {
        Self::validate_size(size)?;
        let blocks = Self::create_empty_blocks(size / BLOCK_SIZE);
        Ok(Self { 
            size, 
            blocks,
            unit_id: None 
        })
    }

    fn validate_size(size: usize) -> Result<()> {
        if size == 0 {
            return Err(NNError::Dimension("Vector size must be positive".to_string()));
        }
        if size % BLOCK_SIZE != 0 {
            return Err(NNError::Dimension(
                format!("Vector size must be multiple of {}", BLOCK_SIZE)
            ));
        }
        Ok(())
    }

    fn create_empty_blocks(num_blocks: usize) -> Vec<Arc<VectorBlock>> {
        (0..num_blocks)
            .map(|_| Arc::new(VectorBlock::new()))
            .collect()
    }

    pub fn get(&self, index: usize) -> Result<f32> {
        if !self.is_valid_index(index) {
            return Err(NNError::Dimension(
                format!("Index {} out of bounds for vector of size {}", index, self.size)
            ));
        }

        let (block_idx, inner_idx) = self.to_block_indices(index);
        self.blocks[block_idx].get(inner_idx)
    }

    fn is_valid_index(&self, index: usize) -> bool {
        index < self.size
    }

    fn to_block_indices(&self, index: usize) -> (usize, usize) {
        (index / BLOCK_SIZE, index % BLOCK_SIZE)
    }

    pub fn set(&mut self, index: usize, value: f32) -> Result<()> {
        if !self.is_valid_index(index) {
            return Err(NNError::Dimension(
                format!("Index {} out of bounds for vector of size {}", index, self.size)
            ));
        }

        let (block_idx, inner_idx) = self.to_block_indices(index);
        let mut new_block = (*self.blocks[block_idx]).clone();
        new_block.set(inner_idx, value)?;
        self.blocks[block_idx] = Arc::new(new_block);
        Ok(())
    }

    /// 新規: 特定ユニットへのバインド
    pub fn bind_to_unit(&mut self, unit_id: usize) -> Result<()> {
        if unit_id >= UNIT_COUNT {
            return Err(NNError::Dimension(
                format!("Unit ID {} is out of range", unit_id)
            ));
        }
        self.unit_id = Some(unit_id);
        Ok(())
    }

    /// 新規: 他ユニットからのベクトルコピー
    pub fn copy_from_unit(&mut self, source_unit: usize) -> Result<()> {
        let target_unit = self.unit_id.ok_or_else(|| 
            NNError::Operation("Vector is not bound to any unit".to_string()))?;

        if source_unit >= UNIT_COUNT {
            return Err(NNError::Dimension(
                format!("Source unit {} is out of range", source_unit)
            ));
        }

        // FPGAへの命令生成（実際の実装ではここでFPGAと通信）
        println!("Copying vector from unit {} to unit {}", source_unit, target_unit);
        Ok(())
    }

    /// 新規: 他ユニットのベクトルとの加算
    pub fn add_from_unit(&mut self, source_unit: usize) -> Result<()> {
        let target_unit = self.unit_id.ok_or_else(|| 
            NNError::Operation("Vector is not bound to any unit".to_string()))?;

        if source_unit >= UNIT_COUNT {
            return Err(NNError::Dimension(
                format!("Source unit {} is out of range", source_unit)
            ));
        }

        // FPGAへの命令生成（実際の実装ではここでFPGAと通信）
        println!("Adding vector from unit {} to unit {}", source_unit, target_unit);
        Ok(())
    }

    pub fn apply_activation(&self, activation: Activation) -> Result<Self> {
        let (sender, receiver) = bounded(self.blocks.len());
        self.process_activation_in_parallel(activation, &sender)?;
        self.collect_activation_results(receiver)
    }

    fn process_activation_in_parallel(
        &self,
        activation: Activation,
        sender: &Sender<(usize, VectorBlock)>
    ) -> Result<()> {
        self.blocks.par_iter().enumerate().try_for_each(|(i, block)| {
            let result = block.apply_activation(activation);
            sender.send((i, result)).map_err(|_| 
                NNError::Computation("Failed to send activation result".to_string())
            )
        })
    }

    fn collect_activation_results(
        &self,
        receiver: crossbeam::channel::Receiver<(usize, VectorBlock)>
    ) -> Result<Self> {
        let mut result = Self::new(self.size)?;
        let mut block_results: Vec<_> = receiver.iter().take(self.blocks.len()).collect();
        block_results.par_sort_by_key(|(idx, _)| *idx);

        for (i, block) in block_results {
            result.blocks[i] = Arc::new(block);
        }

        Ok(result)
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_operations() {
        let mut vec1 = Vector::new(32).unwrap();
        let mut vec2 = Vector::new(32).unwrap();

        // ユニットへのバインドテスト
        assert!(vec1.bind_to_unit(0).is_ok());
        assert!(vec2.bind_to_unit(1).is_ok());
        
        // 無効なユニットIDのテスト
        assert!(vec1.bind_to_unit(UNIT_COUNT).is_err());

        // コピー操作のテスト
        assert!(vec2.copy_from_unit(0).is_ok());
        
        // 加算操作のテスト
        assert!(vec2.add_from_unit(0).is_ok());
    }
}