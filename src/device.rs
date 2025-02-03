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

    // 最適化された行列準備処理
    pub fn prepare_matrix(&mut self, matrix: &Matrix) -> Result<()> {
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();

        // 行列をブロックに分割
        let blocks = matrix.split_blocks()?;
        
        // ユニットごとのブロック数を計算
        let blocks_per_unit = (blocks.len() + self.compute_core.num_units() - 1) 
            / self.compute_core.num_units();

        // 各ユニットに対して並列にブロックをロード
        for chunk_idx in 0..blocks_per_unit {
            let vliw_instructions = self.generate_parallel_load_instructions(
                &blocks,
                chunk_idx,
                self.compute_core.num_units()
            )?;

            // VLIWパケットを一括実行
            for vliw in vliw_instructions {
                self.instruction_channel.execute_vliw(vliw)?;
            }
        }

        Ok(())
    }

    // 並列ロード用VLIW命令生成
    fn generate_parallel_load_instructions(
        &self,
        blocks: &[Matrix],
        chunk_idx: usize,
        num_units: usize
    ) -> Result<Vec<VliwInstruction>> {
        let mut instructions = Vec::new();
        
        // 初期化用VLIW命令（複数ユニットを同時に初期化）
        let mut init_vliw = VliwInstruction::new(
            FpgaInstruction::ZERO_M0,
            FpgaInstruction::ZERO_M0,
            FpgaInstruction::ZERO_M0,
            FpgaInstruction::ZERO_M0
        );
        instructions.push(init_vliw);

        // ロード用VLIW命令（4ユニットずつ並列ロード）
        for unit_group in (0..num_units).step_by(4) {
            let mut load_vliw = VliwInstruction::new(
                FpgaInstruction::LoadM0,
                if unit_group + 1 < num_units { FpgaInstruction::LoadM0 } else { FpgaInstruction::Nop },
                if unit_group + 2 < num_units { FpgaInstruction::LoadM0 } else { FpgaInstruction::Nop },
                if unit_group + 3 < num_units { FpgaInstruction::LoadM0 } else { FpgaInstruction::Nop }
            );
            instructions.push(load_vliw);
        }

        Ok(instructions)
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

            // FPGA上での並列計算とリダクション実行
            self.execute_pipelined_computation(
                &vector_blocks,
                units_in_row,
                block_row
            )?;

            // 最終結果の取得（ユニット0から）
            let row_result = self.get_final_result()?;
            final_result.extend_from_slice(&row_result);
        }

        Vector::new(final_result)
    }

    // FPGA上での並列計算実行とリダクション
    fn execute_pipelined_computation(
        &mut self,
        vector_blocks: &[Vector],
        units_in_row: usize,
        block_row: usize
    ) -> Result<()> {
        // 第1フェーズ: 各ユニットでの並列計算
        for unit_id in 0..units_in_row {
            let unit = self.compute_core.get_unit(unit_id)?;
            
            // 計算とPUSH操作を1つのVLIWパケットで実行
            let vliw = VliwInstruction::new(
                FpgaInstruction::LoadV0,          // ベクトルロード
                FpgaInstruction::MatrixVectorMul, // 行列ベクトル乗算
                FpgaInstruction::PushV0,         // 結果を共有メモリへ
                FpgaInstruction::Nop
            );
            self.instruction_channel.execute_vliw(vliw)?;
        }

        // 第2フェーズ: ツリー構造でのリダクション
        let mut active_units = units_in_row;
        let mut stride = 1;

        while active_units > 1 {
            // 並列リダクションの各レベル
            for i in 0..(active_units / 2) {
                let target_unit = i;
                let source_unit = i + stride;

                // リダクション用VLIW命令パケット
                let reduction_vliw = VliwInstruction::new(
                    FpgaInstruction::PULL_V1,     // 共有メモリから第2オペランドを取得
                    FpgaInstruction::VADD_01,     // V0 += V1を実行
                    FpgaInstruction::PUSH_V0,     // 結果を共有メモリへ書き戻し
                    FpgaInstruction::Nop
                );
                self.instruction_channel.execute_vliw(reduction_vliw)?;
            }

            // 次のレベルの準備
            active_units = (active_units + 1) / 2;
            stride *= 2;
        }

        // 最終結果はユニット0の共有メモリ領域に格納されている
        Ok(())
    }

    // 最終結果の取得（ホストへの転送）
    fn get_final_result(&mut self) -> Result<Vec<FpgaValue>> {
        // ユニット0から最終結果を取得
        let vliw = VliwInstruction::from_single(FpgaInstruction::PULL_V0);
        self.instruction_channel.execute_vliw(vliw)?;
        
        let unit = self.compute_core.get_unit(0)?;
        match &unit.vector_cache {
            Some(data) => Ok(data.clone()),
            None => Err(FpgaError::Computation("No result data available".into()))
        }
    }

    // ベクトル演算の実行
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
                    FpgaInstruction::Nop
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

    pub fn push_vector_to_memory(
        &mut self,
        vector: &Vector,
        unit_id: usize
    ) -> Result<()> {
        let unit = self.compute_core.get_unit(unit_id)?;
        
        // ベクトルをロードしてPUSH
        unit.load_vector(vector.data.clone())?;
        let vliw = VliwInstruction::from_single(FpgaInstruction::PushV0);
        self.instruction_channel.execute_vliw(vliw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataFormat;

    #[test]
    fn test_parallel_matrix_computation() -> Result<()> {
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

    #[test]
    fn test_vector_operations() -> Result<()> {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone())?;

        // 基本的なベクトル演算のテスト
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

    #[test]
    fn test_shared_memory_operations() -> Result<()> {
        let converter = DataConverter::new(DataFormat::Full);
        let mut accelerator = FpgaAccelerator::new(4, converter.clone())?;

        // 共有メモリ操作のテスト
        let vector_data = vec![1.0; 16];
        let vector = Vector::from_f32(&vector_data, &converter)?;

        // ユニット1にプッシュ
        accelerator.push_vector_to_memory(&vector, 1)?;

        // ユニット1からプル
        let result = accelerator.pull_vector_from_memory(1)?;
        assert_eq!(result.len(), 16);
        Ok(())
    }
}