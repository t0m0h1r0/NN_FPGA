//! 演算スケジューラ
//!
//! 効率的で公平なタスク管理と実行を提供します。

use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time;
use tracing::{info, warn, error};

use crate::types::{
    UnitId, 
    Operation, 
    OperationStatus,
    Priority,
};
use crate::error::{Result, DomainError};
use crate::executor::{OperationExecutor, OperationContext};

/// キュー最大サイズ
const MAX_QUEUE_SIZE: usize = 256;
/// デフォルト優先度
const DEFAULT_PRIORITY: Priority = Priority::Normal;
/// キュー処理間隔
const SCHEDULING_INTERVAL: Duration = Duration::from_millis(10);
/// キューのタイムアウト
const QUEUE_TIMEOUT: Duration = Duration::from_secs(60);

/// スケジューラステータス更新
#[derive(Debug, Clone)]
pub enum SchedulerStatus {
    /// 演算完了
    OperationComplete {
        /// ターゲットユニット
        unit: UnitId,
        /// 演算ステータス
        status: OperationStatus,
    },
    /// エラー発生
    Error {
        /// ターゲットユニット
        unit: UnitId,
        /// エラーメッセージ
        error: String,
    },
}

/// 演算エントリ
#[derive(Debug)]
struct OperationEntry {
    /// 演算コンテキスト
    context: OperationContext,
    /// 優先度
    priority: Priority,
    /// キューイング時刻
    queued_at: Instant,
}

/// ユニット別演算キュー
type OperationQueue = VecDeque<OperationEntry>;

/// スケジューラ
pub struct Scheduler {
    /// 演算実行エンジン
    executor: Arc<dyn OperationExecutor>,
    /// ユニット別キュー
    queues: Arc<RwLock<Vec<OperationQueue>>>,
    /// 状態更新チャンネル
    status_tx: mpsc::Sender<SchedulerStatus>,
    /// 状態更新レシーバー
    status_rx: Arc<Mutex<mpsc::Receiver<SchedulerStatus>>>,
    /// アクティブユニット管理
    active_units: Arc<RwLock<HashMap<UnitId, bool>>>,
}

