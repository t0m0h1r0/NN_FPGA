use crate::types::{
    DataConversionType, FpgaValue, MATRIX_SIZE, VECTOR_SIZE,
    TypeConversionError, ComputationType,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MathError {
    #[error("無効なベクトルサイズ: {size}, 16の倍数である必要があります")]
    InvalidVectorSize { size: usize },
    
    #[error("無効な行列サイズ: {rows}x{cols}, 各次元は16の倍数である必要があります")]
    InvalidMatrixSize { rows: usize, cols: usize },
    
    #[error("次元の不一致: 行列 {matrix_size}x{matrix_cols}, ベクトル {vector_size}")]
    DimensionMismatch {
        matrix_size: usize,
        matrix_cols: usize,
        vector_size: usize,
    },
    
    #[error("データ型変換エラー: {0}")]
    TypeConversion(#[from] TypeConversionError),
}

/// FPGAベクトル
#[derive(Debug, Clone)]
pub struct FpgaVector {
    data: Vec<FpgaValue>,
    conversion_type: DataConversionType,
}

/// FPGA行列
#[derive(Debug, Clone)]
pub struct FpgaMatrix {
    data: Vec<Vec<FpgaValue>>,
    rows: usize,
    cols: usize,
    conversion_type: DataConversionType,
}

impl FpgaVector {
    /// NumPy配列からFPGAベクトルを生成
    pub fn from_numpy(values: &[f32], conversion_type: DataConversionType) -> Result<Self, MathError> {
        if values.len() % VECTOR_SIZE != 0 {
            return Err(MathError::InvalidVectorSize { size: values.len() });
        }

        let data = values
            .iter()
            .map(|&v| FpgaValue::from_f32(v, conversion_type))
            .collect();

        Ok(Self {
            data,
            conversion_type,
        })
    }

    /// FPGAベクトルをNumPy配列に変換
    pub fn to_numpy(&self) -> Vec<f32> {
        self.data.iter().map(|v| v.to_f32()).collect()
    }

    /// ベクトルのサイズを取得
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// 変換タイプを取得
    pub fn conversion_type(&self) -> DataConversionType {
        self.conversion_type
    }

    /// インデックスでの値取得
    pub fn get(&self, index: usize) -> FpgaValue {
        self.data[index]
    }

    /// 変換タイプを変更
    pub fn convert(&self, new_type: DataConversionType) -> Self {
        let data = self
            .data
            .iter()
            .map(|v| FpgaValue::from_f32(v.to_f32(), new_type))
            .collect();

        Self {
            data,
            conversion_type: new_type,
        }
    }
}

impl FpgaMatrix {
    /// NumPy行列からFPGA行列を生成
    pub fn from_numpy(
        values: &[Vec<f32>], 
        conversion_type: DataConversionType
    ) -> Result<Self, MathError> {
        // サイズチェック
        if values.is_empty() || values.len() % MATRIX_SIZE != 0 {
            return Err(MathError::InvalidMatrixSize {
                rows: values.len(),
                cols: 0,
            });
        }

        let cols = values[0].len();
        if cols % MATRIX_SIZE != 0 {
            return Err(MathError::InvalidMatrixSize {
                rows: values.len(),
                cols,
            });
        }

        // データ変換
        let data = values
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&v| FpgaValue::from_f32(v, conversion_type))
                    .collect()
            })
            .collect();

        Ok(Self {
            data,
            rows: values.len(),
            cols,
            conversion_type,
        })
    }

    /// FPGA行列をNumPy行列に変換
    pub fn to_numpy(&self) -> Vec<Vec<f32>> {
        self.data
            .iter()
            .map(|row| row.iter().map(|v| v.to_f32()).collect())
            .collect()
    }

    /// 行列を16x16のブロックに分割
    pub fn split_into_blocks(&self) -> Vec<Vec<FpgaMatrix>> {
        let mut blocks = Vec::new();

        for row_start in (0..self.rows).step_by(MATRIX_SIZE) {
            let mut row_blocks = Vec::new();
            
            for col_start in (0..self.cols).step_by(MATRIX_SIZE) {
                let mut block_data = Vec::new();
                
                // ブロックデータの抽出
                for r in row_start..row_start + MATRIX_SIZE {
                    let mut block_row = Vec::new();
                    for c in col_start..col_start + MATRIX_SIZE {
                        let value = if r < self.rows && c < self.cols {
                            self.data[r][c]
                        } else {
                            FpgaValue::from_f32(0.0, self.conversion_type)
                        };
                        block_row.push(value);
                    }
                    block_data.push(block_row);
                }

                // ブロック行列の作成
                row_blocks.push(Self {
                    data: block_data,
                    rows: MATRIX_SIZE,
                    cols: MATRIX_SIZE,
                    conversion_type: self.conversion_type,
                });
            }
            blocks.push(row_blocks);
        }

        blocks
    }

    /// 行数を取得
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// 列数を取得
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// 指定位置の値を取得
    pub fn get(&self, row: usize, col: usize) -> FpgaValue {
        self.data[row][col]
    }
}