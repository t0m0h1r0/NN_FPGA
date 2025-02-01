use crate::core::data_types::{FpgaVector, FpgaMatrix, ComputationType};
use crate::error::AcceleratorError;
use log::{info, error};

pub struct FpgaAccelerator {
    total_units: usize,
    available_units: Vec<bool>,
    memory_vector_size: usize,
    memory_matrix_size: usize,
    block_size: usize,
}

impl FpgaAccelerator {
    pub fn new() -> Self {
        Self {
            total_units: 256,
            available_units: vec![true; 256],
            memory_vector_size: 64,
            memory_matrix_size: 256,
            block_size: 16,
        }
    }

    pub fn compute(
        &mut self, 
        input: &impl ComputeInput, 
        computation_type: ComputationType
    ) -> Result<FpgaVector, AcceleratorError> {
        match computation_type {
            ComputationType::MatrixVectorMultiply => {
                self.matrix_vector_multiply(input)
            },
            _ => {
                if let Some(vector) = input.as_vector() {
                    self.scalar_compute(vector, computation_type)
                } else {
                    Err(AcceleratorError::UnsupportedComputationType(
                        "Input type not supported for this computation".to_string()
                    ))
                }
            }
        }
    }

    fn matrix_vector_multiply(
        &mut self, 
        input: &impl ComputeInput
    ) -> Result<FpgaVector, AcceleratorError> {
        let matrix = input.as_matrix().ok_or_else(|| 
            AcceleratorError::DataConversionError("Expected matrix input".to_string())
        )?;

        // 行列をブロックに分割
        let matrix_blocks = matrix.split_into_blocks(self.block_size);
        let mut result_vector = Vec::new();

        for row_blocks in matrix_blocks {
            let mut row_result = vec![0.0; self.block_size];

            for block in row_blocks {
                let unit_id = self.select_unit()?;
                
                // 16x16ブロックごとに計算
                let block_result = self.compute_matrix_block(&block, input)?;
                
                // 部分結果を累積
                for (i, val) in block_result.data.iter().enumerate() {
                    row_result[i] += val;
                }

                self.release_unit(unit_id);
            }

            result_vector.extend_from_slice(&row_result);
        }

        FpgaVector::new(result_vector)
    }

    fn compute_matrix_block(
        &mut self, 
        matrix_block: &FpgaMatrix,
        input: &impl ComputeInput
    ) -> Result<FpgaVector, AcceleratorError> {
        let input_vector = input.as_vector().ok_or_else(|| 
            AcceleratorError::DataConversionError("Expected vector input".to_string())
        )?;

        // 行列ブロックとベクトルの乗算
        let mut block_result = Vec::new();
        for row in &matrix_block.data {
            let dot_product: f32 = row.iter()
                .zip(input_vector.data.iter())
                .map(|(a, b)| a * b)
                .sum();
            block_result.push(dot_product);
        }

        FpgaVector::new(block_result)
    }

    fn scalar_compute(
        &mut self, 
        input: &FpgaVector, 
        computation_type: ComputationType
    ) -> Result<FpgaVector, AcceleratorError> {
        let unit_id = self.select_unit()?;
        
        let result = match computation_type {
            ComputationType::Add => input.data.iter().map(|&x| x + 1.0).collect(),
            ComputationType::Multiply => input.data.iter().map(|&x| x * 2.0).collect(),
            ComputationType::Tanh => input.data.iter().map(|&x| x.tanh()).collect(),
            ComputationType::ReLU => input.data.iter().map(|&x| x.max(0.0)).collect(),
            _ => return Err(AcceleratorError::UnsupportedComputationType(
                format!("Unsupported computation type: {:?}", computation_type)
            )),
        };

        self.release_unit(unit_id);
        FpgaVector::new(result)
    }

    fn select_unit(&mut self) -> Result<usize, AcceleratorError> {
        if let Some(unit_id) = self.available_units.iter().position(|&x| x) {
            self.available_units[unit_id] = false;
            Ok(unit_id)
        } else {
            error!("No available units for computation");
            Err(AcceleratorError::NoAvailableUnits)
        }
    }

    pub fn release_unit(&mut self, unit_id: usize) {
        if unit_id < self.total_units {
            self.available_units[unit_id] = true;
        }
    }
}

// トレイト: 計算入力の抽象化
pub trait ComputeInput {
    fn as_vector(&self) -> Option<&FpgaVector>;
    fn as_matrix(&self) -> Option<&FpgaMatrix>;
}

// 実装例
impl ComputeInput for FpgaVector {
    fn as_vector(&self) -> Option<&FpgaVector> {
        Some(self)
    }
    fn as_matrix(&self) -> Option<&FpgaMatrix> {
        None
    }
}

impl ComputeInput for FpgaMatrix {
    fn as_vector(&self) -> Option<&FpgaVector> {
        None
    }
    fn as_matrix(&self) -> Option<&FpgaMatrix> {
        Some(self)
    }
}

impl Default for FpgaAccelerator {
    fn default() -> Self {
        Self::new()
    }
}