use crate::types::{FpgaValue, DataConversionType, MATRIX_SIZE, VECTOR_SIZE};
use crate::instruction::{VliwCommand, VliwInstruction};
use super::memory::SharedMemoryEntry;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UnitError {
    #[error("無効な命令シーケンス")]
    InvalidInstruction,
    
    #[error("メモリアクセスエラー")]
    MemoryAccessError,

    #[error("行列が未ロードです")]
    MatrixNotLoaded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitStatus {
    Available,
    Busy,
    MatrixLoaded,
    Error,
}

#[derive(Debug)]
pub struct ComputeUnit {
    pub id: usize,
    pub status: UnitStatus,
    v0: Vec<FpgaValue>,
    v1: Vec<FpgaValue>,
    m0: Vec<Vec<FpgaValue>>,
    matrix_loaded: bool,
    prepared_vector: Option<Vec<f32>>,
    prepared_vector_type: Option<DataConversionType>,
}

impl ComputeUnit {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            status: UnitStatus::Available,
            v0: vec![FpgaValue::from_f32(0.0, DataConversionType::Full); VECTOR_SIZE],
            v1: vec![FpgaValue::from_f32(0.0, DataConversionType::Full); VECTOR_SIZE],
            m0: vec![vec![FpgaValue::from_f32(0.0, DataConversionType::Full); MATRIX_SIZE]; MATRIX_SIZE],
            matrix_loaded: false,
            prepared_vector: None,
            prepared_vector_type: None,
        }
    }

    pub fn load_matrix(&mut self, matrix_data: Vec<Vec<FpgaValue>>) -> Result<(), UnitError> {
        if self.status != UnitStatus::Available {
            return Err(UnitError::InvalidInstruction);
        }
        self.m0 = matrix_data;
        self.matrix_loaded = true;
        self.status = UnitStatus::MatrixLoaded;
        Ok(())
    }

    pub fn load_and_multiply(&mut self, vector_data: &[FpgaValue]) -> Result<(), UnitError> {
        if !self.matrix_loaded {
            return Err(UnitError::MatrixNotLoaded);
        }
        if self.status != UnitStatus::MatrixLoaded {
            return Err(UnitError::InvalidInstruction);
        }

        // ベクトルをロード
        self.v0 = vector_data.to_vec();
        self.status = UnitStatus::Busy;

        // 行列ベクトル乗算を実行
        self.execute_matrix_vector_multiply()?;

        // 状態を行列ロード済みに戻す
        self.status = UnitStatus::MatrixLoaded;
        Ok(())
    }

    pub fn set_prepared_vector(&mut self, vector_data: Vec<f32>) -> Result<(), UnitError> {
        self.prepared_vector = Some(vector_data);
        self.prepared_vector_type = Some(DataConversionType::Full); // デフォルト
        Ok(())
    }

    pub fn get_prepared_vector_type(&self) -> DataConversionType {
        self.prepared_vector_type.unwrap_or(DataConversionType::Full)
    }

    pub fn load_and_multiply_prepared_vector(&mut self) -> Result<(), UnitError> {
        if self.prepared_vector.is_none() {
            return Err(UnitError::InvalidInstruction);
        }

        self.v0 = self.prepared_vector.as_ref().unwrap()
            .iter()
            .map(|&v| FpgaValue::from_f32(v, self.get_prepared_vector_type()))
            .collect();
        
        self.execute_matrix_vector_multiply()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), UnitError> {
        self.v0.fill(FpgaValue::from_f32(0.0, DataConversionType::Full));
        self.v1.fill(FpgaValue::from_f32(0.0, DataConversionType::Full));
        Ok(())
    }

    pub fn execute_push_v0(&mut self) -> Result<(), UnitError> {
        // 共有メモリへのpush（実際の実装は外部で行う）
        Ok(())
    }

    pub fn execute_pop_v0(&mut self) -> Result<(), UnitError> {
        // 共有メモリからのv0へのpop（実際の実装は外部で行う）
        Ok(())
    }

    pub fn execute_pop_v1(&mut self) -> Result<(), UnitError> {
        // 共有メモリからのv1へのpop（実際の実装は外部で行う）
        Ok(())
    }

    pub fn execute_vector_add(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let sum = self.v0[i].to_f32() + self.v1[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(sum, DataConversionType::Full);
        }
        Ok(())
    }

    pub fn get_v0(&self) -> &[FpgaValue] {
        &self.v0
    }

    fn execute_matrix_vector_multiply(&mut self) -> Result<(), UnitError> {
        if !self.matrix_loaded {
            return Err(UnitError::MatrixNotLoaded);
        }

        let mut result = vec![0.0_f32; MATRIX_SIZE];
        for i in 0..MATRIX_SIZE {
            for j in 0..MATRIX_SIZE {
                let m_val = self.m0[i][j].to_f32();
                let v_val = self.v0[j].to_f32();
                result[i] += m_val * v_val;
            }
        }
        
        self.v0 = result.iter()
            .map(|&x| FpgaValue::from_f32(x, DataConversionType::Full))
            .collect();
        
        Ok(())
    }
}