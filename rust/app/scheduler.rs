//! Operation scheduler implementation (continued)

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{Duration, sleep, Instant};
use futures::future::join_all;
use tracing::{info, warn, error};

use crate::domain::{
    operation::{Operation, UnitId, OperationStatus},
    error::{Result, DomainError},
};
use super::executor::{OperationExecutor, OperationContext};

// ... (前半部分は同じ) ...

impl Scheduler {
    // ... (前半部分のメソッドは同じ) ...

    /// Cancel all operations for unit
    pub async fn cancel_all(&self, unit: UnitId) -> Result<()> {
        // Clear queue
        let mut queues = self.queues.write().await;
        queues[unit.raw() as usize].clear();

        // Cancel current operation
        self.executor.cancel(unit).await?;

        info!("Cancelled all operations for unit {}", unit.raw());
        Ok(())
    }

    /// Get queue status for unit
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
                .map(|op| op.waiting_time())
                .max(),
        }
    }

    /// Get status receiver
    pub fn status_receiver(&self) -> Arc<Mutex<mpsc::Receiver<SchedulerStatus>>> {
        self.status_rx.clone()
    }
}

/// Queue status information
#[derive(Debug, Clone)]
pub struct QueueStatus {
    /// Unit ID
    pub unit: UnitId,
    /// Total number of queued operations
    pub queued_operations: usize,
    /// Number of high priority operations
    pub high_priority: usize,
    /// Number of normal priority operations
    pub normal_priority: usize,
    /// Number of low priority operations
    pub low_priority: usize,
    /// Waiting time of oldest operation
    pub oldest_operation: Option<Duration>,
}

/// Scheduler status updates
#[derive(Debug, Clone)]
pub enum SchedulerStatus {
    /// Operation completed
    OperationComplete {
        /// Target unit
        unit: UnitId,
        /// Operation status
        status: OperationStatus,
    },
    /// Error occurred
    Error {
        /// Target unit
        unit: UnitId,
        /// Error message
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct MockExecutor {
        completed: Arc<RwLock<Vec<OperationContext>>>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                completed: Arc::new(RwLock::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl OperationExecutor for MockExecutor {
        async fn execute(&self, context: OperationContext) -> Result<OperationStatus> {
            let mut completed = self.completed.write().await;
            completed.push(context);
            Ok(OperationStatus::Success)
        }

        async fn cancel(&self, _unit: UnitId) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_scheduler_operation() {
        let executor = Box::new(MockExecutor::new());
        let scheduler = Scheduler::new(executor);

        // Start scheduler
        scheduler.start().await.unwrap();

        // Schedule some operations
        let unit = UnitId::new(0).unwrap();
        let op = Operation::Copy {
            source: UnitId::new(1).unwrap(),
        };

        scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();
        scheduler.schedule(op.clone(), unit, Priority::High).await.unwrap();

        // Check queue status
        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.queued_operations, 2);
        assert_eq!(status.high_priority, 1);
        assert_eq!(status.normal_priority, 1);

        // Wait for operations to complete
        let mut receiver = scheduler.status_receiver().lock().await;
        
        for _ in 0..2 {
            match receiver.recv().await {
                Some(SchedulerStatus::OperationComplete { unit: u, status }) => {
                    assert_eq!(u, unit);
                    assert!(matches!(status, OperationStatus::Success));
                }
                _ => panic!("Unexpected status"),
            }
        }

        // Verify queue is empty
        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.queued_operations, 0);
    }

    #[tokio::test]
    async fn test_queue_limits() {
        let executor = Box::new(MockExecutor::new());
        let scheduler = Scheduler::new(executor);
        
        let unit = UnitId::new(0).unwrap();
        let op = Operation::Copy {
            source: UnitId::new(1).unwrap(),
        };

        // Fill queue to capacity
        for _ in 0..MAX_QUEUE_SIZE {
            scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();
        }

        // Verify queue is full
        let result = scheduler.schedule(op.clone(), unit, Priority::Normal).await;
        assert!(result.is_err());

        // Cancel operations
        scheduler.cancel_all(unit).await.unwrap();

        // Verify queue is empty
        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.queued_operations, 0);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let executor = Box::new(MockExecutor::new());
        let scheduler = Scheduler::new(executor);
        
        let unit = UnitId::new(0).unwrap();
        let op = Operation::Copy {
            source: UnitId::new(1).unwrap(),
        };

        // Schedule operations with different priorities
        scheduler.schedule(op.clone(), unit, Priority::Low).await.unwrap();
        scheduler.schedule(op.clone(), unit, Priority::High).await.unwrap();
        scheduler.schedule(op.clone(), unit, Priority::Normal).await.unwrap();

        let status = scheduler.queue_status(unit).await;
        assert_eq!(status.high_priority, 1);
        assert_eq!(status.normal_priority, 1);
        assert_eq!(status.low_priority, 1);
    }
}