use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE};
use crate::memory::{SharedMemory, MatrixBlock};
use crate::math::{Matrix, Vector};
use crate::instructions::{FpgaInstruction, VliwInstruction, InstructionExecutor, FpgaInstructionChannel};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum ComputeOperation {
    MatrixVectorMultiply,
    VectorAdd,
    VectorReLU,
}

pub struct ComputeUnit {
    id: usize,
    matrix_cache: Option<MatrixBlock>,
    vector_cache: Option<Vec<FpgaValue>>,
    shared_memory: Arc<SharedMemory>,
    instruction_channel: FpgaInstructionChannel,
}

impl ComputeUnit {
    pub fn new(id: usize, shared_memory: Arc<SharedMemory>) -> Result<Self> {
        Ok(Self {
            id,
            matrix_cache: None,
            vector_cache: None,
            shared_memory,
            instruction_channel: FpgaInstructionChannel::new()?,
        })
    }

    pub fn load_matrix(&mut self, block: MatrixBlock) -> Result<()> {
        // 行列データをキャッシュ
        self.matrix_cache = Some(block);
        
        // FPGAに行列ロード命令を発行
        let vliw = VliwInstruction::from_single(FpgaInstruction::LoadM0);
        self.instruction_channel.execute_vliw(vliw)
    }

    pub fn load_vector(&mut self, data: Vec<FpgaValue>) -> Result<()> {
        if data.len() != MATRIX_SIZE {
            return Err(FpgaError::Computation("Invalid vector size".into()));
        }
        
        // ベクトルデータをキャッシュ
        self.vector_cache = Some(data);
        
        // FPGAにベクトルロード命令を発行
        let vliw = VliwInstruction::from_single(FpgaInstruction::LoadV0);
        self.instruction_channel.execute_vliw(vliw)
    }

    pub fn execute(&mut self, op: ComputeOperation) -> Result<Vec<FpgaValue>> {
        let inst: FpgaInstruction = op.into();
        let vliw = VliwInstruction::from_single(inst);
        self.instruction_channel.execute_vliw(vliw)?;

        match op {
            ComputeOperation::MatrixVectorMultiply => self.matrix_vector_multiply(),
            ComputeOperation::VectorAdd => self.vector_add(),
            ComputeOperation::VectorReLU => self.vector_relu(),
        }
    }

    fn matrix_vector_multiply(&self) -> Result<Vec<FpgaValue>> {
        // 行列データとベクトルデータの存在確認
        let matrix = self.matrix_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Matrix not loaded".into()))?;
        let vector = self.vector_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Vector not loaded".into()))?;

        // 結果を取得（実際のハードウェアでは非同期で結果が返される）
        let result = Matrix::new(matrix.get_data().to_vec())?
            .multiply_vector(&Vector::new(vector.clone())?)?;

        Ok(result.data)
    }

    fn vector_add(&self) -> Result<Vec<FpgaValue>> {
        let v1 = self.vector_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Vector not loaded".into()))?;
        let v2 = self.shared_memory.read_block(self.id)?;

        Vector::new(v1.clone())?.add(&Vector::new(v2)?).map(|v| v.data)
    }

    fn vector_relu(&self) -> Result<Vec<FpgaValue>> {
        let vector = self.vector_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Vector not loaded".into()))?;
        
        Vector::new(vector.clone())?.relu().map(|v| v.data)
    }
}

pub struct ComputeCore {
    units: Vec<ComputeUnit>,
}

impl ComputeCore {
    pub fn new(num_units: usize) -> Result<Self> {
        let shared_memory = Arc::new(SharedMemory::new(num_units));
        let units = (0..num_units)
            .map(|id| ComputeUnit::new(id, Arc::clone(&shared_memory)))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { units })
    }

    pub fn get_unit(&mut self, id: usize) -> Result<&mut ComputeUnit> {
        self.units.get_mut(id)
            .ok_or_else(|| FpgaError::Computation("Invalid unit ID".into()))
    }

    pub fn execute_parallel(&mut self, op: ComputeOperation) -> Result<Vec<Vec<FpgaValue>>> {
        self.units.iter_mut()
            .map(|unit| unit.execute(op))
            .collect()
    }
}