pub mod error;
pub mod data_types;
pub mod device;

pub use error::AcceleratorError;
pub use data_types::{
    FpgaVector, 
    FpgaMatrix, 
    ComputationType, 
    CompressedNum,
    VectorConversionType,
    MatrixConversionType
};
pub use device::FpgaAccelerator;