//! 演算実行エンジン
//!
//! FPGAでの演算実行を管理し、信頼性の高い処理を提供します。

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{info, warn, error};

use crate::types::{
    UnitId, 
    Operation, 
    OperationStatus,
};
use crate::error::{Result, DomainError};
use crate::fpga::{
    FpgaInterface, 
    FpgaCommand, 
    FpgaResponse,
};
use crate::memory::{
    MemoryManager, 
    BlockId, 
    LockReason,
};

/// 最大リトライ回数
const MAX_RETRIES: u32 = 3;
/// リトライ遅延（ミリ秒）
const RETRY_DELAY_MS: u64 = 100;
/// 演算タイムアウト
const OPERATION_TIMEOUT: Duration = Duration::from_secs(10);

/// 演算コンテキスト
#[derive(Debug)]
pub struct OperationContext {
    /// 実行する演算
    pub operation: Operation,
    /// ターゲットユニット
    pub unit: UnitId,
    /// 関連するメモリブロック
    pub block: Option<BlockId>,
    /// リトライ回数
    pub retries: u32,
    /// 開始タイムスタンプ
    pub start_time: Instant,
}

impl OperationContext {
    /// 新規演算コンテキストの作成
    pub fn new(operation: Operation, unit: UnitId) -> Self {
        Self {
            operation,
            unit,
            block: None,
            retries: 0,
            start_time: Instant::now(),
        }
    }

    /// リトライ回数超過の確認
    pub fn exceeded_retries(&self) -> bool {
        self.retries >= MAX_RETRIES
    }

    /// 演算期間の取得
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// 演算実行トレイト
#[async_trait::async_trait]
pub trait OperationExecutor: Send + Sync {
    /// 演算の実行
    async fn execute(&self, context: OperationContext) -> Result<OperationStatus>;
    
