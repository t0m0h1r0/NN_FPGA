pub mod instruction;
pub mod memory;
pub mod unit;

use thiserror::Error;
use log::{info, error, debug};

use crate::types::{
    ComputationType, DataConversionType, FpgaValue,
    MATRIX_SIZE, VECTOR_SIZE,
};
use crate::math::{FpgaMatrix, FpgaVector, MathError};
use instruction::{VliwCommand, VliwInstruction, InstructionBuilder};
use memory::SharedMemory;
use unit::{ComputeUnit, UnitStatus, UnitError};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("FPGAユニットが利用できません")]
    NoAvailableUnits,

    #[error("メモリアクセスエラー: 最大 {max}, 要求 {attempted}")]
    MemoryAccessError {
        max: usize,
        attempted: usize,
    },

    #[error("計算タイプがサポートされていません: {0}")]
    UnsupportedComputationType(String),

    #[error("内部通信エラー: {0}")]
    CommunicationError(String),

    #[error("行列が準備されていません: {0}")]
    MatrixNotPrepared(String),

    #[error("ユニットエラー: {0}")]
    UnitError(#[from] UnitError),

    #[error("数学的エラー: {0}")]
    Math(#[from] MathError),
}

/// FPGAアクセラレータの本体
pub struct FpgaAccelerator {
    units: Vec<ComputeUnit>,
    shared_memory: SharedMemory,
    prepared_matrix: Option<Vec<Vec<FpgaMatrix>>>,
    matrix_rows: usize,
    matrix_cols: usize,
}

impl FpgaAccelerator {
    pub fn new() -> Self {
        let total_units = 32;
        let units = (0..total_units).map(ComputeUnit::new).collect();
        let shared_memory = SharedMemory::new(total_units);

        Self {
            units,
            shared_memory,
            prepared_matrix: None,
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    /// 行列を準備（プリロード）
    pub fn prepare_matrix(&mut self, matrix: &FpgaMatrix) -> Result<(), DeviceError> {
        debug!("行列の準備開始: {}x{}", matrix.rows(), matrix.cols());
        
        // 行列をブロックに分割
        let matrix_blocks = matrix.split_into_blocks();
        
        // 行列の元のサイズを保存
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();
        
        // 分割した行列を保持
        self.prepared_matrix = Some(matrix_blocks);
        
        info!("行列の準備完了: {}ブロックに分割", matrix_blocks.len());
        Ok(())
    }

    /// 準備済み行列とベクトルの乗算
    pub fn compute_with_prepared_matrix(&mut self, vector: &FpgaVector) -> Result<FpgaVector, DeviceError> {
        // 準備済み行列の確認
        let matrix_blocks = self.prepared_matrix.as_ref()
            .ok_or_else(|| DeviceError::MatrixNotPrepared("行列が準備されていません".to_string()))?;

        // ベクトルの次元チェック
        if vector.size() != self.matrix_cols {
            return Err(DeviceError::Math(MathError::DimensionMismatch {
                matrix_size: self.matrix_rows,
                matrix_cols: self.matrix_cols,
                vector_size: vector.size(),
            }));
        }

        debug!("行列ベクトル乗算開始: ベクトルサイズ {}", vector.size());
        self.compute_large_matrix_multiply(matrix_blocks, vector)
    }

    /// スカラー演算の実行
    pub fn compute_scalar(&mut self, vector: &FpgaVector, comp_type: ComputationType) -> Result<FpgaVector, DeviceError> {
        let unit_id = self.find_available_unit()?;
        let unit = &mut self.units[unit_id];
        unit.status = UnitStatus::Busy;

        // ベクトルをユニットにロード
        unit.load_v0(vector.data.clone());

        // 命令を構築
        let inst = match comp_type {
            ComputationType::Add => VliwInstruction::single(VliwCommand::VectorAdd01),
            ComputationType::Multiply => VliwInstruction::single(VliwCommand::VectorSquare),
            ComputationType::Tanh => VliwInstruction::single(VliwCommand::VectorTanh),
            ComputationType::ReLU => VliwInstruction::single(VliwCommand::VectorReLU),
            _ => return Err(DeviceError::UnsupportedComputationType(
                format!("サポートされていない計算タイプ: {:?}", comp_type)
            )),
        };

        // 命令を実行
        unit.execute_instruction(&inst, self.shared_memory.get_entries_mut())?;

        // 結果を取得
        let result = unit.get_v0().to_vec();
        
        // ユニットを解放
        unit.status = UnitStatus::Available;

        Ok(FpgaVector::from_numpy(
            &result.iter().map(|v| v.to_f32()).collect::<Vec<f32>>(),
            vector.conversion_type()
        )?)
    }

    /// 大規模行列の乗算を実行
    fn compute_large_matrix_multiply(
        &mut self,
        matrix_blocks: &[Vec<FpgaMatrix>],
        input_vector: &FpgaVector,
    ) -> Result<FpgaVector, DeviceError> {
        let num_block_rows = matrix_blocks.len();
        debug!("大規模行列乗算開始: {}ブロック行", num_block_rows);
        
        // 結果を格納するベクトル
        let mut final_result = vec![FpgaValue::from_f32(0.0, DataConversionType::Full); num_block_rows * MATRIX_SIZE];

        // 各ブロック行に対して処理を実行
        for block_row_idx in 0..num_block_rows {
            let row_blocks = &matrix_blocks[block_row_idx];
            debug!("ブロック行 {}/{} の処理開始", block_row_idx + 1, num_block_rows);
            
            // 利用可能なユニットを割り当て
            let mut unit_assignments = Vec::new();
            for block_idx in 0..row_blocks.len() {
                let unit_id = self.find_available_unit()?;
                unit_assignments.push((block_idx, unit_id));
                
                // ユニットに行列ブロックをロード
                let unit = &mut self.units[unit_id];
                unit.load_m0(row_blocks[block_idx].data.clone());
                
                // 入力ベクトルの対応部分をロード
                let vector_start = block_idx * MATRIX_SIZE;
                let vector_end = vector_start + MATRIX_SIZE;
                unit.load_v0(input_vector.data[vector_start..vector_end].to_vec());
            }

            debug!("{}個のユニットに割り当て完了", unit_assignments.len());

            // 並列に行列ベクトル乗算を実行
            for &(_, unit_id) in &unit_assignments {
                let unit = &mut self.units[unit_id];
                let inst = InstructionBuilder::new()
                    .add_op(VliwCommand::MatrixVectorMultiply)
                    .build();
                
                unit.execute_instruction(&inst, self.shared_memory.get_entries_mut())?;
            }

            debug!("並列乗算完了、結果の集約開始");
            
            // 各ユニットの結果を共有メモリを使って集約
            for (i, &(_, unit_id)) in unit_assignments.iter().enumerate() {
                let unit = &mut self.units[unit_id];
                
                // 結果をPUSH
                let push_inst = InstructionBuilder::new()
                    .add_op(VliwCommand::PushV0)
                    .build();
                unit.execute_instruction(&push_inst, self.shared_memory.get_entries_mut())?;

                if i > 0 {
                    // 最初のユニット以外は加算が必要
                    let first_unit = &mut self.units[unit_assignments[0].1];
                    
                    // 共有メモリから結果をPOP
                    let pop_inst = InstructionBuilder::new()
                        .add_op(VliwCommand::PopV1)
                        .build();
                    first_unit.execute_instruction(&pop_inst, self.shared_memory.get_entries_mut())?;

                    // 加算実行
                    let add_inst = InstructionBuilder::new()
                        .add_op(VliwCommand::VectorAdd01)
                        .build();
                    first_unit.execute_instruction(&add_inst, self.shared_memory.get_entries_mut())?;
                }
            }

            // 最終結果を取得
            let first_unit = &self.units[unit_assignments[0].1];
            let result_start = block_row_idx * MATRIX_SIZE;
            let result_end = result_start + MATRIX_SIZE;
            final_result[result_start..result_end].copy_from_slice(first_unit.get_v0());

            // ユニットを解放
            for &(_, unit_id) in &unit_assignments {
                self.units[unit_id].status = UnitStatus::Available;
            }

            debug!("ブロック行 {} の処理完了", block_row_idx + 1);
        }

        info!("大規模行列乗算完了");
        Ok(FpgaVector::from_numpy(
            &final_result.iter().map(|v| v.to_f32()).collect::<Vec<f32>>(),
            input_vector.conversion_type()
        )?)
    }

    /// 利用可能なユニットを探す
    fn find_available_unit(&mut self) -> Result<usize, DeviceError> {
        self.units
            .iter()
            .position(|unit| unit.status == UnitStatus::Available)
            .ok_or(DeviceError::NoAvailableUnits)
    }
}

impl Default for FpgaAccelerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiplication() {
        let mut accelerator = FpgaAccelerator::new();
        
        // 32x32のテスト行列を作成
        let matrix_data: Vec<Vec<f32>> = (0..32)
            .map(|i| (0..32).map(|j| (i * j) as f32).collect())
            .collect();
        
        let matrix = FpgaMatrix::from_numpy(&matrix_data, DataConversionType::Full).unwrap();
        accelerator.prepare_matrix(&matrix).unwrap();

        // テストベクトル
        let vector_data: Vec<f32> = (0..32).map(|x| x as f32).collect();
        let vector = FpgaVector::from_numpy(&vector_data, DataConversionType::Full).unwrap();

        // 乗算を実行
        let result = accelerator.compute_with_prepared_matrix(&vector).unwrap();

        // NumPyスタイルの検証用の乗算を実行
        let mut expected = vec![0.0; 32];
        for i in 0..32 {
            for j in 0..32 {
                expected[i] += matrix_data[i][j] * vector_data[j];
            }
        }

        // 結果を比較
        let result_data = result.data.iter().map(|v| v.to_f32()).collect::<Vec<f32>>();
        for (a, b) in result_data.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_scalar_operations() {
        let mut accelerator = FpgaAccelerator::new();
        
        // テストベクトル
        let vector_data = vec![1.0, -2.0, 0.0, 3.0];
        let vector = FpgaVector::from_numpy(&vector_data, DataConversionType::Full).unwrap();

        // ReLU
        let relu_result = accelerator.compute_scalar(&vector, ComputationType::ReLU).unwrap();
        let relu_data: Vec<f32> = relu_result.data.iter().map(|v| v.to_f32()).collect();
        assert_eq!(relu_data, vec![1.0, 0.0, 0.0, 3.0]);

        // Tanh
        let tanh_result = accelerator.compute_scalar(&vector, ComputationType::Tanh).unwrap();
        let tanh_data: Vec<f32> = tanh_result.data.iter().map(|v| v.to_f32()).collect();
        for (a, b) in tanh_data.iter().zip(vector_data.iter()) {
            assert!((a - b.tanh()).abs() < 1e-5);
        }
    }
}