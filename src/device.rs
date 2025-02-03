use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE, DataConverter};
use crate::memory::MatrixBlock;
use crate::math::{Matrix, Vector};
use crate::compute::{ComputeCore, ComputeOperation};

pub struct FpgaAccelerator {
    compute_core: ComputeCore,
    data_converter: DataConverter,
    matrix_rows: usize,
    matrix_cols: usize,
}

impl FpgaAccelerator {
    pub fn new(num_units: usize, data_converter: DataConverter) -> Self {
        Self {
            compute_core: ComputeCore::new(num_units),
            data_converter,
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    pub fn prepare_matrix(&mut self, matrix: &Matrix) -> Result<()> {
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();

        let blocks = matrix.split_blocks()?;
        for (unit_id, block) in blocks.iter().enumerate() {
            if let Some(unit) = self.compute_core.get_unit(unit_id) {
                unit.load_matrix(MatrixBlock::new(
                    block.data.clone(),
                    unit_id * MATRIX_SIZE,
                    0,
                )?)?;
            }
        }
        Ok(())
    }

    pub fn compute_matrix_vector(&mut self, vector: &Vector) -> Result<Vector> {
        if vector.len() != self.matrix_cols {
            return Err(FpgaError::Computation("Vector size mismatch".into()));
        }

        let vector_blocks = vector.split(MATRIX_SIZE)?;
        let mut result = Vec::new();

        for block_row in 0..(self.matrix_rows / MATRIX_SIZE) {
            let units_in_row = std::cmp::min(
                vector_blocks.len(),
                self.compute_core.num_units()
            );

            // 各ユニットにベクトルブロックをロード
            for unit_id in 0..units_in_row {
                let unit = self.compute_core.get_unit(unit_id)?;
                unit.load_vector(vector_blocks[unit_id].data.clone())?;
            }

            // 並列計算の実行
            let partial_results = self.compute_core.execute_parallel(
                ComputeOperation::MatrixVectorMultiply
            )?;

            // 結果の集約
            let mut row_result = vec![FpgaValue::Float(0.0); MATRIX_SIZE];
            for unit_result in partial_results {
                for (i, value) in unit_result.iter().enumerate() {
                    row_result[i] = FpgaValue::Float(
                        row_result[i].as_f32() + value.as_f32()
                    );
                }
            }
            result.extend_from_slice(&row_result);
        }

        Vector::new(result)
    }

    pub fn compute_vector_operation(
        &mut self,
        vector: &Vector,
        operation: ComputeOperation
    ) -> Result<Vector> {
        let vector_blocks = vector.split(MATRIX_SIZE)?;
        let mut result = Vec::new();

        for (unit_id, block) in vector_blocks.iter().enumerate() {
            if let Some(unit) = self.compute_core.get_unit(unit_id) {
                unit.load_vector(block.data.clone())?;
                let block_result = unit.execute(operation)?;
                result.extend_from_slice(&block_result);
            }
        }

        Vector::new(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataFormat;

    #[test]
    fn test_matrix_vector_computation() {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone());

        let matrix_data = vec![vec![1.0; 32]; 32];
        let vector_data = vec![1.0; 32];

        let matrix = Matrix::from_f32(&matrix_data, &converter).unwrap();
        let vector = Vector::from_f32(&vector_data, &converter).unwrap();

        accelerator.prepare_matrix(&matrix).unwrap();
        let result = accelerator.compute_matrix_vector(&vector).unwrap();

        assert_eq!(result.len(), 32);
    }
}