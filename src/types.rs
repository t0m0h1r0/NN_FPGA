use std::ops::{Add, Mul, Neg};
use thiserror::Error;

/// FPGAデータ型関連のエラー
#[derive(Error, Debug)]
pub enum TypeConversionError {
    #[error("無効な三値データ: {0}")]
    InvalidTrinaryValue(u8),
    #[error("固定小数点変換エラー: {0}")]
    FixedPointConversionError(String),
}

/// 三値データ型（ZERO, PLUS, MINUS）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrinaryValue {
    Zero,
    Plus,
    Minus,
}

impl TrinaryValue {
    /// f32から三値への変換
    pub fn from_f32(value: f32) -> Self {
        if value == 0.0 {
            TrinaryValue::Zero
        } else if value > 0.0 {
            TrinaryValue::Plus
        } else {
            TrinaryValue::Minus
        }
    }

    /// 三値からf32への変換
    pub fn to_f32(self) -> f32 {
        match self {
            TrinaryValue::Zero => 0.0,
            TrinaryValue::Plus => 1.0,
            TrinaryValue::Minus => -1.0,
        }
    }
}

/// 固定小数点数型（1s.31形式）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedPoint {
    value: i32,
}

impl FixedPoint {
    const FRACTIONAL_BITS: i32 = 31;
    const SCALE: f64 = (1i64 << Self::FRACTIONAL_BITS) as f64;

    /// f32から固定小数点への変換
    pub fn from_f32(value: f32) -> Self {
        // -1.0から1.0の範囲にクランプ
        let clamped = value.max(-1.0).min(1.0);
        let fixed = (clamped as f64 * Self::SCALE).round() as i32;
        Self { value: fixed }
    }

    /// 固定小数点からf32への変換
    pub fn to_f32(self) -> f32 {
        (self.value as f64 / Self::SCALE) as f32
    }

    /// 生のi32値を取得
    pub fn raw_value(self) -> i32 {
        self.value
    }
}

/// FPGA上のデータ表現
#[derive(Debug, Clone, Copy)]
pub enum FpgaValue {
    Trinary(TrinaryValue),
    FixedPoint(FixedPoint),
    Full(f32),
}

impl FpgaValue {
    /// f32からの変換
    pub fn from_f32(value: f32, conversion_type: DataConversionType) -> Self {
        match conversion_type {
            DataConversionType::Trinary => FpgaValue::Trinary(TrinaryValue::from_f32(value)),
            DataConversionType::FixedPoint1s31 => FpgaValue::FixedPoint(FixedPoint::from_f32(value)),
            DataConversionType::Full => FpgaValue::Full(value),
        }
    }

    /// f32への変換
    pub fn to_f32(self) -> f32 {
        match self {
            FpgaValue::Trinary(v) => v.to_f32(),
            FpgaValue::FixedPoint(v) => v.to_f32(),
            FpgaValue::Full(v) => v,
        }
    }
}

/// データ変換タイプ
#[derive(Debug, Clone, Copy)]
pub enum DataConversionType {
    Full,           // 通常の浮動小数点
    Trinary,        // 三値化
    FixedPoint1s31, // 1s.31固定小数点
}

/// 計算タイプ
#[derive(Debug, Clone, Copy)]
pub enum ComputationType {
    Add,
    Multiply,
    Tanh,
    ReLU,
    MatrixVectorMultiply,
}

/// FPGAの行列・ベクトルのサイズ定数
pub const MATRIX_SIZE: usize = 16;
pub const VECTOR_SIZE: usize = 16;