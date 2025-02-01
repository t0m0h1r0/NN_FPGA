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

#[pymethods]
impl PyFpgaAccelerator {
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
            "fixed_point" => VectorConversionType::FixedPoint,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid conversion type")),
        };

        let fpga_vector = FpgaVector::from_numpy(&input_vec, converted_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        // 変換後のベクトルをNumPy配列に戻す
        let converted_vec = fpga_vector.to_numpy();
        Ok(converted_vec.to_pyarray(py).to_owned())
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
            "fixed_point" => MatrixConversionType::FixedPoint,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid conversion type")),
        };

        let fpga_matrix = FpgaMatrix::from_numpy(&input_matrix, converted_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        // 変換後の行列をNumPy配列に戻す
        let converted_matrix = fpga_matrix.to_numpy();
        Ok(converted_matrix.to_pyarray(py).to_owned())
    }
}