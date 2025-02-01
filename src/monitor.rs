//! システム監視モジュール
//!
//! アクセラレータのリアルタイム性能監視と診断を提供します。

use std::sync::Arc;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, watch};
use tokio::time;
use tracing::{info, warn, error};
use metrics::{counter, gauge, histogram};

use crate::types::{
    UnitId, 
    Operation, 
    OperationStatus,
};
use crate::error::{Result, DomainError};
use crate::fpga::{FpgaInterface, FpgaStatus};
use crate::memory::MemoryManager;
use crate::scheduler::{Scheduler, QueueStatus};

/// 最大履歴サイズ
const MAX_HISTORY_SIZE: usize = 1000;
/// 監視間隔
const MONITORING_INTERVAL: Duration = Duration::from_secs(1);

/// パフォーマンス統計
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// 1秒あたりの演算数
    pub ops_per_second: f64,
    /// 平均演算レイテンシ
    pub avg_latency: Duration,
    /// ピークメモリ使用量
    pub peak_memory: f64,
    /// FPGA使用率
    pub fpga_utilization: f64,
}

/// 演算メトリクス
#[derive(Debug, Clone)]
struct OperationMetrics {
    /// タイムスタンプ
    timestamp: Instant,
    /// 演算期間
    duration: Duration,
    /// 演算ステータス
    status: OperationStatus,
}

/// システム監視
pub struct Monitor {
    /// FPGAインターフェース
    fpga: Arc<RwLock<Box<dyn FpgaInterface>>>,
    /// メモリマネージャ
    memory_manager: Arc<MemoryManager>,
    /// スケジューラ
    scheduler: Arc<Scheduler>,
    /// 演算履歴
    operation_history: Arc<RwLock<VecDeque<OperationMetrics>>>,
    /// ステータス通知チャネル
    status_tx: watch::Sender<SystemStatus>,
    /// ステータス受信チャネル
    status_rx: watch::Receiver<SystemStatus>,
}

/// システムステータス
#[derive(Debug, Clone)]
pub struct SystemStatus {
    /// FPGA状態
    pub fpga: FpgaStatus,
    /// メモリ使用状況
    pub memory: MemoryUsage,
    /// 各ユニットのキューステータス
    pub unit_queues: Vec<QueueStatus>,
    /// パフォーマンス統計
    pub performance: PerformanceStats,
    /// タイムスタンプ
    pub timestamp: Instant,
}

/// メモリ使用状況
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// 総メモリサイズ
    pub total_size: usize,
    /// 使用中メモリサイズ
    pub used_size: usize,
    /// ブロックサイズ
    pub block_size: usize,
    /// 総ブロック数
    pub total_blocks: usize,
    /// 使用中ブロック数
    pub used_blocks: usize,
    /// ロック中ブロック数
    pub locked_blocks: usize,
}

impl Monitor {
    /// 新規モニターの生成
    pub fn new(
        fpga: Box<dyn FpgaInterface>,
        memory_manager: Arc<MemoryManager>,
        scheduler: Arc<Scheduler>,
    ) -> Self {
        // 初期ステータス
        let initial_status = SystemStatus {
            fpga: FpgaStatus {
                ready: false,
                temperature: 0.0,
                utilization: 0.0,
            },
            memory: MemoryUsage {
                total_size: 0,
                used_size: 0,
                block_size: 0,
                total_blocks: 0,
                used_blocks: 0,
                locked_blocks: 0,
            },
            unit_queues: Vec::new(),
            performance: PerformanceStats {
                ops_per_second: 0.0,
                avg_latency: Duration::from_secs(0),
                peak_memory: 0.0,
                fpga_utilization: 0.0,
            },
            timestamp: Instant::now(),
        };

        // ステータス通知チャネル
        let (status_tx, status_rx) = watch::channel(initial_status);

        Self {
            fpga: Arc::new(RwLock::new(fpga)),
            memory_manager,
            scheduler,
            operation_history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            status_tx,
            status_rx,
        }
    }

