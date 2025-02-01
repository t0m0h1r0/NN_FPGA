use half::f16;
use num_traits::Float;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
pub enum CompressedNum {
    /// 三値化表現: 0（ゼロ）, 1（正）, 2（負）
    Trinary(u8),
    /// 固定小数点数（8ビット整数部, 24ビット小数部）
    FixedPoint(i32),
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

    /// 浮動小数点数を固定小数点数に変換
    pub fn to_fixed_point(value: f32) -> Self {
        // 8ビット整数部, 24ビット小数部
        // スケーリングと丸めを行う
        const FRACTIONAL_BITS: i32 = 24;
        const SCALE: f32 = (1 << FRACTIONAL_BITS) as f32;
        
        // オーバーフロー/アンダーフロー対策
        let clamped = value.max(-128.0).min(127.0);
        
        // 固定小数点数への変換
        let fixed = (clamped * SCALE).round() as i32;
        
        CompressedNum::FixedPoint(fixed)
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

    /// 固定小数点数を浮動小数点数に戻す
    pub fn from_fixed_point(fixed: i32) -> f32 {
        const FRACTIONAL_BITS: i32 = 24;
        const SCALE: f32 = (1 << FRACTIONAL_BITS) as f32;
        
        (fixed as f32) / SCALE
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

impl FpgaVector {
    /// NumPyベクトルから変換
    pub fn from_numpy(numpy_vec: &[f32], conversion_type: VectorConversionType) -> Result<Self, crate::error::AcceleratorError> {
        if numpy_vec.len() % 16 != 0 {
            return Err(crate::error::AcceleratorError::InvalidDimension(numpy_vec.len()));
        }

        let converted_data = match conversion_type {
            VectorConversionType::Full => numpy_vec.iter().map(|&x| CompressedNum::Full(x)).collect(),
            VectorConversionType::Trinary => numpy_vec.iter().map(|&x| CompressedNum::trinarize(x)).collect(),
            VectorConversionType::FixedPoint => numpy_vec.iter().map(|&x| CompressedNum::to_fixed_point(x)).collect(),
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
            CompressedNum::FixedPoint(val) => CompressedNum::from_fixed_point(*val),
            CompressedNum::Full(val) => *val,
        }).collect()
    }
}

impl FpgaMatrix {
    /// NumPy行列から変換
    pub fn from_numpy(numpy_matrix: &[Vec<f32>], conversion_type: MatrixConversionType) -> Result<Self, crate::error::AcceleratorError> {
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
            MatrixConversionType::FixedPoint => 
                numpy_matrix.iter()
                    .map(|row| row.iter().map(|&x| CompressedNum::to_fixed_point(x)).collect())
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
                    CompressedNum::FixedPoint(val) => CompressedNum::from_fixed_point(*val),
                    CompressedNum::Full(val) => *val,
                }).collect()
            )
            .collect()
    }
}

/// ベクトル変換タイプ
#[derive(Debug, Clone, Copy)]
pub enum VectorConversionType {
    Full,           // 通常の浮動小数点数
    Trinary,        // 三値化
    FixedPoint,     // 固定小数点数
}

/// 行列変換タイプ
#[derive(Debug, Clone, Copy)]
pub enum MatrixConversionType {
    Full,           // 通常の浮動小数点数
    Trinary,        // 三値化
    FixedPoint,     // 固定小数点数
}