impl Scheduler {
    /// 新規スケジューラの生成
    pub fn new(executor: Arc<dyn OperationExecutor>) -> Self {
        // チャンネルの生成
        let (status_tx, status_rx) = mpsc::channel(100);

        Self {
            executor,
            // ユニット数分のキューを初期化
            queues: Arc::new(RwLock::new(
                (0..256).map(|_| VecDeque::with_capacity(MAX_QUEUE_SIZE)).collect()
            )),
            status_tx,
            status_rx: Arc::new(Mutex::new(status_rx)),
            active_units: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// スケジューラの起動
    pub async fn start(&self) -> Result<()> {
        let queues = Arc::clone(&self.queues);
        let executor = Arc::clone(&self.executor);
        let status_tx = self.status_tx.clone();
        let active_units = Arc::clone(&self.active_units);

        // スケジューリングタスクの起動
        tokio::spawn(async move {
            loop {
                // 全ユニットのキュー処理
                let mut queues_guard = queues.write().await;
                for (unit_id, queue) in queues_guard.iter_mut().enumerate() {
                    let unit = UnitId::new(unit_id as u8).unwrap();
                    
                    // アクティブユニットの確認
                    let mut active_units_guard = active_units.write().await;
                    let is_active = active_units_guard.entry(unit).or_insert(false);

                    // キューが空、またはユニットがアクティブなら次へ
                    if queue.is_empty() || *is_active {
                        continue;
                    }

                    // 最優先タスクの取り出し
                    if let Some(entry) = queue.pop_front() {
                        // タイムアウトチェック
                        if entry.queued_at.elapsed() > QUEUE_TIMEOUT {
                            let _ = status_tx.send(SchedulerStatus::Error {
                                unit,
                                error: "キューイング時間超過".to_string(),
                            }).await;
                            continue;
                        }

                        // ユニットをアクティブに設定
                        *is_active = true;
                        
                        // 非同期タスク実行
                        let executor_clone = Arc::clone(&executor);
                        let status_tx_clone = status_tx.clone();
                        let active_units_clone = Arc::clone(&active_units);

                        tokio::spawn(async move {
                            // 演算実行
                            match executor_clone.execute(entry.context).await {
                                Ok(status) => {
                                    let _ = status_tx_clone.send(
                                        SchedulerStatus::OperationComplete { 
                                            unit, 
                                            status 
                                        }
                                    ).await;
                                }
                                Err(e) => {
                                    let _ = status_tx_clone.send(
                                        SchedulerStatus::Error { 
                                            unit, 
                                            error: e.to_string() 
                                        }
                                    ).await;
                                }
                            }

                            // ユニットのアクティブ状態解除
                            let mut active_units_guard = active_units_clone.write().await;
                            if let Some(active) = active_units_guard.get_mut(&unit) {
                                *active = false;
                            }
                        });
                    }
                }

                // 処理間隔
                drop(queues_guard);
                time::sleep(SCHEDULING_INTERVAL).await;
            }
        });

        Ok(())
    }

    /// 演算のスケジュール
    pub async fn schedule(
        &self, 
        operation: Operation, 
        unit: UnitId,
        priority: Priority,
    ) -> Result<()> {
        let mut queues = self.queues.write().await;
        let queue_index = unit.raw() as usize;

        // キューサイズチェック
        if queues[queue_index].len() >= MAX_QUEUE_SIZE {
            return Err(DomainError::resource_error(
                "演算キュー",
                format!("ユニット {} のキューがいっぱいです", unit.raw())
            ));
        }

        // 演算エントリの作成
        let context = OperationContext::new(operation, unit);
        let entry = OperationEntry {
            context,
            priority: priority.clone(),
            queued_at: Instant::now(),
        };

        // 優先度に応じたキュー挿入
        match priority {
            Priority::High => queues[queue_index].push_front(entry),
            Priority::Normal => queues[queue_index].push_back(entry),
            Priority::Low => {
                // 低優先度は最後尾に追加
                queues[queue_index].push_back(entry);
            }
        }

        Ok(())
    }

    /// ユニット別キューステータス取得
    pub async fn queue_status(&self, unit: UnitId) -> QueueStatus {
        let queues = self.queues.read().await;
        let queue = &queues[unit.raw() as usize];

        QueueStatus {
            unit,
            queued_operations: queue.len(),
            high_priority: queue.iter().filter(|op| op.priority == Priority::High).count(),
            normal_priority: queue.iter().filter(|op| op.priority == Priority::Normal).count(),
            low_priority: queue.iter().filter(|op| op.priority == Priority::Low).count(),
            oldest_operation: queue.iter()
                .map(|op| op.queued_at.elapsed())
                .max(),
        }
    }

    /// 全演算のキャンセル
    pub async fn cancel_all(&self, unit: UnitId) -> Result<()> {
        // キューのクリア
        let mut queues = self.queues.write().await;
        queues[unit.raw() as usize].clear();

        // 実行中の演算をキャンセル
        self.executor.cancel(unit).await?;

        info!("ユニット {} の全演算をキャンセル", unit.raw());
        Ok(())
    }

    /// ステータス受信用チャンネル取得
    pub fn status_receiver(&self) -> Arc<Mutex<mpsc::Receiver<SchedulerStatus>>> {
        Arc::clone(&self.status_rx)
    }
}

/// キューステータス情報
#[derive(Debug, Clone)]
pub struct QueueStatus {
    /// ユニットID
    pub unit: UnitId,
    /// キューイング中の演算数
    pub queued_operations: usize,
    /// 高優先度演算数
    pub high_priority: usize,
    /// 通常優先度演算数
    pub normal_priority: usize,
    /// 低優先度演算数
    pub low_priority: usize,
    /// 最も古い演算の待ち時間
    pub oldest_operation: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::runtime::Runtime;
    
    // モック実行エンジン
    struct MockExecutor;
    
    #[async_trait::async_trait]
    impl OperationExecutor for MockExecutor {
        async fn execute(&self, context: OperationContext) -> Result<OperationStatus> {
            Ok(OperationStatus::Success)
        }
        
        async fn cancel(&self, _unit: UnitId) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_scheduler_basic_flow() {
        let executor = Arc::new(MockExecutor);
        let scheduler = Scheduler::new(executor);
        
        // スケジューラ起動
        scheduler.start().await.unwrap();

        // 演算スケジュール
        let unit = UnitId::new(0).unwrap();
        let op = Operation::Copy { 
            source: UnitId::new(1).unwrap() 
        };

        // 複数の優先度の演算をスケジュール
        scheduler.schedule(op.clone(), unit, Priority::High).await.unwrap();
        scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();
        scheduler.schedule(op.clone(), unit, Priority::Low).await.unwrap();

        // キューステータスの確認
        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.queued_operations, 3);
        assert_eq!(status.high_priority, 1);
        assert_eq!(status.normal_priority, 1);
        assert_eq!(status.low_priority, 1);
    }

    #[tokio::test]
    async fn test_scheduler_queue_limits() {
        let executor = Arc::new(MockExecutor);
        let scheduler = Scheduler::new(executor);
        
        let unit = UnitId::new(0).unwrap();
        let op = Operation::Copy { 
            source: UnitId::new(1).unwrap() 
        };

        // キュー最大数までスケジュール
        for _ in 0..MAX_QUEUE_SIZE {
            scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();
        }

        // キュー満杯時はエラー
        let result = scheduler.schedule(op.clone(), unit, Priority::Normal).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_cancellation() {
        let executor = Arc::new(MockExecutor);
        let scheduler = Scheduler::new(executor);
        
        let unit = UnitId::new(0).unwrap();
        
        // 演算スケジュール
        let op = Operation::Copy { 
            source: UnitId::new(1).unwrap() 
        };
        scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();

        // 全演算キャンセル
        scheduler.cancel_all(unit).await.unwrap();

        // キューステータスの確認
        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.queued_operations, 0);
    }
}