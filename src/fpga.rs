//! FPGA通信インターフェースモジュール
//!
//! FPGAデバイスとの安全で柔軟な通信を提供します。

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use serde::{Serialize, Deserialize};
use bytes::{BufMut, BytesMut};

use crate::types::{
    UnitId, 
    Operation, 
    ActivationFunction,
    OperationStatus,
};
use crate::error::{Result, DomainError};

/// プロトコルバージョン
const PROTOCOL_VERSION: u8 = 2;

/// 最大パケットサイズ
const MAX_PACKET_SIZE: usize = 1024;

/// 通信タイムアウト
const COMMUNICATION_TIMEOUT: Duration = Duration::from_secs(5);

/// デバイスパス
const FPGA_DEVICE_PATH: &str = "/dev/fpga0";

/// FPGA通信設定
#[derive(Debug, Clone)]
pub struct FpgaConfig {
    /// デバイスパス
    pub device_path: String,
    /// 通信タイムアウト
    pub timeout: Duration,
}

impl Default for FpgaConfig {
    fn default() -> Self {
        Self {
            device_path: FPGA_DEVICE_PATH.to_string(),
            timeout: COMMUNICATION_TIMEOUT,
        }
    }
}

/// FPGAコマンド
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FpgaCommand {
    /// 演算実行
    Execute {
        /// ターゲットユニット
        unit_id: UnitId,
        /// 演算タイプ
        operation: Operation,
    },
    /// ユニットリセット
    Reset {
        /// リセットするユニット
        unit_id: UnitId,
    },
    /// システム全体のリセット
    SystemReset,
    /// ステータス問い合わせ
    QueryStatus {
        /// 問い合わせるユニット
        unit_id: Option<UnitId>,
    },
}

/// FPGAレスポンス
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FpgaResponse {
    /// 演算ステータス
    Status {
        /// 対象ユニット
        unit_id: UnitId,
        /// 演算ステータス
        status: OperationStatus,
    },
    /// エラーレスポンス
    Error {
        /// エラーコード
        code: u8,
        /// エラーメッセージ
        message: String,
    },
    /// システムステータス
    SystemStatus {
        /// 温度
        temperature: f32,
        /// リソース使用率
        utilization: f32,
    },
}

/// FPGAインターフェースのトレイト定義
#[async_trait::async_trait]
pub trait FpgaInterface: Send + Sync {
    /// FPGA初期化
    async fn initialize(&mut self, config: &FpgaConfig) -> Result<()>;
    
    /// コマンド送信
    async fn send_command(&mut self, command: FpgaCommand) -> Result<()>;
    
    /// レスポンス受信
    async fn receive_response(&mut self) -> Result<FpgaResponse>;
    
    /// デバイス準備状態確認
    async fn is_ready(&self) -> bool;
}

/// 実際のFPGAデバイス用実装
pub struct RealFpga {
    /// デバイス設定
    config: FpgaConfig,
    /// デバイスハンドル（擬似的）
    device: Option<String>,
    /// シーケンス番号
    sequence: u32,
    /// 通信バッファ
    transport: BytesMut,
}

impl RealFpga {
    /// 新規FPGA interfaceの生成
    pub fn new() -> Self {
        Self {
            config: FpgaConfig::default(),
            device: None,
            sequence: 0,
            transport: BytesMut::with_capacity(MAX_PACKET_SIZE),
        }
    }

    /// コマンドのパケット化
    fn pack_command(&mut self, command: &FpgaCommand) -> Result<()> {
        // バッファのクリア
        self.transport.clear();
        
        // プロトコルヘッダーの書き込み
        self.transport.put_u8(PROTOCOL_VERSION);
        self.transport.put_u32(self.sequence);
        
        // コマンドのシリアライズ
        let cmd_bytes = bincode::serialize(command)
            .map_err(|e| DomainError::hardware_error(
                "コマンドシリアライズ", 
                e.to_string().as_bytes()[0]
            ))?;
        
        // サイズチェック
        if cmd_bytes.len() > MAX_PACKET_SIZE - 5 {
            return Err(DomainError::hardware_error(
                "コマンドパック", 
                b'S' // サイズオーバー
            ));
        }
        
        self.transport.put_slice(&cmd_bytes);
        self.sequence += 1;
        
        Ok(())
    }