    /// モニタリング開始
    pub async fn start(&self) -> Result<()> {
        info!("システムモニターを起動");

        let monitor = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = monitor.update_status().await {
                    error!("システムステータス更新エラー: {}", e);
                }
                time::sleep(MONITORING_INTERVAL).await;
            }
        });

        Ok(())
    }

    /// ステータス受信チャネルの取得
    pub fn status_receiver(&self) -> watch::Receiver<SystemStatus> {
        self.status_rx.clone()
    }

    /// 演算記録
    pub async fn record_operation(
        &self,
        start_time: Instant,
        status: OperationStatus,
    ) {
        let metrics = OperationMetrics {
            timestamp: start_time,
            duration: start_time.elapsed(),
            status,
        };

        // 履歴更新
        let mut history = self.operation_history.write().await;
        if history.len() >= MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(metrics.clone());

        // メトリクス更新
        counter!("operations.total", 1);
        histogram!("operation.duration", metrics.duration.as_secs_f64());
        
        match status {
            OperationStatus::Success => {
                counter!("operations.success", 1);
            }
            OperationStatus::Failed { code } => {
                counter!("operations.failed", 1);
                counter!("operations.error", 1, "code" => code.to_string());
            }
            OperationStatus::InProgress => {
                counter!("operations.in_progress", 1);
            }
        }
    }

    /// パフォーマンス統計の計算
    async fn calculate_performance(&self) -> PerformanceStats {
        let history = self.operation_history.read().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);
        
        // 直近1分間の演算をフィルタ
        let recent_ops: Vec<_> = history.iter()
            .filter(|op| (now - op.timestamp) < window)
            .collect();

        // 1秒あたりの演算数
        let ops_per_second = if recent_ops.is_empty() {
            0.0
        } else {
            recent_ops.len() as f64 / window.as_secs_f64()
        };

        // 平均レイテンシ
        let avg_latency = if recent_ops.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_duration: Duration = recent_ops.iter()
                .map(|op| op.duration)
                .sum();
            total_duration / recent_ops.len() as u32
        };

        // FPGAとメモリの統計情報取得
        let fpga = self.fpga.read().await;
        let fpga_status = fpga.status().await.unwrap_or(FpgaStatus {
            ready: false,
            temperature: 0.0,
            utilization: 0.0,
        });

        let memory_usage = self.memory_manager.usage().await;

        PerformanceStats {
            ops_per_second,
            avg_latency,
            peak_memory: memory_usage.used_size as f64 / memory_usage.total_size as f64,
            fpga_utilization: fpga_status.utilization,
        }
    }

    /// システムステータスの更新
    async fn update_status(&self) -> Result<()> {
        // FPGAステータスの取得
        let fpga = {
            let fpga_guard = self.fpga.read().await;
            fpga_guard.status().await?
        };

        // メモリ使用状況の取得
        let memory = self.memory_manager.usage().await;
        
        // 各ユニットのキューステータス取得
        let mut unit_queues = Vec::new();
        for unit_id in 0..256 {
            if let Some(unit) = UnitId::new(unit_id as u8) {
                unit_queues.push(self.scheduler.queue_status(unit).await);
            }
        }

        // パフォーマンス統計の計算
        let performance = self.calculate_performance().await;

        // メトリクス更新
        gauge!("memory.usage", memory.used_size as f64);
        gauge!("fpga.temperature", fpga.temperature as f64);
        gauge!("fpga.utilization", fpga.utilization);

        // 新しいステータスの生成と通知
        let status = SystemStatus {
            fpga,
            memory,
            unit_queues,
            performance,
            timestamp: Instant::now(),
        };

        self.status_tx.send(status)?;
        Ok(())
    }
}

// Cloneトレイトの実装
impl Clone for Monitor {
    fn clone(&self) -> Self {
        Self {
            fpga: Arc::clone(&self.fpga),
            memory_manager: Arc::clone(&self.memory_manager),
            scheduler: Arc::clone(&self.scheduler),
            operation_history: Arc::clone(&self.operation_history),
            status_tx: self.status_tx.clone(),
            status_rx: self.status_rx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpga::MockFpga;
    use tokio::time;

    #[tokio::test]
    async fn test_monitor_initialization() {
        let memory_manager = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let scheduler = Arc::new(Scheduler::new(Arc::new(MockExecutor)));
        let monitor = Monitor::new(
            Box::new(MockFpga::default()),
            memory_manager.clone(),
            scheduler.clone(),
        );

        // モニター起動
        assert!(monitor.start().await.is_ok());
    }

    #[tokio::test]
    async fn test_operation_recording() {
        let memory_manager = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let scheduler = Arc::new(Scheduler::new(Arc::new(MockExecutor)));
        let monitor = Monitor::new(
            Box::new(MockFpga::default()),
            memory_manager.clone(),
            scheduler.clone(),
        );

        // 演算の記録
        let start_time = Instant::now();
        monitor.record_operation(start_time, OperationStatus::Success).await;
        monitor.record_operation(start_time, OperationStatus::Failed { code: 1 }).await;

        // 少し待機して状態更新
        monitor.start().await.unwrap();
        time::sleep(Duration::from_secs(2)).await;

        // ステータス受信
        let status = monitor.status_receiver().borrow().clone();
        assert!(status.performance.ops_per_second > 0.0);
    }

    // モックの実行エンジン
    struct MockExecutor;
    
    #[async_trait::async_trait]
    impl OperationExecutor for MockExecutor {
        async fn execute(&self, _context: OperationContext) -> Result<OperationStatus> {
            Ok(OperationStatus::Success)
        }
        
        async fn cancel(&self, _unit: UnitId) -> Result<()> {
            Ok(())
        }
    }
}