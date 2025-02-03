mod types;
mod math;
mod device;

use pyo3::prelude::*;
use numpy::{PyArray1, PyArray2, ToPyArray};
use numpy::ndarray::{Array1, Array2};

use crate::types::{ComputationType, DataConversionType};
use crate::math::{FpgaVector, FpgaMatrix};
use crate::device::FpgaAccelerator;

/// FPGA高速演算アクセラレータのPythonインターフェース
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

    /// 行列を準備（事前にキャッシュ）
    ///
    /// Args:
    ///     matrix (numpy.ndarray): 準備する行列（float32型）
    ///
    /// Returns:
    ///     None
    ///
    /// Raises:
    ///     ValueError: 行列のサイズが不正な場合
    #[pyo3(text_signature = "(self, matrix)")]
    fn prepare_matrix(
        &mut self,
        py: Python,
        matrix: &PyArray2<f32>
    ) -> PyResult<()> {
        let matrix_data: Array2<f32> = matrix.readonly().as_array().to_owned();
        
        let fpga_matrix = FpgaMatrix::from_numpy(
            &matrix_data
                .rows()
                .into_iter()
                .map(|row| row.to_vec())
                .collect(),
            DataConversionType::Full
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        self.inner.prepare_matrix(&fpga_matrix)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// 準備済み行列とベクトルの乗算
    ///
    /// Args:
    ///     vector (numpy.ndarray): 入力ベクトル（float32型）
    ///
    /// Returns:
    ///     numpy.ndarray: 計算結果のベクトル
    ///
    /// Raises:
    ///     ValueError: ベクトルサイズが不正な場合
    ///     RuntimeError: 行列が準備されていない場合
    #[pyo3(text_signature = "(self, vector)")]
    fn compute_with_prepared_matrix(
        &mut self,
        py: Python,
        vector: &PyArray1<f32>
    ) -> PyResult<Py<PyArray1<f32>>> {
        let vector_data: Vec<f32> = vector.readonly().as_slice()?.to_vec();
        
        let fpga_vector = FpgaVector::from_numpy(
            &vector_data,
            DataConversionType::Full
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let result = self.inner.compute_with_prepared_matrix(&fpga_vector)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }

    /// ベクトル演算の実行
    ///
    /// Args:
    ///     vector (numpy.ndarray): 入力ベクトル（float32型）
    ///     computation_type (str): 計算タイプ（"add", "mul", "tanh", "relu"）
    ///
    /// Returns:
    ///     numpy.ndarray: 計算結果のベクトル
    #[pyo3(text_signature = "(self, vector, computation_type)")]
    fn compute_vector(
        &mut self,
        py: Python,
        vector: &PyArray1<f32>,
        computation_type: &str
    ) -> PyResult<Py<PyArray1<f32>>> {
        let vector_data: Vec<f32> = vector.readonly().as_slice()?.to_vec();
        
        let comp_type = match computation_type {
            "add" => ComputationType::Add,
            "mul" => ComputationType::Multiply,
            "tanh" => ComputationType::Tanh,
            "relu" => ComputationType::ReLU,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("不正な計算タイプ: {}", computation_type)
            )),
        };

        let fpga_vector = FpgaVector::from_numpy(
            &vector_data,
            DataConversionType::Full
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let result = self.inner.compute_scalar(&fpga_vector, comp_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(result.to_numpy().to_pyarray(py).to_owned())
    }

    /// データ型変換（ベクトル）
    ///
    /// Args:
    ///     vector (numpy.ndarray): 入力ベクトル（float32型）
    ///     conversion_type (str): 変換タイプ（"full", "trinary", "fixed_point_1s31"）
    ///
    /// Returns:
    ///     numpy.ndarray: 変換後のベクトル
    #[pyo3(text_signature = "(self, vector, conversion_type)")]
    fn convert_vector(
        &self,
        py: Python,
        vector: &PyArray1<f32>,
        conversion_type: &str
    ) -> PyResult<Py<PyArray1<f32>>> {
        let vector_data: Vec<f32> = vector.readonly().as_slice()?.to_vec();
        
        let conv_type = match conversion_type {
            "full" => DataConversionType::Full,
            "trinary" => DataConversionType::Trinary,
            "fixed_point_1s31" => DataConversionType::FixedPoint1s31,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("不正な変換タイプ: {}", conversion_type)
            )),
        };

        let fpga_vector = FpgaVector::from_numpy(&vector_data, conv_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(fpga_vector.to_numpy().to_pyarray(py).to_owned())
    }

    /// データ型変換（行列）
    ///
    /// Args:
    ///     matrix (numpy.ndarray): 入力行列（float32型）
    ///     conversion_type (str): 変換タイプ（"full", "trinary", "fixed_point_1s31"）
    ///
    /// Returns:
    ///     numpy.ndarray: 変換後の行列
    #[pyo3(text_signature = "(self, matrix, conversion_type)")]
    fn convert_matrix(
        &self,
        py: Python,
        matrix: &PyArray2<f32>,
        conversion_type: &str
    ) -> PyResult<Py<PyArray2<f32>>> {
        let matrix_data: Array2<f32> = matrix.readonly().as_array().to_owned();
        
        let conv_type = match conversion_type {
            "full" => DataConversionType::Full,
            "trinary" => DataConversionType::Trinary,
            "fixed_point_1s31" => DataConversionType::FixedPoint1s31,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("不正な変換タイプ: {}", conversion_type)
            )),
        };

        let fpga_matrix = FpgaMatrix::from_numpy(
            &matrix_data
                .rows()
                .into_iter()
                .map(|row| row.to_vec())
                .collect(),
            conv_type
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let numpy_result: Vec<Vec<f32>> = fpga_matrix.to_numpy();
        let rows = numpy_result.len();
        let cols = numpy_result[0].len();
        
        let flat_data: Vec<f32> = numpy_result.into_iter().flatten().collect();
        let array = Array2::from_shape_vec((rows, cols), flat_data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(array.to_pyarray(py).to_owned())
    }
}

#[pymodule]
fn fpga_accelerator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFpgaAccelerator>()?;
    Ok(())