    /// レスポンスの展開
    fn unpack_response(&mut self) -> Result<FpgaResponse> {
        // 最小パケットサイズチェック
        if self.transport.len() < 5 {
            return Err(DomainError::hardware_error(
                "レスポンス展開", 
                b'L' // 長さ不足
            ));
        }

        // プロトコルバージョンチェック
        let version = self.transport.get_u8();
        if version != PROTOCOL_VERSION {
            return Err(DomainError::hardware_error(
                "プロトコルバージョン", 
                version
            ));
        }

        // シーケンス番号の読み飛ばし
        let _sequence = self.transport.get_u32();
        
        // レスポンスの逆シリアライズ
        bincode::deserialize(&self.transport)
            .map_err(|e| DomainError::hardware_error(
                "レスポンス逆シリアライズ", 
                e.to_string().as_bytes()[0]
            ))
    }
}

#[async_trait::async_trait]
impl FpgaInterface for RealFpga {
    async fn initialize(&mut self, config: &FpgaConfig) -> Result<()> {
        // デバイス設定の更新
        self.config = config.clone();
        self.device = Some(config.device_path.clone());
        
        // タイムアウト付きの初期化処理
        let result = time::timeout(config.timeout, async {
            // 実際のデバイス初期化処理（省略）
            Ok(())
        }).await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(DomainError::hardware_error("初期化", 1)),
            Err(_) => Err(DomainError::hardware_error("初期化タイムアウト", 2)),
        }
    }
    
    async fn send_command(&mut self, command: FpgaCommand) -> Result<()> {
        // コマンドのパケット化
        self.pack_command(&command)?;
        
        // 実際のデバイス通信（モック）
        Ok(())
    }
    
    async fn receive_response(&mut self) -> Result<FpgaResponse> {
        // モック実装
        Ok(FpgaResponse::Status {
            unit_id: UnitId::new(0).unwrap(),
            status: OperationStatus::Success,
        })
    }
    
    async fn is_ready(&self) -> bool {
        self.device.is_some()
    }
}

/// モックFPGA実装（テスト用）
pub struct MockFpga {
    /// 準備状態
    ready: bool,
    /// 最後に送信されたコマンド
    last_command: Arc<Mutex<Option<FpgaCommand>>>,
}

impl Default for MockFpga {
    fn default() -> Self {
        Self {
            ready: false,
            last_command: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl FpgaInterface for MockFpga {
    async fn initialize(&mut self, _config: &FpgaConfig) -> Result<()> {
        self.ready = true;
        Ok(())
    }

    async fn send_command(&mut self, command: FpgaCommand) -> Result<()> {
        if !self.ready {
            return Err(DomainError::hardware_error(
                "FPGA初期化", 
                1
            ));
        }
        let mut last_cmd = self.last_command.lock().await;
        *last_cmd = Some(command);
        Ok(())
    }

    async fn receive_response(&mut self) -> Result<FpgaResponse> {
        if !self.ready {
            return Err(DomainError::hardware_error(
                "FPGA初期化", 
                2
            ));
        }
        
        Ok(FpgaResponse::Status {
            unit_id: UnitId::new(0).unwrap(),
            status: OperationStatus::Success,
        })
    }

    async fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_command_serialization() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut fpga = RealFpga::new();
            let command = FpgaCommand::Execute {
                unit_id: UnitId::new(1).unwrap(),
                operation: Operation::Activate { 
                    function: ActivationFunction::ReLU 
                },
            };

            // コマンドのパケット化
            assert!(fpga.pack_command(&command).is_ok());
        });
    }

    #[test]
    fn test_mock_fpga() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut fpga = MockFpga::default();
            let config = FpgaConfig::default();

            // 初期化
            assert!(fpga.initialize(&config).await.is_ok());
            assert!(fpga.is_ready().await);

            // コマンド送信
            let command = FpgaCommand::QueryStatus { unit_id: None };
            assert!(fpga.send_command(command).await.is_ok());

            // レスポンス受信
            let response = fpga.receive_response().await.unwrap();
            assert!(matches!(response, FpgaResponse::Status { .. }));
        });
    }
}