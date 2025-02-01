use half::f16;
use num_traits::Float;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
pub enum CompressedNum {
    /// 三値化表現: 0（ゼロ）, 1（正）, 2（負）
    Trinary(u8),
    /// 1s.31形式の固定小数点数（1ビット符号、31ビット小数部）
    FixedPoint1s31(i32),
    /// 完全な浮動小数点数
    Full(f32),
}

impl CompressedNum {
    /// 浮動小数点数を三値化
    pub fn trinarize(value: f32) -> Self {
        if value == 0.0 {
            CompressedNum::Trinary(0)
        } else if value > 0.0 {
            CompressedNum::Trinary(1)
        } else {
            CompressedNum::Trinary(2)
        }
    }

    /// 浮動小数点数を1s.31形式の固定小数点数に変換
    pub fn to_fixed_point_1s31(value: f32) -> Self {
        // 1s.31形式: 1ビット符号、31ビット小数部
        const FRACTIONAL_BITS: i32 = 31;
        const SCALE: f64 = (1i64 << FRACTIONAL_BITS) as f64;
        
        // クランプ処理（-1.0 から 1.0 の間に制限）
        let clamped = value.max(-1.0).min(1.0);
        
        // 固定小数点数への変換
        let fixed = (clamped as f64 * SCALE).round() as i32;
        
        CompressedNum::FixedPoint1s31(fixed)
    }

    /// 三値化された値を浮動小数点数に戻す
    pub fn from_trinary(trinary: u8) -> f32 {
        match trinary {
            0 => 0.0,
            1 => 1.0,
            2 => -1.0,
            _ => panic!("Invalid trinary value"),
        }
    }

    /// 1s.31形式の固定小数点数を浮動小数点数に戻す
    pub fn from_fixed_point_1s31(fixed: i32) -> f32 {
        const FRACTIONAL_BITS: i32 = 31;
        const SCALE: f64 = (1i64 << FRACTIONAL_BITS) as f64;
        
        (fixed as f64 / SCALE) as f32
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

/// ベクトル変換タイプ
#[derive(Debug, Clone, Copy)]
pub enum VectorConversionType {
    Full,           // 通常の浮動小数点数
    Trinary,        // 三値化
    FixedPoint1s31, // 1s.31固定小数点数
}

/// 行列変換タイプ
#[derive(Debug, Clone, Copy)]
pub enum MatrixConversionType {
    Full,           // 通常の浮動小数点数
    Trinary,        // 三値化
    FixedPoint1s31, // 1s.31固定小数点数
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
    /// NumPyベクトルから変換
    pub fn from_numpy(
        numpy_vec: &[f32], 
        conversion_type: VectorConversionType
    ) -> Result<Self, crate::error::AcceleratorError> {
        if numpy_vec.len() % 16 != 0 {
            return Err(crate::error::AcceleratorError::InvalidDimension(numpy_vec.len()));
        }

        let converted_data = match conversion_type {
            VectorConversionType::Full => 
                numpy_vec.iter().map(|&x| CompressedNum::Full(x)).collect(),
            VectorConversionType::Trinary => 
                numpy_vec.iter().map(|&x| CompressedNum::trinarize(x)).collect(),
            VectorConversionType::FixedPoint1s31 => 
                numpy_vec.iter().map(|&x| CompressedNum::to_fixed_point_1s31(x)).collect(),
        };

        Ok(Self {
            data: converted_data,
            dimension: numpy_vec.len(),
        })
    }

    /// 変換タイプに応じて浮動小数点数ベクトルに戻す
    pub fn to_numpy(&self) -> Vec<f32> {
        self.data.iter().map(|compressed| match compressed {
            CompressedNum::Trinary(val) => CompressedNum::from_trinary(*val),
            CompressedNum::FixedPoint1s31(val) => CompressedNum::from_fixed_point_1s31(*val),
            CompressedNum::Full(val) => *val,
        }).collect()
    }
}

impl FpgaMatrix {
    /// NumPy行列から変換
    pub fn from_numpy(
        numpy_matrix: &[Vec<f32>], 
        conversion_type: MatrixConversionType
    ) -> Result<Self, crate::error::AcceleratorError> {
        if numpy_matrix.is_empty() || 
           numpy_matrix.len() % 16 != 0 || 
           numpy_matrix[0].len() % 16 != 0 {
            return Err(crate::error::AcceleratorError::InvalidDimension(numpy_matrix.len()));
        }

        let converted_data = match conversion_type {
            MatrixConversionType::Full => 
                numpy_matrix.iter()
                    .map(|row| row.iter().map(|&x| CompressedNum::Full(x)).collect())
                    .collect(),
            MatrixConversionType::Trinary => 
                numpy_matrix.iter()
                    .map(|row| row.iter().map(|&x| CompressedNum::trinarize(x)).collect())
                    .collect(),
            MatrixConversionType::FixedPoint1s31 => 
                numpy_matrix.iter()
                    .map(|row| row.iter().map(|&x| CompressedNum::to_fixed_point_1s31(x)).collect())
                    .collect(),
        };

        Ok(Self {
            data: converted_data,
            rows: numpy_matrix.len(),
            cols: numpy_matrix[0].len(),
        })
    }

    /// 変換タイプに応じて浮動小数点数行列に戻す
    pub fn to_numpy(&self) -> Vec<Vec<f32>> {
        self.data.iter()
            .map(|row| 
                row.iter().map(|compressed| match compressed {
                    CompressedNum::Trinary(val) => CompressedNum::from_trinary(*val),
                    CompressedNum::FixedPoint1s31(val) => CompressedNum::from_fixed_point_1s31(*val),
                    CompressedNum::Full(val) => *val,
                }).collect()
            )
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
                        block_row.push(CompressedNum::Full(0.0));
                    }
                    
                    block_data.push(block_row);
                }
                
                // 行のパディング
                while block_data.len() < block_size {
                    block_data.push(vec![CompressedNum::Full(0.0); block_size]);
                }
                
                row_blocks.push(FpgaMatrix::new(block_data).unwrap());
            }
            
            blocks.push(row_blocks);
        }
        
        blocks
    }
}