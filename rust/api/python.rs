//! Python API bindings
//!
//! This module provides Python bindings for the accelerator using PyO3.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyRuntimeError};
use numpy::{PyArray1, PyReadonlyArray1, ToPyArray};
use tokio::runtime::Runtime;

use crate::types::{UnitId, Operation, Activation, VectorBlock};
use crate::error::AccelError;
use crate::core::compute::Vector;
use crate::hw::{
    fpga::MockFpga,
    unit::UnitManager,
};
use crate::api::async_api::{Accelerator, AsyncAccelerator};

/// Python wrapper for Vector
#[pyclass(name = "Vector")]
struct PyVector {
    inner: Vector,
    #[pyo3(get)]
    size: usize,
}

#[pymethods]
impl PyVector {
    #[new]
    fn new(size: usize) -> PyResult<Self> {
        let runtime = Runtime::new().map_err(|e| 
            PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        )?;

        let inner = runtime.block_on(async {
            Vector::new(size)
        }).map_err(|e| 
            PyValueError::new_err(format!("Failed to create vector: {}", e))
        )?;

        Ok(Self { inner, size })
    }

    /// Bind vector to processing unit
    fn bind_to_unit(&mut self, unit_id: usize) -> PyResult<()> {
        let runtime = Runtime::new().map_err(|e|
            PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        )?;

        let unit_id = UnitId::new(unit_id).ok_or_else(||
            PyValueError::new_err(format!("Invalid unit ID: {}", unit_id))
        )?;

        runtime.block_on(async {
            self.inner.bind_to_unit(unit_id).await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("Failed to bind unit: {}", e))
        )
    }

    /// Convert numpy array to vector
    #[staticmethod]
    fn from_numpy(array: PyReadonlyArray1<f32>) -> PyResult<Self> {
        let size = array.dims()[0];
        let mut vector = Self::new(size)?;

        let runtime = Runtime::new().map_err(|e|
            PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        )?;

        runtime.block_on(async {
            for (i, &value) in array.as_array().iter().enumerate() {
                vector.inner.set(i, value).await.map_err(|e|
                    PyValueError::new_err(format!("Failed to set value: {}", e))
                )?;
            }
            Ok::<_, PyErr>(())
        })?;

        Ok(vector)
    }

    /// Convert vector to numpy array
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<&'py PyArray1<f32>> {
        let runtime = Runtime::new().map_err(|e|
            PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        )?;

        let mut data = vec![0.0f32; self.size];
        runtime.block_on(async {
            for i in 0..self.size {
                data[i] = self.inner.get(i).await.map_err(|e|
                    PyValueError::new_err(format!("Failed to get value: {}", e))
                )?;
            }
            Ok::<_, PyErr>(())
        })?;

        Ok(data.to_pyarray(py))
    }
}

/// Python accelerator interface
#[pyclass(name = "Accelerator")]
struct PyAccelerator {
    inner: Accelerator,
    runtime: Runtime,
}

#[pymethods]
impl PyAccelerator {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Runtime::new().map_err(|e|
            PyRuntimeError::new_err(format!("Failed to create runtime: {}", e))
        )?;

        let unit_manager = UnitManager::new(Box::new(MockFpga::default()));
        let accelerator = Accelerator::new(unit_manager);

        Ok(Self {
            inner: accelerator,
            runtime,
        })
    }

    /// Initialize accelerator
    fn initialize(&self) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner.initialize().await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("Failed to initialize: {}", e))
        )
    }

    /// Copy data between vectors
    fn copy(&self, src: &PyVector, dst: &mut PyVector) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner.copy(&src.inner, &mut dst.inner).await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("Copy failed: {}", e))
        )
    }

    /// Add vectors
    fn add(&self, src: &PyVector, dst: &mut PyVector) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner.add(&src.inner, &mut dst.inner).await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("Addition failed: {}", e))
        )
    }

    /// Apply ReLU activation
    fn relu(&self, vector: &mut PyVector) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner.activate(&mut vector.inner, Activation::ReLU).await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("ReLU failed: {}", e))
        )
    }

    /// Apply tanh activation
    fn tanh(&self, vector: &mut PyVector) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner.activate(&mut vector.inner, Activation::Tanh).await
        }).map_err(|e|
            PyRuntimeError::new_err(format!("tanh failed: {}", e))
        )
    }
}

/// Python module definition
#[pymodule]
fn nn_accel(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyVector>()?;
    m.add_class::<PyAccelerator>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use numpy::PyArray;
    use pyo3::Python;

    #[test]
    fn test_python_vector() {
        Python::with_gil(|py| {
            // Create vector
            let vector = PyVector::new(32).unwrap();
            assert_eq!(vector.size, 32);

            // Test numpy conversion
            let data = vec![1.0f32; 32];
            let array = PyArray::from_vec(py, data);
            let vector = PyVector::from_numpy(array.readonly()).unwrap();
            
            let numpy_array = vector.to_numpy(py).unwrap();
            assert_eq!(numpy_array.dims(), [32]);
        });
    }

    #[test]
    fn test_python_accelerator() {
        Python::with_gil(|py| {
            let accelerator = PyAccelerator::new().unwrap();
            assert!(accelerator.initialize().is_ok());

            let mut vec1 = PyVector::new(32).unwrap();
            let mut vec2 = PyVector::new(32).unwrap();

            vec1.bind_to_unit(0).unwrap();
            vec2.bind_to_unit(1).unwrap();

            assert!(accelerator.copy(&vec1, &mut vec2).is_ok());
            assert!(accelerator.add(&vec1, &mut vec2).is_ok());
            assert!(accelerator.relu(&mut vec2).is_ok());
        });
    }
}