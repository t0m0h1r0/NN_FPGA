//! Neural Network Accelerator Library
//!
//! This library provides a high-performance interface to FPGA-based
//! neural network acceleration hardware.
//!
//! # Architecture
//!
//! The library is organized into several main components:
//!
//! - Core types and operations (`core` module)
//! - Hardware interface layer (`hw` module)
//! - High-level APIs (`api` module)
//!
//! # Example
//!
//! ```rust,no_run
//! use nn_accel::{
//!     Accelerator, Vector,
//!     types::{UnitId, Activation},
//!     error::Result,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Initialize accelerator
//!     let accelerator = Accelerator::new_mock();
//!     accelerator.initialize().await?;
//!
//!     // Create and initialize vectors
//!     let mut vec1 = Vector::new(32)?;
//!     let mut vec2 = Vector::new(32)?;
//!
//!     vec1.bind_to_unit(UnitId::new(0).unwrap()).await?;
//!     vec2.bind_to_unit(UnitId::new(1).unwrap()).await?;
//!
//!     // Perform operations
//!     accelerator.copy(&vec1, &mut vec2).await?;
//!     accelerator.add(&vec1, &mut vec2).await?;
//!     accelerator.activate(&mut vec2, Activation::ReLU).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod core {
    mod types;
    mod error;
    mod compute;

    pub use self::types::*;
    pub use self::error::*;
    pub use self::compute::*;
}

pub mod hw {
    mod fpga;
    mod unit;
    mod protocol;

    pub(crate) use self::fpga::*;
    pub(crate) use self::unit::*;
    pub(crate) use self::protocol::*;
}

pub mod api {
    mod async_api;
    #[cfg(feature = "python")]
    mod python;

    pub use self::async_api::*;
    #[cfg(feature = "python")]
    pub use self::python::*;
}

// Re-export commonly used types
pub use core::{
    types::{UnitId, Operation, Activation, VectorBlock, Status},
    error::{Result, AccelError},
    compute::Vector,
};

pub use api::{
    AsyncAccelerator,
    Accelerator,
};

/// Create mock accelerator for testing
pub fn create_mock_accelerator() -> Accelerator {
    use hw::{fpga::MockFpga, unit::UnitManager};
    let unit_manager = UnitManager::new(Box::new(MockFpga::default()));
    Accelerator::new(unit_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_mock_accelerator() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let accelerator = create_mock_accelerator();
            
            // Test initialization
            assert!(accelerator.initialize().await.is_ok());
            
            // Test vector operations
            let mut vec1 = Vector::new(32).unwrap();
            let mut vec2 = Vector::new(32).unwrap();
            
            vec1.bind_to_unit(UnitId::new(0).unwrap()).await.unwrap();
            vec2.bind_to_unit(UnitId::new(1).unwrap()).await.unwrap();
            
            assert!(accelerator.copy(&vec1, &mut vec2).await.is_ok());
            assert!(accelerator.add(&vec1, &mut vec2).await.is_ok());
            assert!(accelerator.activate(&mut vec2, Activation::ReLU).await.is_ok());
        });
    }
}