//! Error handling for the neural network accelerator
//!
//! This module defines the error types and related utilities used
//! throughout the accelerator implementation.

use thiserror::Error;
use std::fmt;
use crate::types::UnitId;

/// Result type for accelerator operations
pub type Result<T> = std::result::Result<T, AccelError>;

/// Main error type for accelerator operations
#[derive(Error, Debug)]
pub enum AccelError {
    #[error("Invalid dimension: {0}")]
    Dimension(String),

    #[error("Hardware communication error: {0}")]
    Hardware(#[from] HardwareError),

    #[error("Processing unit error: {0}")]
    Unit(#[from] UnitError),

    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Hardware-specific errors
#[derive(Error, Debug)]
pub enum HardwareError {
    #[error("FPGA communication failed: {0}")]
    Communication(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Hardware timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Device not found: {0}")]
    NotFound(String),
}

/// Processing unit errors
#[derive(Error, Debug)]
pub enum UnitError {
    #[error("Invalid unit ID: {0}")]
    InvalidId(usize),

    #[error("Unit {0} is busy")]
    Busy(UnitId),

    #[error("Operation not supported: {0}")]
    UnsupportedOperation(String),

    #[error("Unit {unit} failed: {reason}")]
    Failed {
        unit: UnitId,
        reason: String,
    },
}

/// Memory-related errors
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Address {0} out of bounds")]
    OutOfBounds(usize),

    #[error("Alignment error at address {0}")]
    Alignment(usize),

    #[error("Access denied: {0}")]
    AccessDenied(String),
}

/// Computation status with error context
#[derive(Debug)]
pub struct StatusWithError<T> {
    pub result: Option<T>,
    pub error: Option<AccelError>,
}

impl<T> StatusWithError<T> {
    /// Create a new successful status with result
    pub fn success(result: T) -> Self {
        Self {
            result: Some(result),
            error: None,
        }
    }

    /// Create a new error status
    pub fn error(error: AccelError) -> Self {
        Self {
            result: None,
            error: Some(error),
        }
    }

    /// Returns true if status contains a result
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }

    /// Returns true if status contains an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Convert status into a Result
    pub fn into_result(self) -> Result<T> {
        match (self.result, self.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(error),
            _ => Err(AccelError::Other(anyhow::anyhow!("Invalid status"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_conversion() {
        let hw_err = HardwareError::Timeout(Duration::from_secs(1));
        let accel_err: AccelError = hw_err.into();
        assert!(matches!(accel_err, AccelError::Hardware(_)));
    }

    #[test]
    fn test_status_with_error() {
        let success = StatusWithError::success(42);
        assert!(success.is_success());
        assert!(!success.is_error());
        assert!(success.into_result().is_ok());

        let error = StatusWithError::error(
            AccelError::Config("test error".to_string())
        );
        assert!(!error.is_success());
        assert!(error.is_error());
        assert!(error.into_result().is_err());
    }
}