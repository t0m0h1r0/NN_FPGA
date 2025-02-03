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

    // ブロードキャストベースの最適化された行列準備処理
    pub fn prepare_matrix(&mut self, matrix: &Matrix) -> Result<()> {
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();

        // 行列をブロックに分割
        let blocks = matrix.split_blocks()?;
        let num_units = self.compute_core.num_units();
        
        // 各ブロックグループについて処理
        for block_group_idx in 0..(blocks.len() + num_units - 1) / num_units {
            let start_idx = block_group_idx * num_units;
            let end_idx = std::cmp::min(start_idx + num_units, blocks.len());
            
            // このグループの各ブロックを共有メモリを介して配布
            for block_idx in start_idx..end_idx {
                self.broadcast_matrix_block(&blocks[block_idx], block_idx)?;
            }
        }

        Ok(())
    }

    // ブロックの共有メモリを介したブロードキャスト
    fn broadcast_matrix_block(&mut self, block: &Matrix, block_idx: usize) -> Result<()> {
        // Step 1: ブロックをマスターユニット(0)の共有メモリ領域にロード
        let master_unit = self.compute_core.get_unit(0)?;
        let matrix_block = MatrixBlock::new(
            block.data.clone(),
            block_idx * MATRIX_SIZE,
            0,
        )?;

        // マスターユニットにブロックをロード
        let load_vliw = VliwInstruction::new(
            FpgaInstruction::LoadM0,    // 行列ブロックをロード
            FpgaInstruction::PushM0,    // 共有メモリに書き込み
            FpgaInstruction::Nop,
            FpgaInstruction::Nop
        );
        self.instruction_channel.execute_vliw(load_vliw)?;

        // Step 2: 各ユニットが共有メモリから必要なブロックを取得
        let pull_vliw = VliwInstruction::new(
            FpgaInstruction::ZERO_M0,   // まず初期化
            FpgaInstruction::PULL_M0,   // 共有メモリからブロックを取得
            FpgaInstruction::Nop,
            FpgaInstruction::Nop
        );

        // 並列にブロックを取得（4ユニットずつ）
        for unit_group in (0..self.compute_core.num_units()).step_by(4) {
            let mut group_vliw = pull_vliw.clone();
            
            // グループ内の各ユニットに対してPULL命令を設定
            for i in 0..4 {
                if unit_group + i < self.compute_core.num_units() {
                    let unit = self.compute_core.get_unit(unit_group + i)?;
                    self.instruction_channel.execute_vliw(group_vliw)?;
                }
            }
        }

        Ok(())
    }

    // 最適化された行列ベクトル乗算
    pub fn compute_matrix_vector(&mut self, vector: &Vector) -> Result<Vector> {
        if vector.len() != self.matrix_cols {
            return Err(FpgaError::Computation("Vector size mismatch".into()));
        }

        // ベクトルをブロックに分割
        let vector_blocks = vector.split(MATRIX_SIZE)?;
        let mut final_result = Vec::new();

        // 行ブロックごとの処理
        for block_row in 0..(self.matrix_rows / MATRIX_SIZE) {
            let units_in_row = std::cmp::min(
                vector_blocks.len(),
                self.compute_core.num_units()
            );

            // ベクトルブロックの配布と計算（ブロードキャスト）
            self.broadcast_and_compute(
                &vector_blocks,
                units_in_row,
                block_row
            )?;

            // 結果の収集（ツリー状リダクション）
            let row_result = self.get_final_result()?;
            final_result.extend_from_slice(&row_result);
        }

        Vector::new(final_result)
    }

    // ベクトルブロックの配布と計算
    fn broadcast_and_compute(
        &mut self,
        vector_blocks: &[Vector],
        units_in_row: usize,
        block_row: usize
    ) -> Result<()> {
        // Step 1: ベクトルをマスターユニットの共有メモリ領域にブロードキャスト
        let master_vliw = VliwInstruction::new(
            FpgaInstruction::LoadV0,   // ベクトルをロード
            FpgaInstruction::PushV0,   // 共有メモリに書き込み
            FpgaInstruction::Nop,
            FpgaInstruction::Nop
        );
        self.instruction_channel.execute_vliw(master_vliw)?;

        // Step 2: 各ユニットが共有メモリからベクトルを取得し計算
        for unit_group in (0..units_in_row).step_by(4) {
            let compute_vliw = VliwInstruction::new(
                FpgaInstruction::PULL_V0,         // 共有メモリからベクトル取得
                FpgaInstruction::MatrixVectorMul, // 行列ベクトル乗算実行
                FpgaInstruction::PushV0,         // 結果を共有メモリに書き戻し
                FpgaInstruction::Nop
            );

            // グループ内の各ユニットで並列実行
            for i in 0..4 {
                if unit_group + i < units_in_row {
                    self.instruction_channel.execute_vliw(compute_vliw)?;
                }
            }
        }

        // Step 3: ツリー構造でのリダクション
        let mut active_units = units_in_row;
        let mut stride = 1;

        while active_units > 1 {
            for i in 0..(active_units / 2) {
                let reduction_vliw = VliwInstruction::new(
                    FpgaInstruction::PULL_V1,     // 共有メモリから第2オペランド取得
                    FpgaInstruction::VADD_01,     // V0 += V1実行
                    FpgaInstruction::PushV0,      // 結果を共有メモリに書き戻し
                    FpgaInstruction::Nop
                );
                self.instruction_channel.execute_vliw(reduction_vliw)?;
            }

            active_units = (active_units + 1) / 2;
            stride *= 2;
        }

        Ok(())
    }

    // 最終結果の取得
    fn get_final_result(&mut self) -> Result<Vec<FpgaValue>> {
        let vliw = VliwInstruction::from_single(FpgaInstruction::PULL_V0);
        self.instruction_channel.execute_vliw(vliw)?;
        
        let unit = self.compute_core.get_unit(0)?;
        match &unit.vector_cache {
            Some(data) => Ok(data.clone()),
            None => Err(FpgaError::Computation("No result data available".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataFormat;

    #[test]
    fn test_broadcast_matrix_computation() -> Result<()> {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone())?;

        // 大きな行列でのテスト（64x64）
        let matrix_data = vec![vec![1.0; 64]; 64];
        let vector_data = vec![1.0; 64];

        let matrix = Matrix::from_f32(&matrix_data, &converter)?;
        let vector = Vector::from_f32(&vector_data, &converter)?;

        accelerator.prepare_matrix(&matrix)?;
        let result = accelerator.compute_matrix_vector(&vector)?;

        assert_eq!(result.len(), 64);
        Ok(())
    }
}