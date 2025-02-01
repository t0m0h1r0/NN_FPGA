//! Application monitoring and diagnostics
//!
//! This module provides monitoring and diagnostic capabilities for the accelerator.

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::{RwLock, watch};
use tokio::time::{Duration, Instant};
use metrics::{counter, gauge, histogram};
use tracing::{info, warn, error};

use crate::domain::{
    operation::{Operation, UnitId, OperationStatus},
    error::{Result, DomainError},
};
use crate::infra::{
    fpga::{FpgaStatus, FpgaMonitor},
    memory::{MemoryManager, MemoryUsage},
};
use super::scheduler::{Scheduler, QueueStatus};

/// Maximum history size for statistics
const MAX_HISTORY_SIZE: usize = 1000;

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Operations per second
    pub ops_per_second: f64,
    /// Average operation latency
    pub avg_latency: Duration,
    /// Peak memory usage
    pub peak_memory: f64,
    /// FPGA utilization
    pub fpga_utilization: f64,
}

/// Operation metrics
#[derive(Debug, Clone)]
struct OperationMetrics {
    /// Operation timestamp
    timestamp: Instant,
    /// Operation duration
    duration: Duration,
    /// Operation status
    status: OperationStatus,
}

/// System monitor
pub struct Monitor {
    /// FPGA monitor
    fpga_monitor: Arc<FpgaMonitor>,
    /// Memory manager
    memory_manager: Arc<MemoryManager>,
    /// Scheduler
    scheduler: Arc<Scheduler>,
    /// Operation history
    operation_history: Arc<RwLock<VecDeque<OperationMetrics>>>,
    /// Status channel
    status_tx: watch::Sender<SystemStatus>,
    status_rx: watch::Receiver<SystemStatus>,
}

/// System status information
#[derive(Debug, Clone)]
pub struct SystemStatus {
    /// FPGA status
    pub fpga: FpgaStatus,
    /// Memory usage
    pub memory: MemoryUsage,
    /// Queue status for each unit
    pub queues: Vec<QueueStatus>,
    /// Performance statistics
    pub performance: PerformanceStats,
    /// Timestamp
    pub timestamp: Instant,
}

impl Monitor {
    /// Create new monitor
    pub fn new(
        fpga_monitor: Arc<FpgaMonitor>,
        memory_manager: Arc<MemoryManager>,
        scheduler: Arc<Scheduler>,
    ) -> Self {
        let (status_tx, status_rx) = watch::channel(SystemStatus {
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
            queues: Vec::new(),
            performance: PerformanceStats {
                ops_per_second: 0.0,
                avg_latency: Duration::from_secs(0),
                peak_memory: 0.0,
                fpga_utilization: 0.0,
            },
            timestamp: Instant::now(),
        });

        Self {
            fpga_monitor,
            memory_manager,
            scheduler,
            operation_history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            status_tx,
            status_rx,
        }
    }

    /// Start monitoring
    pub async fn start(&self) -> Result<()> {
        info!("Starting system monitor");

        // Start monitoring loop
        let monitor = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = monitor.update_status().await {
                    error!("Failed to update system status: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    /// Get status receiver
    pub fn status_receiver(&self) -> watch::Receiver<SystemStatus> {
        self.status_rx.clone()
    }

    /// Record operation completion
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

        // Update history
        let mut history = self.operation_history.write().await;
        if history.len() >= MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(metrics.clone());

        // Update metrics
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
        }
    }

    /// Calculate performance statistics
    async fn calculate_performance(&self) -> PerformanceStats {
        let history = self.operation_history.read().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);
        
        // Filter operations in the last minute
        let recent_ops: Vec<_> = history.iter()
            .filter(|op| (now - op.timestamp) < window)
            .collect();

        let ops_per_second = if recent_ops.is_empty() {
            0.0
        } else {
            recent_ops.len() as f64 / window.as_secs_f64()
        };

        let avg_latency = if recent_ops.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_duration: Duration = recent_ops.iter()
                .map(|op| op.duration)
                .sum();
            total_duration / recent_ops.len() as u32
        };

        // Get memory and FPGA stats
        let memory = self.memory_manager.usage().await;
        let fpga = self.fpga_monitor.status().await.unwrap();

        PerformanceStats {
            ops_per_second,
            avg_latency,
            peak_memory: memory.used_size as f64 / memory.total_size as f64,
            fpga_utilization: fpga.utilization,
        }
    }

    /// Update system status
    async fn update_status(&self) -> Result<()> {
        // Get component status
        let fpga = self.fpga_monitor.status().await?;
        let memory = self.memory_manager.usage().await;
        
        // Get queue status for all units
        let mut queues = Vec::new();
        for unit_id in 0..=UnitId::MAX_UNITS {
            if let Some(unit) = UnitId::new(unit_id) {
                queues.push(self.scheduler.queue_status(unit).await);
            }
        }

        // Calculate performance stats
        let performance = self.calculate_performance().await;

        // Update metrics
        gauge!("memory.usage", memory.used_size as f64);
        gauge!("fpga.temperature", fpga.temperature);
        gauge!("fpga.utilization", fpga.utilization);

        // Broadcast new status
        let status = SystemStatus {
            fpga,
            memory,
            queues,
            performance,
            timestamp: Instant::now(),
        };

        self.status_tx.send(status)?;
        Ok(())
    }
}

impl Clone for Monitor {
    fn clone(&self) -> Self {
        Self {
            fpga_monitor: self.fpga_monitor.clone(),
            memory_manager: self.memory_manager.clone(),
            scheduler: self.scheduler.clone(),
            operation_history: self.operation_history.clone(),
            status_tx: self.status_tx.clone(),
            status_rx: self.status_rx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_monitor() {
        // Create mock components
        let fpga_monitor = Arc::new(FpgaMonitor::default());
        let memory_manager = Arc::new(MemoryManager::new(1024, 16).unwrap());
        let scheduler = Arc::new(Scheduler::default());

        let monitor = Monitor::new(
            fpga_monitor,
            memory_manager,
            scheduler,
        );

        // Start monitor
        monitor.start().await.unwrap();

        // Record some operations
        monitor.record_operation(
            Instant::now() - Duration::from_millis(100),
            OperationStatus::Success,
        ).await;

        monitor.record_operation(
            Instant::now() - Duration::from_millis(200),
            OperationStatus::Failed { code: 1 },
        ).await;

        // Wait for status update
        sleep(Duration::from_secs(1)).await;

        // Check status
        let status = monitor.status_receiver().borrow().clone();
        assert!(status.performance.ops_per_second > 0.0);
        assert!(status.performance.avg_latency > Duration::from_millis(0));
    }
}