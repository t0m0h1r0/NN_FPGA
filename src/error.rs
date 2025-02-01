//! 堅牢で詳細なエラーハンドリングモジュール
//!
//! このモジュールは、アクセラレータシステムの包括的なエラー管理を提供します。

use std::fmt;
use thiserror::Error;
use crate::types::{UnitId, Operation};

/// ドメインレベルの Result 型
pub type Result<T> = std::result::Result<T, DomainError>;

/// ドメイン固有のエラー定義
#[derive(Error, Debug)]
pub enum DomainError {
    /// リソース関連のエラー
    #[error("リソースエラー: {resource} - {details}")]
    ResourceError {
        /// 影響を受けるリソース
        resource: String,
        /// エラーの詳細
        details: String,
    },

    /// 演算実行時のエラー
    #[error("演算エラー: {operation} - {reason}")]
    OperationError {
        /// 失敗した演算
        operation: Operation,
        /// エラーの理由
        reason: String,
    },

    /// ハードウェア関連のエラー
    #[error("ハードウェアエラー: {component} - コード {code}")]
    HardwareError {
        /// 影響を受けるコンポーネント
        component: String,
        /// エラーコード
        code: u8,
    },

    /// メモリ関連のエラー
    #[error("メモリエラー: {details}")]
    MemoryError {
        /// メモリエラーの詳細
        details: String,
    },

    /// ユニット関連のエラー
    #[error("ユニットエラー: {unit} - {message}")]
    UnitError {
        /// 影響を受けるユニット
        unit: UnitId,
        /// エラーメッセージ
        message: String,
    },

    /// 設定関連のエラー
    #[error("設定エラー: {key} - {reason}")]
    ConfigurationError {
        /// 問題のある設定キー
        key: String,
        /// エラーの理由
        reason: String,
    },

    /// 外部ライブラリや予期せぬエラー
    #[error("予期せぬエラー: {0}")]
    Unexpected(#[from] anyhow::Error),
}

/// エラー生成のヘルパーメソッド
impl DomainError {
    /// リソースエラーを作成
    pub fn resource_error(resource: impl ToString, details: impl ToString) -> Self {
        Self::ResourceError {
            resource: resource.to_string(),
            details: details.to_string(),
        }
    }

    /// 演算エラーを作成
    pub fn operation_error(operation: Operation, reason: impl ToString) -> Self {
        Self::OperationError {
            operation,
            reason: reason.to_string(),
        }
    }

    /// ハードウェアエラーを作成
    pub fn hardware_error(component: impl ToString, code: u8) -> Self {
        Self::HardwareError {
            component: component.to_string(),
            code,
        }
    }

    /// メモリエラーを作成
    pub fn memory_error(details: impl ToString) -> Self {
        Self::MemoryError {
            details: details.to_string(),
        }
    }

    /// ユニットエラーを作成
    pub fn unit_error(unit: UnitId, message: impl ToString) -> Self {
        Self::UnitError {
            unit,
            message: message.to_string(),
        }
    }

    /// 設定エラーを作成
    pub fn config_error(key: impl ToString, reason: impl ToString) -> Self {
        Self::ConfigurationError {
            key: key.to_string(),
            reason: reason.to_string(),
        }
    }
}

/// カスタムエラートレイト
pub trait ErrorExt {
    /// エラーの詳細な説明を取得
    fn detailed_description(&self) -> String;
}

impl ErrorExt for DomainError {
    fn detailed_description(&self) -> String {
        match self {
            DomainError::ResourceError { resource, details } => 
                format!("リソース '{}'で問題が発生: {}", resource, details),
            DomainError::OperationError { operation, reason } => 
                format!("演算 {:?} の実行中にエラー: {}", operation, reason),
            DomainError::HardwareError { component, code } => 
                format!("ハードウェアコンポーネント '{}' のエラーコード: {}", component, code),
            DomainError::MemoryError { details } => 
                format!("メモリ管理エラー: {}", details),
            DomainError::UnitError { unit, message } => 
                format!"{} のエラー: {}", unit, message),
            DomainError::ConfigurationError { key, reason } => 
                format!("設定 '{}' の構成エラー: {}", key, reason),
            DomainError::Unexpected(err) => 
                format!("予期せぬエラー: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Operation, UnitId};

    #[test]
    fn test_error_creation() {
        let unit = UnitId::new(5).unwrap();

        // リソースエラーテスト
        let resource_err = DomainError::resource_error("メモリ", "容量不足");
        assert!(resource_err.to_string().contains("メモリ"));

        // 演算エラーテスト
        let op_err = DomainError::operation_error(
            Operation::Copy { source: unit }, 
            "コピー中にエラー発生"
        );
        assert!(op_err.to_string().contains("コピー"));

        // ユニットエラーテスト
        let unit_err = DomainError::unit_error(unit, "無効な状態");
        assert!(unit_err.to_string().contains("Unit(5)"));
    }

    #[test]
    fn test_error_detailed_description() {
        let unit = UnitId::new(5).unwrap();
        let err = DomainError::unit_error(unit, "リソース割り当てエラー");
        
        let description = err.detailed_description();
        assert!(description.contains("Unit(5)"));
        assert!(description.contains("リソース割り当てエラー"));
    }
}