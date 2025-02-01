use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use numpy::{PyArray1, PyArray2, PyReadonlyArray2};
use crate::{Matrix, Vector, Store, MatrixIndex, Activation};

/// Python用の行列ラッパー
#[pyclass(name = "NNMatrix")]
pub struct PyMatrix {
    inner: Matrix,
    store: Store,
}

/// Python用のベクトルラッパー
#[pyclass(name = "NNVector")]
pub struct PyVector {
    inner: Vector,
}

#[pymethods]
impl PyMatrix {
    #[new]
    pub fn new(rows: usize, cols: usize) -> PyResult<Self> {
        let matrix = Matrix::new(rows, cols)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: matrix,
            store: Store::new(),
        })
    }

    /// NumPy配列から行列を作成
    #[staticmethod]
    pub fn from_numpy(array: PyReadonlyArray2<f32>) -> PyResult<Self> {
        let dims = array.dims();
        let mut matrix = Matrix::new(dims[0], dims[1])
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        for i in 0..dims[0] {
            for j in 0..dims[1] {
                matrix.set(MatrixIndex::new(i, j), array.get([i, j]).unwrap())
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
            }
        }

        Ok(Self {
            inner: matrix,
            store: Store::new(),
        })
    }

    /// 単位行列を作成
    #[staticmethod]
    pub fn identity(size: usize) -> PyResult<Self> {
        let matrix = Matrix::identity(size)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: matrix,
            store: Store::new(),
        })
    }

    /// 行列とベクトルの積を計算
    pub fn multiply(&self, vector: &PyVector) -> PyResult<PyVector> {
        let result = self.inner.multiply(&vector.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyVector { inner: result })
    }

    /// 行列をNumPy配列に変換
    pub fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<&'py PyArray2<f32>> {
        let data = self.inner.to_vec()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let rows = self.inner.rows();
        let cols = self.inner.cols();
        
        let array = PyArray2::zeros(py, [rows, cols], false);
        let mut array_mut = unsafe { array.as_array_mut() };
        
        for i in 0..rows {
            for j in 0..cols {
                array_mut[[i, j]] = data[i][j];
            }
        }
        
        Ok(array)
    }

    /// 行列を保存
    pub fn save(&self, name: &str) -> PyResult<()> {
        self.inner.store(name, &self.store)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// 行列を読み込み
    #[staticmethod]
    pub fn load(name: &str, rows: usize, cols: usize) -> PyResult<Self> {
        let store = Store::new();
        let matrix = Matrix::load(name, rows, cols, &store)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: matrix, store })
    }
}

#[pymethods]
impl PyVector {
    #[new]
    pub fn new(size: usize) -> PyResult<Self> {
        let vector = Vector::new(size)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: vector })
    }

    /// NumPy配列からベクトルを作成
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

    /// ReLUアクティベーションを適用
    pub fn relu(&self) -> Self {
        Self {
            inner: self.inner.apply_activation(Activation::ReLU)
        }
    }

    /// tanhアクティベーションを適用
    pub fn tanh(&self) -> Self {
        Self {
            inner: self.inner.apply_activation(Activation::Tanh)
        }
    }

    /// ベクトルをNumPy配列に変換
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
}

#[pymodule]
fn nn_accel(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyMatrix>()?;
    m.add_class::<PyVector>()?;
    Ok(())
}