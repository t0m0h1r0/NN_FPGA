use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("Invalid vector/matrix dimension: expected multiple of 16, got {0}")]
    InvalidDimension(usize),

    #[error("Memory access out of bounds: max {max}, attempted {attempted}")]
    MemoryAccessError {
        max: usize,
        attempted: usize,
    },

    #[error("Unit selection failed: no available units")]
    NoAvailableUnits,

    #[error("Computation type not supported: {0}")]
    UnsupportedComputationType(String),

    #[error("Data conversion error: {0}")]
    DataConversionError(String),

    #[error("Internal FPGA communication error: {0}")]
    CommunicationError(String),
}