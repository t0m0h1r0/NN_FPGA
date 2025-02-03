use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE, DataConverter};
use crate::memory::MatrixBlock;
use crate::math::{Matrix, Vector};
use crate::compute::{ComputeCore, ComputeOperation};
use crate::instructions::{FpgaInstruction, VliwInstruction, InstructionExecutor, FpgaInstructionChannel};

pub struct FpgaAccelerator {
    compute_core: ComputeCore,
    data_converter: DataConverter,
    matrix_rows: usize,
    matrix_cols: usize,
    instruction_channel: FpgaInstructionChannel,
}

impl FpgaAccelerator {
    pub fn new(num_units: usize, data_converter: DataConverter) -> Result<Self> {
        Ok(Self {
            compute_core: ComputeCore::new(num_units)?,
            data_converter,
            matrix_rows: 0,
            matrix_cols: 0,
            instruction_channel: FpgaInstructionChannel::new()?,
        })
    }

    pub fn prepare_matrix(&mut self, matrix: &Matrix) -> Result<()> {
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();

        // 行列をブロックに分割
        let blocks = matrix.split_blocks()?;
        
        // 各ブロックを対応するユニットにロード
        for (unit_id, block) in blocks.iter().enumerate() {
            if let Some(unit) = self.compute_core.get_unit(unit_id) {
                // まずユニットのメモリに行列ブロックを配置
                unit.load_matrix(MatrixBlock::new(
                    block.data.clone(),
                    unit_id * MATRIX_SIZE,
                    0,
                )?)?;

                // ゼロ初期化命令を発行
                let vliw = VliwInstruction::from_single(FpgaInstruction::ZeroM0);
                self.instruction_channel.execute_vliw(vliw)?;

                // 行列ロード命令を発行
                let vliw = VliwInstruction::new(
                    FpgaInstruction::LoadM0,
                    FpgaInstruction::Nop,
                    FpgaInstruction::Nop,
                    FpgaInstruction::Nop,
                );
                self.instruction_channel.execute_vliw(vliw)?;
            }
        }
        Ok(())
    }

    pub fn compute_matrix_vector(&mut self, vector: &Vector) -> Result<Vector> {
        if vector.len() != self.matrix_cols {
            return Err(FpgaError::Computation("Vector size mismatch".into()));
        }

        // ベクトルをブロックに分割
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
                
                // ベクトルデータをロード
                unit.load_vector(vector_blocks[unit_id].data.clone())?;

                // VLIW命令を構築して実行
                let vliw = VliwInstruction::new(
                    FpgaInstruction::LoadV0,
                    FpgaInstruction::MatrixVectorMul,
                    FpgaInstruction::PushV0,
                    FpgaInstruction::Nop,
                );
                self.instruction_channel.execute_vliw(vliw)?;
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
        // ベクトルをブロックに分割
        let vector_blocks = vector.split(MATRIX_SIZE)?;
        let mut result = Vec::new();

        for (unit_id, block) in vector_blocks.iter().enumerate() {
            if let Some(unit) = self.compute_core.get_unit(unit_id) {
                // ベクトルデータをロード
                unit.load_vector(block.data.clone())?;

                // 対応するFPGA命令を取得
                let inst: FpgaInstruction = operation.into();
                
                // VLIW命令を構築
                let vliw = VliwInstruction::new(
                    FpgaInstruction::LoadV0,
                    inst,
                    FpgaInstruction::StoreV0,
                    FpgaInstruction::Nop,
                );
                
                // 命令を実行
                self.instruction_channel.execute_vliw(vliw)?;
                
                // 結果を取得
                let block_result = unit.execute(operation)?;
                result.extend_from_slice(&block_result);
            }
        }

        Vector::new(result)
    }
    
    pub fn pull_vector_from_memory(&mut self, unit_id: usize) -> Result<Vector> {
        // 指定されたユニットを取得
        let unit = self.compute_core.get_unit(unit_id)?;
        
        // PULL命令を発行
        let vliw = VliwInstruction::from_single(FpgaInstruction::PullV0);
        self.instruction_channel.execute_vliw(vliw)?;
        
        // 結果を取得
        match &unit.vector_cache {
            Some(data) => Vector::new(data.clone()),
            None => Err(FpgaError::Computation("No vector data in cache".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataFormat;

    #[test]
    fn test_matrix_vector_computation() -> Result<()> {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone())?;

        let matrix_data = vec![vec![1.0; 32]; 32];
        let vector_data = vec![1.0; 32];

        let matrix = Matrix::from_f32(&matrix_data, &converter)?;
        let vector = Vector::from_f32(&vector_data, &converter)?;

        accelerator.prepare_matrix(&matrix)?;
        let result = accelerator.compute_matrix_vector(&vector)?;

        assert_eq!(result.len(), 32);
        Ok(())
    }

    #[test]
    fn test_vector_operations() -> Result<()> {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone())?;

        let vector_data = vec![1.0; 16];
        let vector = Vector::from_f32(&vector_data, &converter)?;

        // ReLU演算のテスト
        let result = accelerator.compute_vector_operation(
            &vector,
            ComputeOperation::VectorReLU
        )?;

        assert_eq!(result.len(), 16);
        Ok(())
    }
}