use pyo3::prelude::*;
use numpy::{PyArray1, PyArray2, ToPyArray};
use numpy::ndarray::{Array1, Array2};

use crate::core::data_types::{
    FpgaVector, 
    FpgaMatrix, 
    ComputationType, 
    VectorConversionType,
    MatrixConversionType
};
use crate::core::device::FpgaAccelerator;

#[pymodule]
fn fpga_accelerator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFpgaAccelerator>()?;
    Ok(())
}

#[pyclass]
struct PyFpgaAccelerator {
    inner: FpgaAccelerator,
}

#[pymethods]
impl PyFpgaAccelerator {
    #[new]
    fn new() -> Self {
        Self {
            inner: FpgaAccelerator::new(),
        }
    }

    /// 行列を準備（整数値-1,0,1のみ受け付け）
    fn prepare_matrix(
        &mut self,
        py: Python,
        matrix: &PyArray2<i32>
    ) -> PyResult<()> {
        let matrix_data: Array2<i32> = matrix.readonly().as_array().to_owned();
        
        // 値の検証（-1, 0, 1のみ許可）
        if matrix_data.iter().any(|&x| x < -1 || x > 1) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Matrix values must be -1, 0, or 1"
            ));
        }

        let fpga_matrix = FpgaMatrix::from_numpy(
            &matrix_data
                .rows()
                .into_iter()
                .map(|row| row.to_vec())
                .collect()
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        self.inner.prepare_matrix(&fpga_matrix)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// 準備済み行列とベクトルの乗算
    fn compute_with_prepared_matrix(
        &mut self,
        py: Python,
        vector: &PyArray1<i32>
    ) -> PyResult<Py<PyArray1<i32>>> {
        let vector_data: Vec<i32> = vector.readonly().as_slice()?.to_vec();
        
        // 値の検証（-1, 0, 1のみ許可）
        if vector_data.iter().any(|&x| x < -1 || x > 1) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Vector values must be -1, 0, or 1"
            ));
        }

        let fpga_vector = FpgaVector::from_numpy(&vector_data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let result = self.inner.compute_with_prepared_matrix(&fpga_vector)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }

    /// ベクトル計算
    fn compute_vector(
        &mut self, 
        py: Python, 
        input: &PyArray1<i32>, 
        comp_type: &str
    ) -> PyResult<Py<PyArray1<i32>>> {
        let input_vec: Vec<i32> = input.readonly().as_slice()?.to_vec();
        
        // 値の検証（-1, 0, 1のみ許可）
        if input_vec.iter().any(|&x| x < -1 || x > 1) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Input values must be -1, 0, or 1"
            ));
        }

        let computation_type = match comp_type {
            "add" => ComputationType::Add,
            "mul" => ComputationType::Multiply,
            "tanh" => ComputationType::Tanh,
            "relu" => ComputationType::ReLU,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid computation type")),
        };

        let fpga_input = FpgaVector::from_numpy(&input_vec)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        let result = self.inner.compute(&fpga_input, computation_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }
}