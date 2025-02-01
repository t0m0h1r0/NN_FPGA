// python.rs

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use numpy::{PyArray1, PyArray2, PyReadonlyArray2};
use crate::{Matrix, Vector, Store, MatrixIndex, Activation, Operation};

#[pyclass(name = "NNVector")]
pub struct PyVector {
    inner: Vector,
}

#[pymethods]
impl PyVector {
    #[new]
    pub fn new(size: usize) -> PyResult<Self> {
        let vector = Vector::new(size)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: vector })
    }

    /// 新規: ユニットへのバインド
    pub fn bind_to_unit(&mut self, unit_id: usize) -> PyResult<()> {
        self.inner.bind_to_unit(unit_id)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// 新規: 他ユニットからのデータコピー
    pub fn copy_from_unit(&mut self, source_unit: usize) -> PyResult<()> {
        self.inner.copy_from_unit(source_unit)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// 新規: 他ユニットのデータとの加算
    pub fn add_from_unit(&mut self, source_unit: usize) -> PyResult<()> {
        self.inner.add_from_unit(source_unit)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    pub fn from_numpy(array: PyReadonlyArray1<f32>) -> PyResult<Self> {
        let size = array.dims()[0];
        let mut vector = Vector::new(size)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        for i in 0..size {
            vector.set(i, array.get(i).unwrap())
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }

        Ok(Self { inner: vector })
    }

    pub fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<&'py PyArray1<f32>> {
        let size = self.inner.size();
        let array = PyArray1::zeros(py, size, false);
        let mut array_mut = unsafe { array.as_array_mut() };
        
        for i in 0..size {
            array_mut[i] = self.inner.get(i)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }
        
        Ok(array)
    }

    pub fn relu(&self) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.apply_activation(Activation::ReLU)
                .map_err(|e| PyValueError::new_err(e.to_string()))?
        })
    }

    pub fn tanh(&self) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.apply_activation(Activation::Tanh)
                .map_err(|e| PyValueError::new_err(e.to_string()))?
        })
    }
}

#[pymodule]
fn nn_accel(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyVector>()?;
    Ok(())
}