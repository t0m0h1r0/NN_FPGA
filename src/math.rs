use crate::types::{FpgaError, Result, FpgaValue, MATRIX_SIZE, VECTOR_SIZE, DataConverter};
use std::ops::{Add, Mul};

#[derive(Debug, Clone)]
pub struct Matrix {
    data: Vec<Vec<FpgaValue>>,
    rows: usize,
    cols: usize,
}

impl Matrix {
    pub fn new(data: Vec<Vec<FpgaValue>>) -> Result<Self> {
        if data.is_empty() {
            return Err(FpgaError::Computation("Empty matrix".into()));
        }
        let rows = data.len();
        let cols = data[0].len();
        if data.iter().any(|row| row.len() != cols) {
            return Err(FpgaError::Computation("Irregular matrix shape".into()));
        }
        
        Ok(Self { data, rows, cols })
    }

    pub fn from_f32(data: &[Vec<f32>], converter: &DataConverter) -> Result<Self> {
        let converted = data.iter()
            .map(|row| row.iter()
                .map(|&x| converter.convert(x))
                .collect::<Result<Vec<_>>>())
            .collect::<Result<Vec<_>>>()?;
        Self::new(converted)
    }

    pub fn multiply_vector(&self, vector: &Vector) -> Result<Vector> {
        if self.cols != vector.len() {
            return Err(FpgaError::Computation("Dimension mismatch".into()));
        }

        let result = (0..self.rows)
            .map(|i| {
                let sum = (0..self.cols)
                    .map(|j| {
                        let a = self.data[i][j].as_f32();
                        let b = vector.data[j].as_f32();
                        a * b
                    })
                    .sum();
                Ok(FpgaValue::Float(sum))
            })
            .collect::<Result<Vec<_>>>()?;

        Vector::new(result)
    }

    pub fn split_blocks(&self) -> Result<Vec<Matrix>> {
        if self.rows % MATRIX_SIZE != 0 || self.cols % MATRIX_SIZE != 0 {
            return Err(FpgaError::Computation("Matrix size must be multiple of block size".into()));
        }

        let mut blocks = Vec::new();
        for i in (0..self.rows).step_by(MATRIX_SIZE) {
            for j in (0..self.cols).step_by(MATRIX_SIZE) {
                let block_data: Vec<Vec<FpgaValue>> = self.data[i..i + MATRIX_SIZE]
                    .iter()
                    .map(|row| row[j..j + MATRIX_SIZE].to_vec())
                    .collect();
                blocks.push(Matrix::new(block_data)?);
            }
        }
        Ok(blocks)
    }
}

#[derive(Debug, Clone)]
pub struct Vector {
    data: Vec<FpgaValue>,
}

impl Vector {
    pub fn new(data: Vec<FpgaValue>) -> Result<Self> {
        if data.is_empty() {
            return Err(FpgaError::Computation("Empty vector".into()));
        }
        Ok(Self { data })
    }

    pub fn from_f32(data: &[f32], converter: &DataConverter) -> Result<Self> {
        let converted = data.iter()
            .map(|&x| converter.convert(x))
            .collect::<Result<Vec<_>>>()?;
        Self::new(converted)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn split(&self, block_size: usize) -> Result<Vec<Vector>> {
        if self.len() % block_size != 0 {
            return Err(FpgaError::Computation("Vector size must be multiple of block size".into()));
        }

        let mut blocks = Vec::new();
        for chunk in self.data.chunks(block_size) {
            blocks.push(Vector::new(chunk.to_vec())?);
        }
        Ok(blocks)
    }

    pub fn add(&self, other: &Vector) -> Result<Vector> {
        if self.len() != other.len() {
            return Err(FpgaError::Computation("Vector size mismatch".into()));
        }

        let result = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| FpgaValue::Float(a.as_f32() + b.as_f32()))
            .collect();

        Vector::new(result)
    }

    pub fn relu(&self) -> Result<Vector> {
        let result = self.data.iter()
            .map(|x| FpgaValue::Float(x.as_f32().max(0.0)))
            .collect();
        Vector::new(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataFormat;

    #[test]
    fn test_matrix_vector_multiplication() {
        let converter = DataConverter::new(DataFormat::Full);
        
        let matrix_data = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
        ];
        let vector_data = vec![2.0, 1.0];

        let matrix = Matrix::from_f32(&matrix_data, &converter).unwrap();
        let vector = Vector::from_f32(&vector_data, &converter).unwrap();

        let result = matrix.multiply_vector(&vector).unwrap();
        assert_eq!(result.data[0].as_f32(), 4.0);
        assert_eq!(result.data[1].as_f32(), 10.0);
    }

    #[test]
    fn test_vector_operations() {
        let converter = DataConverter::new(DataFormat::Full);
        
        let v1 = Vector::from_f32(&[1.0, -2.0], &converter).unwrap();
        let v2 = Vector::from_f32(&[2.0, 3.0], &converter).unwrap();

        let sum = v1.add(&v2).unwrap();
        assert_eq!(sum.data[0].as_f32(), 3.0);
        assert_eq!(sum.data[1].as_f32(), 1.0);

        let relu = v1.relu().unwrap();
        assert_eq!(relu.data[0].as_f32(), 1.0);
        assert_eq!(relu.data[1].as_f32(), 0.0);
    }
}