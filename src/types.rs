use std::ops::{Add, Mul};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FpgaError {
    #[error("型変換エラー: {0}")]
    TypeConversion(String),
    #[error("計算エラー: {0}")]
    Computation(String),
    #[error("メモリエラー: {0}")]
    Memory(String),
    #[error("設定エラー: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, FpgaError>;

#[derive(Debug, Clone, Copy)]
pub struct FixedPoint {
    value: i32,
    scale: u8,
}

impl FixedPoint {
    pub fn new(value: f32, scale: u8) -> Result<Self> {
        if scale > 31 {
            return Err(FpgaError::Configuration("スケールは31以下である必要があります".into()));
        }
        let scaled = (value * (1 << scale) as f32) as i32;
        Ok(Self { value: scaled, scale })
    }

    pub fn to_f32(&self) -> f32 {
        self.value as f32 / (1 << self.scale) as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrinaryValue {
    Zero,
    Plus,
    Minus,
}

impl TrinaryValue {
    pub fn from_f32(value: f32) -> Self {
        match value {
            v if v == 0.0 => TrinaryValue::Zero,
            v if v > 0.0 => TrinaryValue::Plus,
            _ => TrinaryValue::Minus,
        }
    }

    pub fn to_f32(self) -> f32 {
        match self {
            TrinaryValue::Zero => 0.0,
            TrinaryValue::Plus => 1.0,
            TrinaryValue::Minus => -1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FpgaValue {
    Float(f32),
    Fixed(FixedPoint),
    Trinary(TrinaryValue),
}

impl FpgaValue {
    pub fn as_f32(&self) -> f32 {
        match self {
            FpgaValue::Float(v) => *v,
            FpgaValue::Fixed(v) => v.to_f32(),
            FpgaValue::Trinary(v) => v.to_f32(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DataFormat {
    Full,
    Fixed { scale: u8 },
    Trinary,
}

pub struct DataConverter {
    format: DataFormat,
}

impl DataConverter {
    pub fn new(format: DataFormat) -> Self {
        Self { format }
    }

    pub fn convert(&self, value: f32) -> Result<FpgaValue> {
        match self.format {
            DataFormat::Full => Ok(FpgaValue::Float(value)),
            DataFormat::Fixed { scale } => {
                Ok(FpgaValue::Fixed(FixedPoint::new(value, scale)?))
            }
            DataFormat::Trinary => {
                Ok(FpgaValue::Trinary(TrinaryValue::from_f32(value)))
            }
        }
    }
}

pub const MATRIX_SIZE: usize = 16;
pub const VECTOR_SIZE: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_point_conversion() {
        let fp = FixedPoint::new(0.5, 16).unwrap();
        assert!((fp.to_f32() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_trinary_conversion() {
        assert_eq!(TrinaryValue::from_f32(1.5), TrinaryValue::Plus);
        assert_eq!(TrinaryValue::from_f32(0.0), TrinaryValue::Zero);
        assert_eq!(TrinaryValue::from_f32(-1.5), TrinaryValue::Minus);
    }

    #[test]
    fn test_data_converter() {
        let converter = DataConverter::new(DataFormat::Fixed { scale: 16 });
        let value = converter.convert(0.5).unwrap();
        match value {
            FpgaValue::Fixed(fp) => assert!((fp.to_f32() - 0.5).abs() < 1e-6),
            _ => panic!("Wrong type conversion"),
        }
    }
}