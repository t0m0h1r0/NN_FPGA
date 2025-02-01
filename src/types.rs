//! Core type definitions for neural network accelerator
//!
//! このモジュールは、アクセラレータシステムの基本的な型と定数を定義します。

use std::fmt;
use serde::{Serialize, Deserialize};

/// 定数定義
pub mod constants {
    /// ベクトルのブロックサイズ
    pub const BLOCK_SIZE: usize = 16;
    /// サポートされる最大プロセッシングユニット数
    pub const MAX_UNITS: usize = 256;
    /// ベクトル要素のビット幅
    pub const VECTOR_WIDTH: usize = 32;
}

/// プロセッシングユニットID
///
/// 有効範囲内のユニットIDを安全に管理します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(u8);

impl UnitId {
    /// 新しいユニットIDを作成
    ///
    /// # Errors
    /// 無効なIDの場合はNoneを返します。
    pub fn new(id: u8) -> Option<Self> {
        if (id as usize) < constants::MAX_UNITS {
            Some(Self(id))
        } else {
            None
        }
    }

    /// 生のID値を取得
    pub fn raw(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unit({})", self.0)
    }
}

/// 演算タイプ
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    /// ベクトルコピー
    Copy {
        /// コピー元のユニットID
        source: UnitId
    },
    /// ベクトル加算
    Add {
        /// 加算元のユニットID
        source: UnitId
    },
    /// 活性化関数適用
    Activate {
        /// 適用する活性化関数
        function: ActivationFunction
    },
    /// No Operation
    Nop,
}

/// 活性化関数
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ActivationFunction {
    /// ReLU活性化関数
    ReLU,
    /// Tanh活性化関数
    Tanh,
    /// Sigmoid活性化関数
    Sigmoid,
}

/// 演算の優先度
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Priority {
    /// 高優先度
    High,
    /// 通常優先度
    Normal,
    /// 低優先度
    Low,
}

/// 演算状態
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// 成功
    Success,
    /// 失敗（エラーコード付き）
    Failed {
        /// エラーコード
        code: u8
    },
    /// 処理中
    InProgress,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_id_validation() {
        assert!(UnitId::new(0).is_some());
        assert!(UnitId::new(255).is_some());
        assert!(UnitId::new(256).is_none());
    }

    #[test]
    fn test_operation_serialization() {
        let op = Operation::Copy { 
            source: UnitId::new(1).unwrap() 
        };
        let serialized = bincode::serialize(&op).unwrap();
        let deserialized: Operation = bincode::deserialize(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }
}