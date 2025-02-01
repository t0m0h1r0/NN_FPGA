// vector.rs

use rayon::prelude::*;
use std::sync::Arc;
use crossbeam::channel::{bounded, Sender};
use crate::types::{BLOCK_SIZE, Activation};
use crate::block::VectorBlock;
use crate::error::{Result, NNError};

/// 並列処理用の設定
const MIN_PARALLEL_SIZE: usize = 4;

/// 大きなベクトル（16の倍数サイズ）
#[derive(Clone)]
pub struct Vector {
    size: usize,
    blocks: Vec<Arc<VectorBlock>>,
}

impl Vector {
    /// 新しいベクトルを作成
    pub fn new(size: usize) -> Result<Self> {
        Self::validate_size(size)?;
        let blocks = Self::create_empty_blocks(size / BLOCK_SIZE);
        Ok(Self { size, blocks })
    }

    /// ベクトルのサイズを検証
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

    /// 空のブロックを作成
    fn create_empty_blocks(num_blocks: usize) -> Vec<Arc<VectorBlock>> {
        (0..num_blocks)
            .map(|_| Arc::new(VectorBlock::new()))
            .collect()
    }

    /// ベクトルの要素を取得
    pub fn get(&self, index: usize) -> Result<f32> {
        if !self.is_valid_index(index) {
            return Err(NNError::Dimension(
                format!("Index {} out of bounds for vector of size {}", index, self.size)
            ));
        }

        let (block_idx, inner_idx) = self.to_block_indices(index);
        self.blocks[block_idx].get(inner_idx)
    }

    /// インデックスの妥当性を確認
    fn is_valid_index(&self, index: usize) -> bool {
        index < self.size
    }

    /// ブロックインデックスに変換
    fn to_block_indices(&self, index: usize) -> (usize, usize) {
        (index / BLOCK_SIZE, index % BLOCK_SIZE)
    }

    /// ベクトルの要素を設定
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

    /// アクティベーション関数を並列適用
    pub fn apply_activation(&self, activation: Activation) -> Result<Self> {
        let (sender, receiver) = bounded(self.blocks.len());
        
        // ブロック単位で並列処理を実行
        self.process_activation_in_parallel(activation, &sender)?;
        
        // 結果を集約
        self.collect_activation_results(receiver)
    }

    /// アクティベーション関数の並列処理
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

    /// アクティベーション結果の集約
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

    /// ベクトルの加算を並列実行
    pub fn add(&self, other: &Vector) -> Result<Self> {
        self.validate_binary_op(other)?;
        let mut result = Self::new(self.size)?;

        self.blocks.par_iter().enumerate().try_for_each(|(i, block)| {
            let sum_block = block.add(&other.blocks[i]);
            result.blocks[i] = Arc::new(sum_block);
            Ok(())
        })?;

        Ok(result)
    }

    /// 二項演算の妥当性を検証
    fn validate_binary_op(&self, other: &Vector) -> Result<()> {
        if self.size != other.size {
            return Err(NNError::Dimension(
                format!("Vector sizes must match: {} != {}", self.size, other.size)
            ));
        }
        Ok(())
    }

    /// スカラー倍を並列実行
    pub fn scale(&self, scalar: f32) -> Result<Self> {
        let mut result = Self::new(self.size)?;

        self.blocks.par_iter().enumerate().for_each(|(i, block)| {
            let scaled_block = block.scale(scalar);
            result.blocks[i] = Arc::new(scaled_block);
        });

        Ok(result)
    }

    /// 内積を並列計算
    pub fn dot(&self, other: &Vector) -> Result<f32> {
        self.validate_binary_op(other)?;

        Ok(self.blocks.par_iter()
            .zip(other.blocks.par_iter())
            .map(|(block1, block2)| block1.dot(block2))
            .sum())
    }

    /// ベクトルのサイズを取得
    pub fn size(&self) -> usize {
        self.size
    }

    /// ブロックを取得（内部使用）
    pub(crate) fn get_block(&self, index: usize) -> Result<VectorBlock> {
        self.blocks.get(index)
            .map(|b| (*b).clone())
            .ok_or_else(|| NNError::Dimension(
                format!("Block index {} out of bounds", index)
            ))
    }

    /// ブロックを設定（内部使用）
    pub(crate) fn set_block(&mut self, index: usize, block: VectorBlock) -> Result<()> {
        if index >= self.blocks.len() {
            return Err(NNError::Dimension(
                format!("Block index {} out of bounds", index)
            ));
        }
        self.blocks[index] = Arc::new(block);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_creation() {
        assert!(Vector::new(16).is_ok());
        assert!(Vector::new(32).is_ok());
        assert!(Vector::new(0).is_err());
        assert!(Vector::new(15).is_err());
    }

    #[test]
    fn test_vector_operations() {
        let mut vec1 = Vector::new(32).unwrap();
        let mut vec2 = Vector::new(32).unwrap();

        // 値の設定
        for i in 0..32 {
            vec1.set(i, 1.0).unwrap();
            vec2.set(i, 2.0).unwrap();
        }

        // 加算のテスト
        let sum = vec1.add(&vec2).unwrap();
        for i in 0..32 {
            assert_eq!(sum.get(i).unwrap(), 3.0);
        }

        // スケーリングのテスト
        let scaled = vec1.scale(2.0).unwrap();
        for i in 0..32 {
            assert_eq!(scaled.get(i).unwrap(), 2.0);
        }

        // 内積のテスト
        assert_eq!(vec1.dot(&vec2).unwrap(), 64.0); // 32 * (1.0 * 2.0)
    }

    #[test]
    fn test_activation_functions() {
        let mut vector = Vector::new(16).unwrap();
        
        // 負の値を設定
        for i in 0..16 {
            vector.set(i, -1.0).unwrap();
        }

        // ReLUのテスト
        let relu_result = vector.apply_activation(Activation::ReLU).unwrap();
        for i in 0..16 {
            assert_eq!(relu_result.get(i).unwrap(), 0.0);
        }

        // tanhのテスト
        let tanh_result = vector.apply_activation(Activation::Tanh).unwrap();
        for i in 0..16 {
            assert!(tanh_result.get(i).unwrap() < 0.0);
        }
    }
}