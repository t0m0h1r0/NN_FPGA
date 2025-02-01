// block.rs

use crate::types::{BLOCK_SIZE, Activation, MatrixIndex};
use crate::error::{Result, NNError};

/// 16次元ベクトルブロック
#[derive(Clone, Debug)]
pub struct VectorBlock {
    data: [f32; BLOCK_SIZE],
}

/// 16x16行列ブロック
#[derive(Clone, Debug)]
pub struct MatrixBlock {
    data: [[f32; BLOCK_SIZE]; BLOCK_SIZE],
}

impl VectorBlock {
    /// 新しいベクトルブロックを作成
    pub fn new() -> Self {
        Self {
            data: [0.0; BLOCK_SIZE],
        }
    }

    /// スライスからベクトルブロックを作成
    pub fn from_slice(slice: &[f32]) -> Result<Self> {
        if slice.len() != BLOCK_SIZE {
            return Err(NNError::Dimension(
                format!("Expected length {}, got {}", BLOCK_SIZE, slice.len())
            ));
        }
        let mut data = [0.0; BLOCK_SIZE];
        data.copy_from_slice(slice);
        Ok(Self { data })
    }

    /// インデックスで要素を取得
    pub fn get(&self, index: usize) -> Result<f32> {
        self.data.get(index).copied().ok_or_else(|| 
            NNError::Dimension(format!("Index {} out of bounds", index)))
    }

    /// インデックスで要素を設定
    pub fn set(&mut self, index: usize, value: f32) -> Result<()> {
        if index >= BLOCK_SIZE {
            return Err(NNError::Dimension(
                format!("Index {} out of bounds", index)
            ));
        }
        self.data[index] = value;
        Ok(())
    }

    /// アクティベーション関数を適用
    pub fn apply_activation(&self, activation: Activation) -> Self {
        let mut result = self.clone();
        for val in result.data.iter_mut() {
            *val = match activation {
                Activation::Tanh => val.tanh(),
                Activation::ReLU => val.max(0.0),
            };
        }
        result
    }

    /// データをスライスとして取得
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }
}

impl MatrixBlock {
    /// 新しい行列ブロックを作成
    pub fn new() -> Self {
        Self {
            data: [[0.0; BLOCK_SIZE]; BLOCK_SIZE],
        }
    }

    /// 要素を取得
    pub fn get(&self, index: MatrixIndex) -> Result<f32> {
        if index.row >= BLOCK_SIZE || index.col >= BLOCK_SIZE {
            return Err(NNError::Dimension(
                format!("Indices ({}, {}) out of bounds", index.row, index.col)
            ));
        }
        Ok(self.data[index.row][index.col])
    }

    /// 要素を設定
    pub fn set(&mut self, index: MatrixIndex, value: f32) -> Result<()> {
        if index.row >= BLOCK_SIZE || index.col >= BLOCK_SIZE {
            return Err(NNError::Dimension(
                format!("Indices ({}, {}) out of bounds", index.row, index.col)
            ));
        }
        self.data[index.row][index.col] = value;
        Ok(())
    }

    /// ベクトルとの乗算
    pub fn multiply(&self, vector: &VectorBlock) -> VectorBlock {
        let mut result = VectorBlock::new();
        for row in 0..BLOCK_SIZE {
            let mut sum = 0.0;
            for col in 0..BLOCK_SIZE {
                sum += self.data[row][col] * vector.data[col];
            }
            result.data[row] = sum;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_block_operations() {
        let mut vec = VectorBlock::new();
        
        // 値の設定と取得
        vec.set(0, 1.0).unwrap();
        assert_eq!(vec.get(0).unwrap(), 1.0);
        
        // 範囲外アクセス
        assert!(vec.set(BLOCK_SIZE, 0.0).is_err());
        assert!(vec.get(BLOCK_SIZE).is_err());
    }

    #[test]
    fn test_matrix_block_operations() {
        let mut matrix = MatrixBlock::new();
        let idx = MatrixIndex::new(0, 0);
        
        // 値の設定と取得
        matrix.set(idx, 1.0).unwrap();
        assert_eq!(matrix.get(idx).unwrap(), 1.0);
        
        // 範囲外アクセス
        let invalid_idx = MatrixIndex::new(BLOCK_SIZE, 0);
        assert!(matrix.set(invalid_idx, 0.0).is_err());
        assert!(matrix.get(invalid_idx).is_err());
    }

    #[test]
    fn test_matrix_vector_multiplication() {
        let mut matrix = MatrixBlock::new();
        let mut vector = VectorBlock::new();
        
        // 単位行列の設定
        for i in 0..BLOCK_SIZE {
            matrix.set(MatrixIndex::new(i, i), 1.0).unwrap();
            vector.set(i, 2.0).unwrap();
        }
        
        let result = matrix.multiply(&vector);
        
        // 単位行列との乗算は元のベクトルを返すはず
        for i in 0..BLOCK_SIZE {
            assert_eq!(result.get(i).unwrap(), 2.0);
        }
    }
}