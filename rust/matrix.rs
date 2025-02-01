// matrix.rs

use rayon::prelude::*;
use std::sync::Arc;
use crossbeam::channel::{bounded, Sender};

use crate::types::{BLOCK_SIZE, BlockIndex, MatrixIndex};
use crate::block::MatrixBlock;
use crate::vector::Vector;
use crate::store::{Store, make_block_name};
use crate::error::{Result, NNError};

/// 並列処理用の設定
const MIN_PARALLEL_SIZE: usize = 32;

/// 大きな行列（16の倍数サイズ）
#[derive(Clone)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    blocks: Vec<Vec<Arc<MatrixBlock>>>,
}

impl Matrix {
    /// 新しい行列を作成
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        Self::validate_dimensions(rows, cols)?;

        let num_row_blocks = rows / BLOCK_SIZE;
        let num_col_blocks = cols / BLOCK_SIZE;
        
        let blocks = Self::create_empty_blocks(num_row_blocks, num_col_blocks);

        Ok(Self { rows, cols, blocks })
    }

    /// 行列のサイズを検証
    fn validate_dimensions(rows: usize, cols: usize) -> Result<()> {
        if rows == 0 || cols == 0 {
            return Err(NNError::Dimension("Matrix dimensions must be positive".to_string()));
        }
        if rows % BLOCK_SIZE != 0 || cols % BLOCK_SIZE != 0 {
            return Err(NNError::Dimension(
                format!("Matrix dimensions must be multiple of {}", BLOCK_SIZE)
            ));
        }
        Ok(())
    }

    /// 空のブロックを作成
    fn create_empty_blocks(rows: usize, cols: usize) -> Vec<Vec<Arc<MatrixBlock>>> {
        (0..rows)
            .map(|_| {
                (0..cols)
                    .map(|_| Arc::new(MatrixBlock::new()))
                    .collect()
            })
            .collect()
    }

    /// 単位行列を作成
    pub fn identity(size: usize) -> Result<Self> {
        let mut matrix = Self::new(size, size)?;
        matrix.blocks.iter_mut().enumerate().for_each(|(i, row)| {
            row[i] = Arc::new(MatrixBlock::identity());
        });
        Ok(matrix)
    }

    /// 行列の要素を取得
    pub fn get(&self, index: MatrixIndex) -> Result<f32> {
        if !self.is_valid_index(&index) {
            return Err(NNError::Dimension(
                format!("Indices ({}, {}) out of bounds", index.row, index.col)
            ));
        }

        let (block_idx, inner_idx) = index.to_block_indices();
        self.blocks[block_idx.row][block_idx.col].get(inner_idx)
    }

    /// インデックスの妥当性を確認
    fn is_valid_index(&self, index: &MatrixIndex) -> bool {
        index.row < self.rows && index.col < self.cols
    }

    /// 行列の要素を設定
    pub fn set(&mut self, index: MatrixIndex, value: f32) -> Result<()> {
        if !self.is_valid_index(&index) {
            return Err(NNError::Dimension(
                format!("Indices ({}, {}) out of bounds", index.row, index.col)
            ));
        }

        let (block_idx, inner_idx) = index.to_block_indices();
        let mut new_block = (*self.blocks[block_idx.row][block_idx.col]).clone();
        new_block.set(inner_idx, value)?;
        self.blocks[block_idx.row][block_idx.col] = Arc::new(new_block);

        Ok(())
    }

    /// ベクトルとの行列乗算を並列実行
    pub fn multiply(&self, vector: &Vector) -> Result<Vector> {
        self.validate_multiplication(vector)?;

        let num_blocks = self.rows / BLOCK_SIZE;
        let (sender, receiver) = bounded(num_blocks);
        
        // ブロック単位で並列処理を実行
        self.process_blocks_in_parallel(vector, &sender)?;

        // 結果を集約して返す
        self.collect_multiplication_results(receiver, num_blocks)
    }

    /// 行列ベクトル積の妥当性を検証
    fn validate_multiplication(&self, vector: &Vector) -> Result<()> {
        if self.cols != vector.size() {
            return Err(NNError::Dimension(
                format!("Matrix columns ({}) must match vector size ({})",
                    self.cols, vector.size())
            ));
        }
        Ok(())
    }

    /// ブロックの並列処理を実行
    fn process_blocks_in_parallel(
        &self, 
        vector: &Vector, 
        sender: &Sender<(usize, VectorBlock)>
    ) -> Result<()> {
        self.blocks.par_iter().enumerate().try_for_each(|(i, row_blocks)| {
            let mut result = VectorBlock::new();

            // 行内の各ブロックとベクトルブロックの乗算
            for (j, block) in row_blocks.iter().enumerate() {
                if let Ok(vec_block) = vector.get_block(j) {
                    let partial = block.multiply(vec_block);
                    result.add_assign(&partial)?;
                }
            }

            sender.send((i, result)).map_err(|_| 
                NNError::Computation("Failed to send multiplication result".to_string())
            )?;
            Ok(())
        })
    }

    /// 乗算結果を集約
    fn collect_multiplication_results(
        &self,
        receiver: crossbeam::channel::Receiver<(usize, VectorBlock)>,
        num_blocks: usize
    ) -> Result<Vector> {
        let mut result = Vector::new(self.rows)?;
        let mut block_results: Vec<_> = receiver.iter().take(num_blocks).collect();
        block_results.par_sort_by_key(|(idx, _)| *idx);

        for (i, block) in block_results {
            result.set_block(i, block)?;
        }

        Ok(result)
    }

    /// 行列の加算を並列実行
    pub fn add(&self, other: &Matrix) -> Result<Matrix> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(NNError::Dimension(
                "Matrix dimensions must match for addition".to_string()
            ));
        }

        let mut result = Matrix::new(self.rows, self.cols)?;

        self.blocks.par_iter().enumerate().for_each(|(i, row)| {
            row.par_iter().enumerate().for_each(|(j, block)| {
                let other_block = &other.blocks[i][j];
                let sum_block = block.add(other_block);
                result.blocks[i][j] = Arc::new(sum_block);
            });
        });

        Ok(result)
    }

    /// 行列の転置を並列実行
    pub fn transpose(&self) -> Result<Matrix> {
        let mut result = Matrix::new(self.cols, self.rows)?;

        self.blocks.par_iter().enumerate().for_each(|(i, row)| {
            row.par_iter().enumerate().for_each(|(j, block)| {
                result.blocks[j][i] = Arc::new(block.transpose());
            });
        });

        Ok(result)
    }

    /// 行列の保存
    pub fn store(&self, name: &str, store: &Store) -> Result<()> {
        self.blocks.par_iter().enumerate().try_for_each(|(i, row)| {
            row.par_iter().enumerate().try_for_each(|(j, block)| {
                let block_name = make_block_name(name, BlockIndex::new(i, j));
                store.store(&block_name, (*block).clone())
            })
        })
    }

    /// 行列の読み込み
    pub fn load(name: &str, rows: usize, cols: usize, store: &Store) -> Result<Self> {
        let mut matrix = Self::new(rows, cols)?;

        for i in 0..(rows / BLOCK_SIZE) {
            for j in 0..(cols / BLOCK_SIZE) {
                let block_name = make_block_name(name, BlockIndex::new(i, j));
                matrix.blocks[i][j] = store.get(&block_name)?;
            }
        }

        Ok(matrix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_creation() {
        assert!(Matrix::new(16, 16).is_ok());
        assert!(Matrix::new(32, 16).is_ok());
        assert!(Matrix::new(0, 16).is_err());
        assert!(Matrix::new(15, 16).is_err());
    }

    // 他のテストケースは維持...
}