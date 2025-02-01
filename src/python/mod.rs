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

    fn compute_vector(&mut self, py: Python, input: &PyArray1<f32>, comp_type: &str) -> PyResult<Py<PyArray1<f32>>> {
        let input_vec: Vec<f32> = input.readonly().as_slice()?.to_vec();
        
        let computation_type = match comp_type {
            "add" => ComputationType::Add,
            "mul" => ComputationType::Multiply,
            "tanh" => ComputationType::Tanh,
            "relu" => ComputationType::ReLU,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid computation type")),
        };

        let fpga_input = FpgaVector::from_numpy(&input_vec, VectorConversionType::Full)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        let result = self.inner.compute(&fpga_input, computation_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }

    fn compute_matrix_vector_multiply(&mut self, py: Python, matrix: &PyArray2<f32>, vector: &PyArray1<f32>) -> PyResult<Py<PyArray1<f32>>> {
        // NumPy配列からVecに変換
        let matrix_data: Array2<f32> = matrix.readonly().as_array().to_owned();
        let vector_data: Array1<f32> = vector.readonly().as_array().to_owned();

        // FPGAデータ型に変換
        let fpga_matrix = FpgaMatrix::from_numpy(
            &matrix_data
                .rows()
                .into_iter()
                .map(|row| row.to_vec())
                .collect()
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let fpga_vector = FpgaVector::from_numpy(vector_data.to_vec().as_slice(), VectorConversionType::Full)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        // 行列ベクトル乗算の計算
        let result = self.inner.compute(&fpga_matrix, ComputationType::MatrixVectorMultiply)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }

    /// ベクトルの変換メソッド
    fn convert_vector(
        &self, 
        py: Python, 
        input: &PyArray1<f32>, 
        conversion_type: &str
    ) -> PyResult<Py<PyArray1<f32>>> {
        let input_vec: Vec<f32> = input.readonly().as_slice()?.to_vec();
        
        let converted_type = match conversion_type {
            "full" => VectorConversionType::Full,
            "trinary" => VectorConversionType::Trinary,
            "fixed_point_1s31" => VectorConversionType::FixedPoint1s31,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid conversion type")),
        };

        let fpga_vector = FpgaVector::from_numpy(&input_vec, converted_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(fpga_vector.to_numpy().to_pyarray(py).to_owned())
    }

    /// 行列の変換メソッド
    fn convert_matrix(
        &self, 
        py: Python, 
        input: &PyArray2<f32>, 
        conversion_type: &str
    ) -> PyResult<Py<PyArray2<f32>>> {
        // NumPy配列からVecに変換
        let matrix_data: Array2<f32> = input.readonly().as_array().to_owned();
        let input_matrix: Vec<Vec<f32>> = matrix_data
            .rows()
            .into_iter()
            .map(|row| row.to_vec())
            .collect();
        
        let converted_type = match conversion_type {
            "full" => MatrixConversionType::Full,
            "trinary" => MatrixConversionType::Trinary,
            "fixed_point_1s31" => MatrixConversionType::FixedPoint1s31,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid conversion type")),
        };

        let fpga_matrix = FpgaMatrix::from_numpy(&input_matrix, converted_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(fpga_matrix.to_numpy().to_pyarray(py).to_owned())
    }
}

// データ型変換用のトレイト実装
impl ComputeInput for Box<dyn ComputeInput> {
    fn as_vector(&self) -> Option<&FpgaVector> {
        (**self).as_vector()
    }
    
    fn as_matrix(&self) -> Option<&FpgaMatrix> {
        (**self).as_matrix()
    }
}