    /// 演算のキャンセル
    async fn cancel(&self, unit: UnitId) -> Result<()>;
}

/// メイン実行エンジン
pub struct Executor {
    /// FPGAインターフェース
    fpga: Arc<Mutex<Box<dyn FpgaInterface>>>,
    /// メモリマネージャ
    memory: Arc<MemoryManager>,
    /// アクティブな演算
    active_operations: Arc<RwLock<Vec<OperationContext>>>,
}

impl Executor {
    /// 新規実行エンジンの生成
    pub fn new(
        fpga: Box<dyn FpgaInterface>,
        memory: Arc<MemoryManager>,
    ) -> Self {
        Self {
            fpga: Arc::new(Mutex::new(fpga)),
            memory,
            active_operations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 演算準備
    async fn prepare_operation(&self, context: &mut OperationContext) -> Result<()> {
        // メモリブロックのロック
        if let Some(block_id) = context.block {
            self.memory.lock(
                block_id,
                context.unit,
                LockReason::Writing
            ).await?;
        }

        // アクティブ演算リストへの追加
        let mut active_ops = self.active_operations.write().await;
        active_ops.push(context.clone());

        Ok(())
    }

    /// 演算完了処理
    async fn complete_operation(
        &self,
        context: &OperationContext,
        status: OperationStatus
    ) -> Result<()> {
        // メモリブロックのロック解除
        if let Some(block_id) = context.block {
            self.memory.unlock(block_id).await?;
        }

        // アクティブ演算リストからの削除
        let mut active_ops = self.active_operations.write().await;
        active_ops.retain(|op| op.unit != context.unit);

        // 演算完了のログ
        match status {
            OperationStatus::Success => {
                info!(
                    "演算成功: {:?} (ユニット {})",
                    context.operation,
                    context.unit.raw()
                );
            }
            OperationStatus::Failed { code } => {
                error!(
                    "演算失敗: {:?} (ユニット {}, エラーコード {})",
                    context.operation,
                    context.unit.raw(),
                    code
                );
            }
            OperationStatus::InProgress => {
                warn!(
                    "演算進行中: {:?} (ユニット {})",
                    context.operation,
                    context.unit.raw()
                );
            }
        }

        Ok(())
    }

    /// 演算リトライ
    async fn retry_operation(&self, mut context: OperationContext) -> Result<OperationStatus> {
        // リトライ回数のインクリメント
        context.retries += 1;
        
        warn!(
            "演算リトライ: {:?} (ユニット {}, リトライ {}/{})",
            context.operation,
            context.unit.raw(),
            context.retries,
            MAX_RETRIES
        );

        // リトライ遅延
        time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
        
        // 演算の再実行
        self.execute(context).await
    }
}

#[async_trait::async_trait]
impl OperationExecutor for Executor {
    async fn execute(&self, mut context: OperationContext) -> Result<OperationStatus> {
        // 演算準備
        self.prepare_operation(&mut context).await?;

        // タイムアウト付きの演算実行
        let result = time::timeout(OPERATION_TIMEOUT, async {
            // FPGAへのコマンド送信
            let mut fpga = self.fpga.lock().await;
            fpga.send_command(FpgaCommand::Execute {
                unit_id: context.unit,
                operation: context.operation.clone(),
            }).await?;

            // レスポンス待機
            let response = fpga.receive_response().await?;
            
            // レスポンス処理
            match response {
                FpgaResponse::Status { status, .. } => {
                    match status {
                        OperationStatus::Success => {
                            self.complete_operation(&context, status).await?;
                            Ok(status)
                        }
                        OperationStatus::Failed { .. } => {
                            if context.exceeded_retries() {
                                self.complete_operation(&context, status).await?;
                                Ok(status)
                            } else {
                                self.retry_operation(context).await
                            }
                        }
                        OperationStatus::InProgress => {
                            self.complete_operation(&context, status).await?;
                            Ok(status)
                        }
                    }
                }
                FpgaResponse::Error { code, message } => {
                    error!(
                        "FPGAエラー: {} (コード: {})",
                        message,
                        code
                    );
                    Err(DomainError::operation_error(
                        context.operation.clone(),
                        message
                    ))
                }
            }
        }).await;

        // タイムアウト処理
        match result {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!(
                    "演算タイムアウト: {:?} (ユニット {})",
                    context.operation,
                    context.unit.raw()
                );
                Err(DomainError::operation_error(
                    context.operation.clone(), 
                    "演算タイムアウト"
                ))
            }
        }
    }

    async fn cancel(&self, unit: UnitId) -> Result<()> {
        // キャンセルコマンド送信
        let mut fpga = self.fpga.lock().await;
        fpga.send_command(FpgaCommand::Reset { 
            unit_id: unit 
        }).await?;

        // アクティブ演算のクリーンアップ
        let mut active_ops = self.active_operations.write().await;
        if let Some(op) = active_ops.iter().find(|op| op.unit == unit) {
            if let Some(block_id) = op.block {
                self.memory.unlock(block_id).await?;
            }
        }
        active_ops.retain(|op| op.unit != unit);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpga::MockFpga;

    #[tokio::test]
    async fn test_operation_execution() {
        let memory = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let executor = Executor::new(
            Box::new(MockFpga::default()),
            memory.clone(),
        );

        // 正常な実行テスト
        let context = OperationContext::new(
            Operation::Copy {
                source: UnitId::new(0).unwrap(),
            },
            UnitId::new(1).unwrap(),
        );

        let status = executor.execute(context).await.unwrap();
        assert!(matches!(status, OperationStatus::Success));

        // キャンセルテスト
        let unit = UnitId::new(1).unwrap();
        assert!(executor.cancel(unit).await.is_ok());
    }

    #[tokio::test]
    async fn test_operation_retry() {
        let memory = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let executor = Executor::new(
            Box::new(MockFpga::default()),
            memory.clone(),
        );

        let mut context = OperationContext::new(
            Operation::Copy {
                source: UnitId::new(0).unwrap(),
            },
            UnitId::new(1).unwrap(),
        );

        // リトライシミュレーション
        context.retries = MAX_RETRIES - 1;
        let status = executor.execute(context).await.unwrap();
        
        // モックFPGAは最終的に成功するはず
        assert!(matches!(status, OperationStatus::Success));
    }
}