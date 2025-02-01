//! ニューラルネットワークアクセラレータ
//!
//! FPGAベースの高性能ニューラルネットワーク演算ライブラリ

// 内部モジュール
mod types;
mod error;
mod compute;
mod fpga;
mod memory;
mod executor;
mod scheduler;
mod monitor;

// 公開モジュール
pub mod prelude {
    //! 一般的に使用される型と関数の再エクスポート

    pub use crate::{
        types::{
            UnitId, 
            Operation, 
            OperationStatus, 
            ActivationFunction,
            Priority,
        },
        error::{Result, DomainError},
        compute::Vector,
    };
}

// メインインターフェース
pub struct Accelerator {
    /// 演算実行エンジン
    executor: Arc<dyn executor::OperationExecutor>,
    /// スケジューラ
    scheduler: Arc<scheduler::Scheduler>,
    /// システムモニター
    monitor: Arc<monitor::Monitor>,
    /// メモリマネージャ
    memory: Arc<memory::MemoryManager>,
}

impl Accelerator {
    /// 新規アクセラレータの初期化
    pub async fn new() -> Result<Self> {
        // FPGAインターフェースの初期化
        let fpga = Box::new(fpga::RealFpga::new());
        
        // メモリマネージャの初期化
        let memory = Arc::new(memory::MemoryManager::new(
            1024 * 1024,  // 1MBメモリ
            16            // 16バイトブロック
        )?);
        
        // 演算実行エンジンの初期化
        let executor = Arc::new(executor::Executor::new(
            fpga,
            memory.clone()
        ));
        
        // スケジューラの初期化
        let scheduler = Arc::new(scheduler::Scheduler::new(
            executor.clone()
        ));
        
        // モニターの初期化
        let monitor = Arc::new(monitor::Monitor::new(
            Box::new(fpga::RealFpga::new()),
            memory.clone(),
            scheduler.clone()
        ));

        // コンポーネントの起動
        scheduler.start().await?;
        monitor.start().await?;

        Ok(Self {
            executor,
            scheduler,
            monitor,
            memory,
        })
    }

    /// モックアクセラレータの生成（テスト用）
    pub fn new_mock() -> Self {
        // モック実装
        let fpga = Box::new(fpga::MockFpga::default());
        let memory = Arc::new(memory::MemoryManager::new(1024, 16).unwrap());
        
        let executor = Arc::new(executor::Executor::new(
            fpga,
            memory.clone()
        ));
        
        let scheduler = Arc::new(scheduler::Scheduler::new(
            executor.clone()
        ));
        
        let monitor = Arc::new(monitor::Monitor::new(
            Box::new(fpga::MockFpga::default()),
            memory.clone(),
            scheduler.clone()
        ));

        Self {
            executor,
            scheduler,
            monitor,
            memory,
        }
    }

    /// 演算の実行
    pub async fn execute(
        &self, 
        operation: types::Operation, 
        unit: types::UnitId
    ) -> Result<types::OperationStatus> {
        let context = executor::OperationContext::new(operation, unit);
        self.executor.execute(context).await
    }

    /// システムステータスの取得
    pub async fn status(&self) -> Result<monitor::SystemStatus> {
        Ok(self.monitor.status_receiver().borrow().clone())
    }

    /// 演算のキャンセル
    pub async fn cancel(&self, unit: types::UnitId) -> Result<()> {
        self.scheduler.cancel_all(unit).await
    }
}

// 必要な依存関係のインポート
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_accelerator_mock_creation() {
        let accelerator = Accelerator::new_mock();
        
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let unit = types::UnitId::new(0).unwrap();
            let op = types::Operation::Copy { 
                source: types::UnitId::new(1).unwrap() 
            };

            // 基本的な演算実行テスト
            let status = accelerator.execute(op, unit).await.unwrap();
            assert!(matches!(status, types::OperationStatus::Success));
        });
    }
}