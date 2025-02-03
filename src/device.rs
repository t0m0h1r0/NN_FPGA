use std::sync::{Arc, Mutex};
use thiserror::Error;
use log::{info, error, debug};

use crate::types::{
    ComputationType, DataConversionType, FpgaValue,
    MATRIX_SIZE, VECTOR_SIZE,
};
use crate::math::{FpgaMatrix, FpgaVector, MathError};

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

    #[error("数学的エラー: {0}")]
    Math(#[from] MathError),
}

/// FPGAユニットの状態
#[derive(Debug, Clone, Copy, PartialEq)]
enum UnitStatus {
    Available,
    Busy,
    Error,
}

/// FPGAユニットの構造体
struct ComputeUnit {
    id: usize,
    status: UnitStatus,
    local_memory: Vec<FpgaValue>,
}

impl ComputeUnit {
    fn new(id: usize) -> Self {
        Self {
            id,
            status: UnitStatus::Available,
            local_memory: vec![FpgaValue::from_f32(0.0, DataConversionType::Full); VECTOR_SIZE],
        }
    }
}

/// FPGAアクセラレータの本体
pub struct FpgaAccelerator {
    units: Arc<Mutex<Vec<ComputeUnit>>>,
    total_units: usize,
    block_size: usize,
    prepared_matrix: Option<Vec<Vec<FpgaMatrix>>>,
    matrix_rows: usize,
    matrix_cols: usize,
}

impl FpgaAccelerator {
    /// 新しいFPGAアクセラレータインスタンスを作成
    pub fn new() -> Self {
        let total_units = 32; // VerilogのFPGAモジュールに合わせる
        let units = (0..total_units)
            .map(ComputeUnit::new)
            .collect();

        Self {
            units: Arc::new(Mutex::new(units)),
            total_units,
            block_size: MATRIX_SIZE,
            prepared_matrix: None,
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    /// 行列を準備（プリロード）
    pub fn prepare_matrix(&mut self, matrix: &FpgaMatrix) -> Result<(), DeviceError> {
        // 行列をブロックに分割
        let matrix_blocks = matrix.split_into_blocks();
        
        // 行列の元のサイズを保存
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();
        
        // 分割した行列を保持
        self.prepared_matrix = Some(matrix_blocks);
        
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

        self.compute_matrix_vector_multiply_internal(matrix_blocks, vector)
    }

    /// スカラー演算の実行
    pub fn compute_scalar(&mut self, vector: &FpgaVector, comp_type: ComputationType) -> Result<FpgaVector, DeviceError> {
        let mut units = self.units.lock().unwrap();
        let unit_id = self.find_available_unit(&units)?;
        let unit = &mut units[unit_id];
        unit.status = UnitStatus::Busy;

        // 演算の実行
        let result = match comp_type {
            ComputationType::Add => {
                vector.to_numpy().iter()
                    .map(|&x| x + 1.0)
                    .collect::<Vec<f32>>()
            },
            ComputationType::Multiply => {
                vector.to_numpy().iter()
                    .map(|&x| x * 2.0)
                    .collect::<Vec<f32>>()
            },
            ComputationType::Tanh => {
                vector.to_numpy().iter()
                    .map(|&x| x.tanh())
                    .collect::<Vec<f32>>()
            },
            ComputationType::ReLU => {
                vector.to_numpy().iter()
                    .map(|&x| x.max(0.0))
                    .collect::<Vec<f32>>()
            },
            _ => return Err(DeviceError::UnsupportedComputationType(
                format!("サポートされていない計算タイプ: {:?}", comp_type)
            )),
        };

        unit.status = UnitStatus::Available;
        Ok(FpgaVector::from_numpy(&result, vector.conversion_type())?)
    }

    /// 行列ベクトル乗算の内部実装
    fn compute_matrix_vector_multiply_internal(
        &self,
        matrix_blocks: &[Vec<FpgaMatrix>],
        input_vector: &FpgaVector,
    ) -> Result<FpgaVector, DeviceError> {
        let mut result = vec![FpgaValue::from_f32(0.0, DataConversionType::Full); self.matrix_rows];
        let mut units = self.units.lock().unwrap();

        for (block_row_idx, row_blocks) in matrix_blocks.iter().enumerate() {
            for block in row_blocks {
                // 利用可能なユニットを探す
                let unit_id = self.find_available_unit(&units)?;
                let unit = &mut units[unit_id];

                // ブロック計算の実行
                let block_result = self.execute_block_computation(unit, block, input_vector)?;

                // 結果の集約
                for (i, value) in block_result.iter().enumerate() {
                    let idx = block_row_idx * self.block_size + i;
                    if idx < result.len() {
                        result[idx] = *value;
                    }
                }

                // ユニットを解放
                unit.status = UnitStatus::Available;
            }
        }

        Ok(FpgaVector::from_numpy(
            &result.iter().map(|v| v.to_f32()).collect::<Vec<f32>>(),
            input_vector.conversion_type()
        )?)
    }

    /// 利用可能なユニットを探す
    fn find_available_unit(&self, units: &[ComputeUnit]) -> Result<usize, DeviceError> {
        units.iter()
            .position(|unit| unit.status == UnitStatus::Available)
            .ok_or(DeviceError::NoAvailableUnits)
    }

    /// ブロック計算の実行
    fn execute_block_computation(
        &self,
        unit: &mut ComputeUnit,
        block: &FpgaMatrix,
        vector: &FpgaVector,
    ) -> Result<Vec<FpgaValue>, DeviceError> {
        unit.status = UnitStatus::Busy;

        // FPGA IPで実装されている行列ベクトル乗算を模擬
        let mut result = vec![FpgaValue::from_f32(0.0, vector.conversion_type()); MATRIX_SIZE];
        
        for i in 0..MATRIX_SIZE {
            let mut sum = 0.0;
            for j in 0..MATRIX_SIZE {
                let matrix_val = block.get(i, j).to_f32();
                let vector_val = vector.get(j).to_f32();
                sum += matrix_val * vector_val;
            }
            result[i] = FpgaValue::from_f32(sum, vector.conversion_type());
        }

        Ok(result)
    }
}

impl Default for FpgaAccelerator {
    fn default() -> Self {
        Self::new()
    }
}