pub mod core {
    pub mod error;
    pub mod data_types;
    pub mod device;
}

pub mod python;

pub use core::error::AcceleratorError;
pub use core::data_types::{
    FpgaVector, 
    FpgaMatrix, 
    ComputationType, 
    CompressedNum,
    VectorConversionType,
    MatrixConversionType
};