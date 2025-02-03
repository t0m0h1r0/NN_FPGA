use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE, VECTOR_SIZE};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct MemoryBlock {
    data: Vec<FpgaValue>,
    block_id: usize,
    is_valid: bool,
}

impl MemoryBlock {
    pub fn new(block_id: usize) -> Self {
        Self {
            data: vec![FpgaValue::Float(0.0); VECTOR_SIZE],
            block_id,
            is_valid: false,
        }
    }

    pub fn write(&mut self, data: Vec<FpgaValue>) -> Result<()> {
        if data.len() != VECTOR_SIZE {
            return Err(FpgaError::Memory(format!(
                "Invalid vector size: expected {}, got {}",
                VECTOR_SIZE,
                data.len()
            )));
        }
        self.data = data;
        self.is_valid = true;
        Ok(())
    }

    pub fn read(&self) -> Result<&[FpgaValue]> {
        if !self.is_valid {
            return Err(FpgaError::Memory("Block not initialized".into()));
        }
        Ok(&self.data)
    }
}

pub struct SharedMemory {
    blocks: Vec<Mutex<MemoryBlock>>,
}

impl SharedMemory {
    pub fn new(num_blocks: usize) -> Self {
        let blocks = (0..num_blocks)
            .map(|id| Mutex::new(MemoryBlock::new(id)))
            .collect();
        Self { blocks }
    }

    pub fn write_block(&self, block_id: usize, data: Vec<FpgaValue>) -> Result<()> {
        self.blocks
            .get(block_id)
            .ok_or_else(|| FpgaError::Memory("Invalid block ID".into()))?
            .lock()
            .map_err(|_| FpgaError::Memory("Lock acquisition failed".into()))?
            .write(data)
    }

    pub fn read_block(&self, block_id: usize) -> Result<Vec<FpgaValue>> {
        let block = self.blocks
            .get(block_id)
            .ok_or_else(|| FpgaError::Memory("Invalid block ID".into()))?
            .lock()
            .map_err(|_| FpgaError::Memory("Lock acquisition failed".into()))?;
        Ok(block.read()?.to_vec())
    }
}

#[derive(Debug)]
pub struct MatrixBlock {
    data: Vec<Vec<FpgaValue>>,
    row_offset: usize,
    col_offset: usize,
}

impl MatrixBlock {
    pub fn new(data: Vec<Vec<FpgaValue>>, row_offset: usize, col_offset: usize) -> Result<Self> {
        if data.len() != MATRIX_SIZE || data.iter().any(|row| row.len() != MATRIX_SIZE) {
            return Err(FpgaError::Memory("Invalid matrix block size".into()));
        }
        Ok(Self {
            data,
            row_offset,
            col_offset,
        })
    }

    pub fn get_data(&self) -> &[Vec<FpgaValue>] {
        &self.data
    }

    pub fn get_offsets(&self) -> (usize, usize) {
        (self.row_offset, self.col_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataConverter;

    #[test]
    fn test_memory_block_operations() {
        let mut block = MemoryBlock::new(0);
        let data = vec![FpgaValue::Float(1.0); VECTOR_SIZE];
        
        assert!(block.write(data.clone()).is_ok());
        assert_eq!(block.read().unwrap().len(), VECTOR_SIZE);
    }

    #[test]
    fn test_shared_memory() {
        let mem = SharedMemory::new(4);
        let data = vec![FpgaValue::Float(1.0); VECTOR_SIZE];
        
        assert!(mem.write_block(0, data.clone()).is_ok());
        assert_eq!(mem.read_block(0).unwrap().len(), VECTOR_SIZE);
    }
}