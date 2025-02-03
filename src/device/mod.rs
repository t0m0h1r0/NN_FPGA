use crate::types::{
    DataConversionType, FpgaValue,
    MATRIX_SIZE, VECTOR_SIZE,
};
use crate::math::{FpgaMatrix, FpgaVector, MathError};
use crate::unit::{ComputeUnit, UnitError};
use thiserror::Error;
use log::{info, debug};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("無効な行列サイズ")]
    InvalidMatrixSize,
    #[error("無効なベクトルサイズ")]
    InvalidVectorSize,
    #[error("ユニットエラー: {0}")]
    UnitError(#[from] UnitError),
    #[error("数学的エラー: {0}")]
    MathError(#[from] MathError),
}

pub struct FpgaAccelerator {
    units: Vec<ComputeUnit>,
    matrix_blocks: Vec<Vec<FpgaMatrix>>,
    matrix_rows: usize,
    matrix_cols: usize,
}

impl FpgaAccelerator {
    pub fn new(num_units: usize) -> Self {
        let units = (0..num_units).map(ComputeUnit::new).collect();
        
        Self {
            units,
            matrix_blocks: Vec::new(),
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    fn validate_matrix_size(&self, matrix: &FpgaMatrix) -> Result<(), DeviceError> {
        if matrix.rows() % MATRIX_SIZE != 0 || matrix.cols() % MATRIX_SIZE != 0 {
            return Err(DeviceError::InvalidMatrixSize);
        }
        Ok(())
    }

    pub fn prepare_matrix(&mut self, matrix: &FpgaMatrix) -> Result<(), DeviceError> {
        self.validate_matrix_size(matrix)?;

        debug!("行列準備開始: {}x{}", matrix.rows(), matrix.cols());
        
        let blocks = matrix.split_into_blocks();
        
        self.matrix_rows = matrix.rows();
        self.matrix_cols = matrix.cols();
        self.matrix_blocks = blocks;

        // 各ユニットに対応するブロックをロード
        for (unit_idx, unit) in self.units.iter_mut().enumerate() {
            if let Some(block) = self.matrix_blocks[0].get(unit_idx) {
                unit.load_matrix(block.data.clone())?;
            }
        }

        info!("行列準備完了: {}ブロック", 
              self.matrix_blocks.len() * self.matrix_blocks[0].len());
        
        Ok(())
    }

    pub fn prepare_vector(&mut self, vector: &FpgaVector) -> Result<(), DeviceError> {
        if vector.size() != self.matrix_cols {
            return Err(DeviceError::MathError(MathError::DimensionMismatch {
                matrix_size: self.matrix_rows,
                matrix_cols: self.matrix_cols,
                vector_size: vector.size(),
            }));
        }

        // 列先頭の全てのユニットに対応するベクトルブロックをロード
        for (unit_idx, block) in self.matrix_blocks[0].iter().take(self.units.len()).enumerate() {
            let start = unit_idx * MATRIX_SIZE;
            let end = start + MATRIX_SIZE;
            
            let vector_block: Vec<FpgaValue> = vector.to_numpy()[start..end]
                .iter()
                .map(|&v| FpgaValue::from_f32(v, vector.conversion_type()))
                .collect();
            
            self.units[unit_idx].load_and_multiply(&vector_block)?;
            self.units[unit_idx].set_prepared_vector(vector.to_numpy())?;
        }

        // 最初のユニットのv0を一度だけpush
        self.units[0].execute_push_v0()?;

        // 同じ列の他のユニットに共有メモリを介してコピー
        for row_blocks in &self.matrix_blocks[1..] {
            for (unit_idx, _block) in row_blocks.iter().take(self.units.len()).enumerate() {
                // 同列の他ユニットにpop
                self.units[unit_idx].execute_pop_v0()?;
            }
        }

        Ok(())
    }

    pub fn compute_matrix_vector(&mut self) -> Result<FpgaVector, DeviceError> {
        let mut final_result = vec![
            FpgaValue::from_f32(0.0, 
            self.units[0].get_prepared_vector_type()); 
            self.matrix_rows
        ];

        for (row_idx, row_blocks) in self.matrix_blocks.iter().enumerate() {
            // 先頭ユニットをベースとして初期化
            self.units[0].reset()?;

            // 並列に計算
            for (unit_idx, _block) in row_blocks.iter().take(self.units.len()).enumerate() {
                // 各ユニットで計算
                self.units[unit_idx].load_and_multiply_prepared_vector()?;

                // 最初のユニット以外の結果を先頭ユニットに集約
                if unit_idx > 0 {
                    // 他のユニットの結果をpush
                    self.units[unit_idx].execute_push_v0()?;
                    
                    // 先頭ユニットでpop & 加算
                    self.units[0].execute_pop_v1()?;
                    self.units[0].execute_vector_add()?;
                }
            }

            // 結果をコピー
            let result_block = self.units[0].get_v0();
            let result_start = row_idx * MATRIX_SIZE;
            let result_end = result_start + MATRIX_SIZE;
            final_result[result_start..result_end].copy_from_slice(result_block);
        }

        Ok(FpgaVector::from_numpy(
            &final_result.iter().map(|v| v.to_f32()).collect::<Vec<f32>>(),
            self.units[0].get_prepared_vector_type()
        )?)
    }
}

impl Default for FpgaAccelerator {
    fn default() -> Self {
        Self::new(4)  // デフォルトで4ユニット
    }
}