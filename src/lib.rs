use pyo3::prelude::*;
use numpy::{PyArray1, PyArray2, ToPyArray};
use numpy::ndarray::{Array1, Array2};

mod types;
mod memory;
mod math;
mod compute;
mod device;

use types::{DataConverter, QFormat, FpgaError};
use math::{Matrix, Vector};
use device::FpgaAccelerator;

#[pyclass]
struct PyFpgaAccelerator {
    inner: FpgaAccelerator,
    q_format: QFormat,
}

#[pymethods]
impl PyFpgaAccelerator {
    #[new]
    fn new(q: Option<u8>, int: Option<u8>) -> PyResult<Self> {
        // デフォルト値：Q23.8
        let q_format = QFormat::new(
            q.unwrap_or(23),
            int.unwrap_or(8)
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(Self {
            inner: FpgaAccelerator::new(4, q_format)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
            q_format,
        })
    }

    #[getter]
    fn get_format(&self) -> PyResult<(u8, u8)> {
        Ok((self.q_format.q, self.q_format.int))
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

        let fpga_matrix = Matrix::from_f32(&matrix_data, self.q_format)
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
        
        let fpga_vector = Vector::from_f32(&vector_data, self.q_format)
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
        let fpga_vector = Vector::from_f32(&vector_data, self.q_format)
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

    // フォーマット情報の文字列表現を返す
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Q{}.{} 固定小数点形式 FPGA アクセラレータ", 
            self.q_format.q, self.q_format.int))
    }
}

#[pymodule]
fn fpga_accelerator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFpgaAccelerator>()?;
    Ok(())
}