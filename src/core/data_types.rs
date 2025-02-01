use num_traits::Zero;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressedNum {
    /// 三値化表現: -1, 0, 1のみ
    Trinary(i8),
}

impl CompressedNum {
    /// 整数値からの変換（-1, 0, 1のみ許可）
    pub fn from_integer(value: i32) -> Result<Self, crate::error::AcceleratorError> {
        match value {
            -1 => Ok(CompressedNum::Trinary(-1)),
            0 => Ok(CompressedNum::Trinary(0)),
            1 => Ok(CompressedNum::Trinary(1)),
            _ => Err(crate::error::AcceleratorError::DataConversionError(
                format!("Invalid value: {}. Must be -1, 0, or 1", value)
            ))
        }
    }

    /// 内部値の取得
    pub fn value(&self) -> i32 {
        match self {
            CompressedNum::Trinary(v) => *v as i32
        }
    }
}

#[derive(Debug, Clone)]
pub struct FpgaVector {
    pub data: Vec<CompressedNum>,
    pub dimension: usize,
}

#[derive(Debug, Clone)]
pub struct FpgaMatrix {
    pub data: Vec<Vec<CompressedNum>>,
    pub rows: usize,
    pub cols: usize,
}

/// 計算タイプ列挙型
#[derive(Debug, Clone, Copy)]
pub enum ComputationType {
    Add,
    Multiply,
    Tanh,
    ReLU,
    MatrixVectorMultiply,
}

impl FpgaVector {
    /// 整数ベクトルから変換
    pub fn from_numpy(numpy_vec: &[i32]) -> Result<Self, crate::error::AcceleratorError> {
        if numpy_vec.len() % 16 != 0 {
            return Err(crate::error::AcceleratorError::InvalidDimension(numpy_vec.len()));
        }

        let converted_data = numpy_vec.iter()
            .map(|&x| CompressedNum::from_integer(x))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            data: converted_data,
            dimension: numpy_vec.len(),
        })
    }

    /// 整数ベクトルに戻す
    pub fn to_numpy(&self) -> Vec<i32> {
        self.data.iter().map(|x| x.value()).collect()
    }
}

impl FpgaMatrix {
    /// 整数行列から変換
    pub fn from_numpy(
        numpy_matrix: &[Vec<i32>>
    ) -> Result<Self, crate::error::AcceleratorError> {
        if numpy_matrix.is_empty() || 
           numpy_matrix.len() % 16 != 0 || 
           numpy_matrix[0].len() % 16 != 0 {
            return Err(crate::error::AcceleratorError::InvalidDimension(numpy_matrix.len()));
        }

        let converted_data = numpy_matrix.iter()
            .map(|row| {
                row.iter()
                    .map(|&x| CompressedNum::from_integer(x))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            data: converted_data,
            rows: numpy_matrix.len(),
            cols: numpy_matrix[0].len(),
        })
    }

    /// 整数行列に戻す
    pub fn to_numpy(&self) -> Vec<Vec<i32>> {
        self.data.iter()
            .map(|row| row.iter().map(|x| x.value()).collect())
            .collect()
    }

    /// 大規模行列を16x16ブロックに分割
    pub fn split_into_blocks(&self, block_size: usize) -> Vec<Vec<FpgaMatrix>> {
        let mut blocks = Vec::new();

        for row_start in (0..self.rows).step_by(block_size) {
            let mut row_blocks = Vec::new();
            
            for col_start in (0..self.cols).step_by(block_size) {
                let mut block_data = Vec::new();
                
                for r in row_start..std::cmp::min(row_start + block_size, self.rows) {
                    let mut block_row = Vec::new();
                    
                    for c in col_start..std::cmp::min(col_start + block_size, self.cols) {
                        block_row.push(self.data[r][c]);
                    }
                    
                    // パディング
                    while block_row.len() < block_size {
                        block_row.push(CompressedNum::Trinary(0));
                    }
                    
                    block_data.push(block_row);
                }
                
                // 行のパディング
                while block_data.len() < block_size {
                    block_data.push(vec![CompressedNum::Trinary(0); block_size]);
                }
                
                row_blocks.push(FpgaMatrix {
                    data: block_data,
                    rows: block_size,
                    cols: block_size,
                });
            }
            
            blocks.push(row_blocks);
        }
        
        blocks
    }
}