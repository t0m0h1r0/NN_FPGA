use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE};
use crate::memory::{SharedMemory, MatrixBlock};
use crate::math::{Matrix, Vector};
use std::sync::Arc;

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
}

impl ComputeUnit {
    pub fn new(id: usize, shared_memory: Arc<SharedMemory>) -> Self {
        Self {
            id,
            matrix_cache: None,
            vector_cache: None,
            shared_memory,
        }
    }

    pub fn load_matrix(&mut self, block: MatrixBlock) -> Result<()> {
        self.matrix_cache = Some(block);
        Ok(())
    }

    pub fn load_vector(&mut self, data: Vec<FpgaValue>) -> Result<()> {
        if data.len() != MATRIX_SIZE {
            return Err(FpgaError::Computation("Invalid vector size".into()));
        }
        self.vector_cache = Some(data);
        Ok(())
    }

    pub fn execute(&mut self, op: ComputeOperation) -> Result<Vec<FpgaValue>> {
        match op {
            ComputeOperation::MatrixVectorMultiply => self.matrix_vector_multiply(),
            ComputeOperation::VectorAdd => self.vector_add(),
            ComputeOperation::VectorReLU => self.vector_relu(),
        }
    }

    fn matrix_vector_multiply(&self) -> Result<Vec<FpgaValue>> {
        let matrix = self.matrix_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Matrix not loaded".into()))?;
        let vector = self.vector_cache.as_ref()
            .ok_or_else(|| FpgaError::Computation("Vector not loaded".into()))?;

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
    pub fn new(num_units: usize) -> Self {
        let shared_memory = Arc::new(SharedMemory::new(num_units));
        let units = (0..num_units)
            .map(|id| ComputeUnit::new(id, Arc::clone(&shared_memory)))
            .collect();

        Self { units }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataConverter;
    use crate::types::DataFormat;

    #[test]
    fn test_compute_unit_matrix_multiply() {
        let shared_memory = Arc::new(SharedMemory::new(1));
        let mut unit = ComputeUnit::new(0, shared_memory);

        let converter = DataConverter::new(DataFormat::Full);
        let matrix_data = vec![vec![1.0; MATRIX_SIZE]; MATRIX_SIZE];
        let vector_data = vec![1.0; MATRIX_SIZE];

        let matrix = Matrix::from_f32(&matrix_data, &converter).unwrap();
        let vector = Vector::from_f32(&vector_data, &converter).unwrap();

        unit.load_matrix(MatrixBlock::new(matrix.data, 0, 0).unwrap()).unwrap();
        unit.load_vector(vector.data).unwrap();

        let result = unit.execute(ComputeOperation::MatrixVectorMultiply).unwrap();
        assert_eq!(result.len(), MATRIX_SIZE);
    }
}