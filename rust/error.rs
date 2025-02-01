// error.rs

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum NNError {
    /// 次元が不正な場合のエラー
    Dimension(String),
    /// 行列が見つからない場合のエラー
    NotFound(String),
    /// ストレージ操作のエラー
    Storage(String),
    /// 計算エラー
    Computation(String),
}

impl Error for NNError {}

impl fmt::Display for NNError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Dimension(msg) => write!(f, "Dimension error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Storage(msg) => write!(f, "Storage error: {}", msg),
            Self::Computation(msg) => write!(f, "Computation error: {}", msg),
        }
    }
}

pub type Result<T> = std::result::Result<T, NNError>;