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

// 固定小数点フォーマットの設定
#[derive(Debug, Clone, Copy)]
pub struct QFormat {
    pub q: u8,      // 小数部ビット数
    pub int: u8,    // 整数部ビット数
}

impl QFormat {
    pub fn new(q: u8, int: u8) -> Result<Self> {
        // パラメータの検証
        if !(16..=29).contains(&q) {
            return Err(FpgaError::Configuration(
                format!("小数部ビット数は16から29の間である必要があります: {}", q)
            ));
        }
        if !(2..=12).contains(&int) {
            return Err(FpgaError::Configuration(
                format!("整数部ビット数は2から12の間である必要があります: {}", int)
            ));
        }
        if q + int + 1 != 32 {
            return Err(FpgaError::Configuration(
                "総ビット数は32である必要があります".into()
            ));
        }

        Ok(Self { q, int })
    }

    // f32からの変換（切り捨て）
    pub fn from_f32(&self, value: f32) -> i32 {
        let scaled = value * (1 << self.q) as f32;
        scaled as i32
    }

    // i32からf32への変換
    pub fn to_f32(&self, value: i32) -> f32 {
        value as f32 / (1 << self.q) as f32
    }
}

// 三値型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrinaryValue {
    Zero,
    Plus,
    Minus,
}

impl TrinaryValue {
    pub fn to_i32(self) -> i32 {
        match self {
            TrinaryValue::Zero => 0b00,
            TrinaryValue::Plus => 0b01,
            TrinaryValue::Minus => 0b10,
        }
    }

    pub fn from_i32(value: i32) -> Result<Self> {
        match value & 0b11 {
            0b00 => Ok(TrinaryValue::Zero),
            0b01 => Ok(TrinaryValue::Plus),
            0b10 => Ok(TrinaryValue::Minus),
            _ => Err(FpgaError::TypeConversion("不正な三値".into())),
        }
    }
}

// 固定小数点値
#[derive(Debug, Clone)]
pub struct FpgaValue {
    pub value: i32,
    pub format: QFormat,
}

impl FpgaValue {
    // f32からの生成
    pub fn from_f32(value: f32, format: QFormat) -> Self {
        Self {
            value: format.from_f32(value),
            format,
        }
    }

    // f32への変換
    pub fn as_f32(&self) -> f32 {
        self.format.to_f32(self.value)
    }
}

// 行列の次元定数
pub const MATRIX_SIZE: usize = 16;
pub const VECTOR_SIZE: usize = 16;