//! FPGAアクセラレータのメイン実装

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

/// 行列ブロックの割り当て情報
#[derive(Debug)]
struct MatrixBlockAssignment {
    unit_id: usize,
    block_index: usize,
}

/// FPGAアクセラレータの本体
pub struct FpgaAccelerator {
    units: Vec<ComputeUnit>,
    shared_memory: SharedMemory,
    matrix_assignments: Vec<Vec<MatrixBlockAssignment>>, // 行列ブロックの割り当て情報を保持
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
            matrix_assignments: Vec::new(),
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    /// 行列を準備（プリロード）
    /// この段階で各ブロックをユニットに割り当て、ロードする
    pub fn prepare_matrix(&mut self, matrix: &FpgaMatrix) -> Result<(), DeviceError> {
        debug!("行列の準備開始: {}x{}", matrix.rows(), matrix.cols());
        
        // 行列をブロックに分割
        let matrix_blocks = matrix.split_into_blocks();
        
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();
        
        // 既存の割り当てをクリア
        self.clear_matrix_assignments()?;
        
        let mut assignments = Vec::new();
        
        // 各ブロック行に対して処理
        for row_blocks in matrix_blocks {
            let mut row_assignments = Vec::new();
            
            // 各ブロックに対してユニットを割り当て
            for (block_idx, block) in row_blocks.iter().enumerate() {
                let unit_id = self.find_available_unit()?;
                
                // ユニットに行列ブロックをロード
                self.units[unit_id].load_matrix(block.data.clone())?;
                
                row_assignments.push(MatrixBlockAssignment {
                    unit_id,
                    block_index: block_idx,
                });
            }
            
            assignments.push(row_assignments);
        }
        
        self.matrix_assignments = assignments;
        info!("行列の準備完了: {}ブロックを{}ユニットに割り当て",
              matrix_blocks.len() * matrix_blocks[0].len(),
              self.matrix_assignments.iter().flatten().count());
        
        Ok(())
    }

    /// 準備済み行列とベクトルの乗算
    pub fn compute_with_prepared_matrix(&mut self, vector: &FpgaVector) -> Result<FpgaVector, DeviceError> {
        if self.matrix_assignments.is_empty() {
            return Err(DeviceError::MatrixNotPrepared("行列が準備されていません".to_string()));
        }

        // ベクトルの次元チェック
        if vector.size() != self.matrix_cols {
            return Err(DeviceError::Math(MathError::DimensionMismatch {
                matrix_size: self.matrix_rows,
                matrix_cols: self.matrix_cols,
                vector_size: vector.size(),
            }));
        }

        debug!("行列ベクトル乗算開始: ベクトルサイズ {}", vector.size());
        
        let mut final_result = vec![FpgaValue::from_f32(0.0, vector.conversion_type()); self.matrix_rows];

        // 各ブロック行に対して処理を実行
        for (block_row_idx, row_assignments) in self.matrix_assignments.iter().enumerate() {
            debug!("ブロック行 {}/{} の処理開始", block_row_idx + 1, self.matrix_assignments.len());
            
            // 各ユニットで並列に乗算を実行
            for assignment in row_assignments {
                // 入力ベクトルの対応部分を取得
                let vector_start = assignment.block_index * MATRIX_SIZE;
                let vector_end = vector_start + MATRIX_SIZE;
                let vector_block = vector.data[vector_start..vector_end].to_vec();
                
                // 乗算を実行
                let unit = &mut self.units[assignment.unit_id];
                unit.load_and_multiply(vector_block)?;
            }

            // 結果の集約
            let mut row_result = self.accumulate_results(row_assignments)?;
            
            // 最終結果に格納
            let result_start = block_row_idx * MATRIX_SIZE;
            let result_end = result_start + MATRIX_SIZE;
            final_result[result_start..result_end].copy_from_slice(&row_result);
        }

        Ok(FpgaVector::from_numpy(
            &final_result.iter().map(|v| v.to_f32()).collect::<Vec<f32>>(),
            vector.conversion_type()
        )?)
    }

    /// スカラー演算の実行
    pub fn compute_scalar(&mut self, vector: &FpgaVector, comp_type: ComputationType) -> Result<FpgaVector, DeviceError> {
        let unit_id = self.find_available_unit()?;
        let unit = &mut self.units[unit_id];
        unit.status = UnitStatus::Busy;

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

        // ベクトルをロードして命令を実行
        unit.load_and_multiply(vector.data.clone())?;
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

    /// 結果の集約処理
    fn accumulate_results(&mut self, assignments: &[MatrixBlockAssignment]) -> Result<Vec<FpgaValue>, DeviceError> {
        // 最初のユニットの結果をベースとする
        let first_unit = &mut self.units[assignments[0].unit_id];
        let mut accumulated = first_unit.get_v0().to_vec();

        // 2番目以降のユニットの結果を加算
        for assignment in &assignments[1..] {
            let unit = &mut self.units[assignment.unit_id];
            
            // 結果をPUSH
            let push_inst = VliwInstruction::single(VliwCommand::PushV0);
            unit.execute_instruction(&push_inst, self.shared_memory.get_entries_mut())?;

            // 結果をPOPして加算
            let pop_inst = VliwInstruction::single(VliwCommand::PopV1);
            first_unit.execute_instruction(&pop_inst, self.shared_memory.get_entries_mut())?;

            let add_inst = VliwInstruction::single(VliwCommand::VectorAdd01);
            first_unit.execute_instruction(&add_inst, self.shared_memory.get_entries_mut())?;
        }

        Ok(first_unit.get_v0().to_vec())
    }

    /// 既存の行列割り当てをクリア
    fn clear_matrix_assignments(&mut self) -> Result<(), DeviceError> {
        for assignments in &self.matrix_assignments {
            for assignment in assignments {
                let unit = &mut self.units[assignment.unit_id];
                unit.status = UnitStatus::Available;
            }
        }
        self.matrix_assignments.clear();
        Ok(())
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
    fn test_matrix_reuse() {
        let mut accelerator = FpgaAccelerator::new();
        
        // 32x32のテスト行列を作成
        let matrix_data: Vec<Vec<f32>> = (0..32)
            .map(|i| (0..32).map(|j| (i * j) as f32).collect())
            .collect();
        
        let matrix = FpgaMatrix::from_numpy(&matrix_data, DataConversionType::Full).unwrap();
        accelerator.prepare_matrix(&matrix).unwrap();

        // 複数のベクトルで乗算をテスト
        for k in 0..5 {
            let vector_data: Vec<f32> = (0..32).map(|x| (x + k) as f32).collect();
            let vector = FpgaVector::from_numpy(&vector_data, DataConversionType::Full).unwrap();

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
    }
}