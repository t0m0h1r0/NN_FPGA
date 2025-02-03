use pyo3::prelude::*;
use numpy::{PyArray1, PyArray2, ToPyArray};
use numpy::ndarray::{Array1, Array2};

mod types;
mod memory;
mod math;
mod compute;
mod device;

use types::{DataConverter, DataFormat, FpgaError};
use math::{Matrix, Vector};
use device::FpgaAccelerator;

#[pyclass]
struct PyFpgaAccelerator {
    inner: FpgaAccelerator,
    converter: DataConverter,
}

#[pymethods]
impl PyFpgaAccelerator {
    #[new]
    fn new(precision: Option<&str>) -> PyResult<Self> {
        let format = match precision.unwrap_or("full") {
            "full" => DataFormat::Full,
            "fixed" => DataFormat::Fixed { scale: 31 },
            "trinary" => DataFormat::Trinary,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("不正な精度指定")),
        };

        Ok(Self {
            inner: FpgaAccelerator::new(4, DataConverter::new(format)),
            converter: DataConverter::new(format),
        })
    }

    #[pyo3(text_signature = "(self, matrix)")]
    fn prepare_matrix(
        &mut self,
        py: Python,
        matrix: &PyArray2<f32>
    ) -> PyResult<()> {
        let matrix_data: Vec<Vec<f32>> = matrix
            .readonly()
            .as_array()
            .rows()
            .into_iter()
            .map(|row| row.to_vec())
            .collect();

        let fpga_matrix = Matrix::from_f32(&matrix_data, &self.converter)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        self.inner.prepare_matrix(&fpga_matrix)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[pyo3(text_signature = "(self, vector)")]
    fn compute_matrix_vector(
        &mut self,
        py: Python,
        vector: &PyArray1<f32>
    ) -> PyResult<Py<PyArray1<f32>>> {
        let vector_data: Vec<f32> = vector.readonly().as_slice()?.to_vec();
        
        let fpga_vector = Vector::from_f32(&vector_data, &self.converter)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let result = self.inner.compute_matrix_vector(&fpga_vector)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let numpy_result: Vec<f32> = result.data.iter().map(|x| x.as_f32()).collect();
        Ok(numpy_result.to_pyarray(py).to_owned())
    }

    #[pyo3(text_signature = "(self, vector, operation)")]
    fn compute_vector(
        &mut self,
        py: Python,
        vector: &PyArray1<f32>,
        operation: &str
    ) -> PyResult<Py<PyArray1<f32>>> {
        let vector_data: Vec<f32> = vector.readonly().as_slice()?.to_vec();
        let fpga_vector = Vector::from_f32(&vector_data, &self.converter)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let op = match operation {
            "relu" => compute::ComputeOperation::VectorReLU,
            "add" => compute::ComputeOperation::VectorAdd,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("不正な演算タイプ")),
        };

        let result = self.inner.compute_vector_operation(&fpga_vector, op)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let numpy_result: Vec<f32> = result.data.iter().map(|x| x.as_f32()).collect();
        Ok(numpy_result.to_pyarray(py).to_owned())
    }
}

#[pymodule]
fn fpga_accelerator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFpgaAccelerator>()?;
    Ok(())
}