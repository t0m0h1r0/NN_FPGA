use crate::types::{FpgaValue, DataConversionType, MATRIX_SIZE, VECTOR_SIZE};
use super::instruction::{VliwCommand, VliwInstruction};
use super::memory::SharedMemoryEntry;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UnitError {
    #[error("無効な命令シーケンス")]
    InvalidInstruction,
    
    #[error("メモリアクセスエラー")]
    MemoryAccessError,
}

/// FPGAユニットの状態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitStatus {
    Available,
    Busy,
    Error,
}

/// FPGAユニットの構造体
pub struct ComputeUnit {
    pub id: usize,
    pub status: UnitStatus,
    v0: Vec<FpgaValue>,  // ベクトルレジスタ0
    v1: Vec<FpgaValue>,  // ベクトルレジスタ1
    m0: Vec<Vec<FpgaValue>>,  // 行列レジスタ
}

impl ComputeUnit {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            status: UnitStatus::Available,
            v0: vec![FpgaValue::from_f32(0.0, DataConversionType::Full); VECTOR_SIZE],
            v1: vec![FpgaValue::from_f32(0.0, DataConversionType::Full); VECTOR_SIZE],
            m0: vec![vec![FpgaValue::from_f32(0.0, DataConversionType::Full); MATRIX_SIZE]; MATRIX_SIZE],
        }
    }

    /// レジスタの内容を取得
    pub fn get_v0(&self) -> &[FpgaValue] {
        &self.v0
    }

    pub fn get_v1(&self) -> &[FpgaValue] {
        &self.v1
    }

    pub fn get_m0(&self) -> &[Vec<FpgaValue>] {
        &self.m0
    }

    /// レジスタに直接データをロード
    pub fn load_v0(&mut self, data: Vec<FpgaValue>) {
        self.v0 = data;
    }

    pub fn load_v1(&mut self, data: Vec<FpgaValue>) {
        self.v1 = data;
    }

    pub fn load_m0(&mut self, data: Vec<Vec<FpgaValue>>) {
        self.m0 = data;
    }

    // 命令の実行
    pub fn execute_instruction(&mut self, inst: &VliwInstruction, shared_memory: &mut [SharedMemoryEntry]) -> Result<(), UnitError> {
        // 4段のVLIW命令を順番に実行
        for op in [inst.op1, inst.op2, inst.op3, inst.op4] {
            match op {
                VliwCommand::Nop => {},
                VliwCommand::LoadV0 => {},  // 外部からのロード命令は別途実装
                VliwCommand::LoadV1 => {},
                VliwCommand::LoadM0 => {},
                VliwCommand::StoreV0 => {},
                VliwCommand::StoreV1 => {},
                VliwCommand::StoreM0 => {},
                VliwCommand::ZeroV0 => {
                    self.v0.fill(FpgaValue::from_f32(0.0, DataConversionType::Full));
                },
                VliwCommand::ZeroV1 => {
                    self.v1.fill(FpgaValue::from_f32(0.0, DataConversionType::Full));
                },
                VliwCommand::ZeroM0 => {
                    for row in self.m0.iter_mut() {
                        row.fill(FpgaValue::from_f32(0.0, DataConversionType::Full));
                    }
                },
                VliwCommand::PushV0 => {
                    // V0の内容を共有メモリに書き込み
                    shared_memory[self.id] = SharedMemoryEntry {
                        data: self.v0.clone(),
                        valid: true,
                    };
                },
                VliwCommand::PopV1 => {
                    // 共有メモリからV1にデータを読み込み
                    if shared_memory[self.id].valid {
                        self.v1 = shared_memory[self.id].data.clone();
                    }
                },
                VliwCommand::MatrixVectorMultiply => {
                    self.execute_matrix_vector_multiply()?;
                },
                VliwCommand::VectorAdd01 => {
                    self.execute_vector_add()?;
                },
                VliwCommand::VectorSub01 => {
                    self.execute_vector_sub()?;
                },
                VliwCommand::VectorReLU => {
                    self.execute_vector_relu()?;
                },
                VliwCommand::VectorTanh => {
                    self.execute_vector_tanh()?;
                },
                VliwCommand::VectorSquare => {
                    self.execute_vector_square()?;
                },
            }
        }
        Ok(())
    }

    // 各演算の実装
    fn execute_matrix_vector_multiply(&mut self) -> Result<(), UnitError> {
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

    fn execute_vector_add(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let sum = self.v0[i].to_f32() + self.v1[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(sum, DataConversionType::Full);
        }
        Ok(())
    }

    fn execute_vector_sub(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let diff = self.v0[i].to_f32() - self.v1[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(diff, DataConversionType::Full);
        }
        Ok(())
    }

    fn execute_vector_relu(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val.max(0.0), DataConversionType::Full);
        }
        Ok(())
    }

    fn execute_vector_tanh(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val.tanh(), DataConversionType::Full);
        }
        Ok(())
    }

    fn execute_vector_square(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val * val, DataConversionType::Full);
        }
        Ok(())
    }
}