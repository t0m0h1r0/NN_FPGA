//! FPGA演算ユニットの実装

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

    #[error("行列が未ロードです")]
    MatrixNotLoaded,
}

/// FPGAユニットの状態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitStatus {
    Available,
    Busy,
    MatrixLoaded,  // 行列がロード済みの状態を追加
    Error,
}

/// FPGAユニットの構造体
#[derive(Debug)]
pub struct ComputeUnit {
    pub id: usize,
    pub status: UnitStatus,
    v0: Vec<FpgaValue>,  // ベクトルレジスタ0
    v1: Vec<FpgaValue>,  // ベクトルレジスタ1
    m0: Vec<Vec<FpgaValue>>,  // 行列レジスタ
    matrix_loaded: bool,  // 行列がロード済みかのフラグ
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
        }
    }

    /// 行列をロード
    pub fn load_matrix(&mut self, matrix_data: Vec<Vec<FpgaValue>>) -> Result<(), UnitError> {
        if self.status != UnitStatus::Available {
            return Err(UnitError::InvalidInstruction);
        }
        self.m0 = matrix_data;
        self.matrix_loaded = true;
        self.status = UnitStatus::MatrixLoaded;
        Ok(())
    }

    /// ベクトルをロードして乗算を実行
    pub fn load_and_multiply(&mut self, vector_data: Vec<FpgaValue>) -> Result<(), UnitError> {
        if !self.matrix_loaded {
            return Err(UnitError::MatrixNotLoaded);
        }
        if self.status != UnitStatus::MatrixLoaded {
            return Err(UnitError::InvalidInstruction);
        }

        // ベクトルをロード
        self.v0 = vector_data;
        self.status = UnitStatus::Busy;

        // 行列ベクトル乗算を実行
        self.execute_matrix_vector_multiply()?;

        // 状態を行列ロード済みに戻す（次の乗算に備える）
        self.status = UnitStatus::MatrixLoaded;
        Ok(())
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

    /// VLIW命令を実行
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
                    self.matrix_loaded = false;
                },
                VliwCommand::PushV0 => {
                    shared_memory[self.id] = SharedMemoryEntry {
                        data: self.v0.clone(),
                        valid: true,
                    };
                },
                VliwCommand::PopV1 => {
                    if shared_memory[self.id].valid {
                        self.v1 = shared_memory[self.id].data.clone();
                    }
                },
                VliwCommand::PopV0 => {  // 【新規追加】
                    if shared_memory[self.id].valid {
                        self.v0 = shared_memory[self.id].data.clone();
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

    // 行列ベクトル乗算の実行
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

    // ベクトル加算の実行
    fn execute_vector_add(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let sum = self.v0[i].to_f32() + self.v1[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(sum, DataConversionType::Full);
        }
        Ok(())
    }

    // ベクトル減算の実行
    fn execute_vector_sub(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let diff = self.v0[i].to_f32() - self.v1[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(diff, DataConversionType::Full);
        }
        Ok(())
    }

    // ベクトルReLUの実行
    fn execute_vector_relu(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val.max(0.0), DataConversionType::Full);
        }
        Ok(())
    }

    // ベクトルtanhの実行
    fn execute_vector_tanh(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val.tanh(), DataConversionType::Full);
        }
        Ok(())
    }

    // ベクトル二乗の実行
    fn execute_vector_square(&mut self) -> Result<(), UnitError> {
        for i in 0..VECTOR_SIZE {
            let val = self.v0[i].to_f32();
            self.v0[i] = FpgaValue::from_f32(val * val, DataConversionType::Full);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_v0_instruction() {
        let mut unit = ComputeUnit::new(0);
        let mut shared_memory = vec![SharedMemoryEntry { data: Vec::new(), valid: false }; 1];

        // テストデータの準備
        let test_data: Vec<FpgaValue> = (0..VECTOR_SIZE)
            .map(|i| FpgaValue::from_f32(i as f32, DataConversionType::Full))
            .collect();

        // 共有メモリにデータを設定
        shared_memory[0] = SharedMemoryEntry {
            data: test_data.clone(),
            valid: true,
        };

        // PopV0命令を実行
        let pop_inst = VliwInstruction {
            op1: VliwCommand::PopV0,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        };

        // 命令実行
        assert!(unit.execute_instruction(&pop_inst, &mut shared_memory).is_ok());

        // 結果の検証
        for (i, val) in unit.get_v0().iter().enumerate() {
            assert_eq!(val.to_f32(), i as f32);
        }
    }

    #[test]
    fn test_pop_v0_with_invalid_memory() {
        let mut unit = ComputeUnit::new(0);
        let mut shared_memory = vec![SharedMemoryEntry { data: Vec::new(), valid: false }; 1];

        // PopV0命令を実行
        let pop_inst = VliwInstruction {
            op1: VliwCommand::PopV0,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        };

        // 命令実行（メモリが無効な場合）
        assert!(unit.execute_instruction(&pop_inst, &mut shared_memory).is_ok());

        // 結果の検証（変更がないことを確認）
        for val in unit.get_v0() {
            assert_eq!(val.to_f32(), 0.0);
        }
    }
}