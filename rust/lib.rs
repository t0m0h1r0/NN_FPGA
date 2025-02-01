//! Neural network accelerator library
//!
//! This library provides a high-level interface for interacting with FPGA-based
//! neural network acceleration hardware. It supports efficient vector operations,
//! parallel processing, and hardware resource management.
//!
//! # Architecture
//!
//! The library is organized into three main layers:
//!
//! - Domain layer: Core types and business logic
//! - Hardware layer: FPGA and memory management
//! - Application layer: High-level operations and monitoring
//!
//! # Example
//!
//! ```rust,no_run
//! use nn_accel::{
//!     Accelerator, Vector,
//!     operation::{Operation, UnitId},
//!     error::Result,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Initialize accelerator
//!     let mut accelerator = Accelerator::new().await?;
//!
//!     // Create and initialize vector
//!     let mut vec1 = Vector::new(32)?;
//!     vec1.bind_to_unit(UnitId::new(0).unwrap()).await?;
//!
//!     // Execute operation
//!     accelerator.execute(
//!         Operation::Copy { source: UnitId::new(1).unwrap() },
//!         vec1.unit_id().unwrap()
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

// Domain layer modules
pub mod domain {
    mod compute;
    mod types;
    mod error;

    pub use compute::Vector;
    pub use types::{Operation, UnitId, OperationStatus};
    pub use error::{Result, Error};

    // Re-export common types for convenience
    pub mod prelude {
        pub use super::{Vector, Operation, UnitId, OperationStatus, Result, Error};
    }
}

// Hardware interface modules
pub mod hw {
    mod fpga;
    mod memory;

    pub(crate) use fpga::{FpgaInterface, RealFpga, MockFpga, FpgaConfig};
    pub(crate) use memory::{MemoryManager, MemoryUsage};
}

// Application layer modules
pub mod app {
    mod executor;
    mod scheduler;
    mod monitor;

    pub(crate) use executor::Executor;
    pub(crate) use scheduler::Scheduler;
    pub(crate) use monitor::Monitor;
}

// Public interface modules
pub mod interface {
    mod rest;

    pub use rest::{create_router, AppState};
}

// Re-export commonly used types
pub use domain::{Vector, Operation, UnitId, OperationStatus, Result, Error};

/// Main accelerator interface
pub struct Accelerator {
    executor: std::sync::Arc<app::Executor>,
    scheduler: std::sync::Arc<app::Scheduler>,
    monitor: std::sync::Arc<app::Monitor>,
}

impl Accelerator {
    /// Create new accelerator instance
    pub async fn new() -> Result<Self> {
        // Initialize components with default configuration
        let fpga = Box::new(hw::RealFpga::new());
        let memory = std::sync::Arc::new(hw::MemoryManager::new(1024 * 1024, 16)?);
        
        let executor = std::sync::Arc::new(
            app::Executor::new(fpga, memory.clone())
        );
        
        let scheduler = std::sync::Arc::new(
            app::Scheduler::new(executor.clone())
        );
        
        let monitor = std::sync::Arc::new(
            app::Monitor::new(
                memory,
                scheduler.clone(),
            )
        );

        // Start monitor
        monitor.start().await?;

        Ok(Self {
            executor,
            scheduler,
            monitor,
        })
    }

    /// Create accelerator with mock FPGA for testing
    pub fn new_mock() -> Self {
        let fpga = Box::new(hw::MockFpga::default());
        let memory = std::sync::Arc::new(
            hw::MemoryManager::new(1024, 16).unwrap()
        );

        let executor = std::sync::Arc::new(
            app::Executor::new(fpga, memory.clone())
        );
        
        let scheduler = std::sync::Arc::new(
            app::Scheduler::new(executor.clone())
        );
        
        let monitor = std::sync::Arc::new(
            app::Monitor::new(
                memory,
                scheduler.clone(),
            )
        );

        Self {
            executor,
            scheduler,
            monitor,
        }
    }

    /// Execute operation on unit
    pub async fn execute(&self, operation: Operation, unit: UnitId) -> Result<OperationStatus> {
        self.executor.execute(operation, unit).await
    }

    /// Get system status
    pub async fn status(&self) -> Result<app::monitor::SystemStatus> {
        self.monitor.status().await
    }

    /// Create REST API router
    pub fn create_router(&self) -> axum::Router {
        interface::create_router(
            self.scheduler.clone(),
            self.monitor.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_accelerator() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            // Test mock accelerator
            let accelerator = Accelerator::new_mock();
            
            let unit = UnitId::new(0).unwrap();
            let op = Operation::Copy {
                source: UnitId::new(1).unwrap(),
            };
            
            let status = accelerator.execute(op, unit).await.unwrap();
            assert!(matches!(status, OperationStatus::Success));

            // Create and test vector
            let mut vec = Vector::new(32).unwrap();
            vec.bind_to_unit(unit).await.unwrap();
            assert_eq!(vec.size(), 32);
        });
    }
}