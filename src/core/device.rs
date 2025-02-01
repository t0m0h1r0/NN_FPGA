use crate::core::data_types::{
    FpgaVector, 
    FpgaMatrix, 
    ComputationType, 
    CompressedNum
};
use crate::error::AcceleratorError;
use log::{info, error};

pub struct FpgaAccelerator {
    total_units: usize,
    available_units: Vec<bool>,
    memory_vector_size: usize,
    memory_matrix_size: usize,
    block_size: usize,
    // 追加: 前処理済み行列用のフィールド
    prepared_matrix: Option<Vec<Vec<FpgaMatrix>>>, // ブロック分割済みの行列
    matrix_rows: usize,                           // 元の行列の行数
    matrix_cols: usize,                           // 元の行列の列数
}

pub trait ComputeInput {
    fn as_vector(&self) -> Option<&FpgaVector>;
    fn as_matrix(&self) -> Option<&FpgaMatrix>;
}

impl ComputeInput for FpgaVector {
    fn as_vector(&self) -> Option<&FpgaVector> {
        Some(self)
    }
    fn as_matrix(&self) -> Option<&FpgaMatrix> {
        None
    }
}

impl ComputeInput for FpgaMatrix {
    fn as_vector(&self) -> Option<&FpgaVector> {
        None
    }
    fn as_matrix(&self) -> Option<&FpgaMatrix> {
        Some(self)
    }
}

impl FpgaAccelerator {
    pub fn new() -> Self {
        Self {
            total_units: 256,
            available_units: vec![true; 256],
            memory_vector_size: 64,
            memory_matrix_size: 256,
            block_size: 16,
            prepared_matrix: None,
            matrix_rows: 0,
            matrix_cols: 0,
        }
    }

    /// 行列を準備し、内部に保持
    pub fn prepare_matrix(
        &mut self, 
        matrix: &impl ComputeInput
    ) -> Result<(), AcceleratorError> {
        let matrix = matrix.as_matrix().ok_or_else(|| 
            AcceleratorError::DataConversionError("Expected matrix input".to_string())
        )?;

        // 行列をブロックに分割
        let matrix_blocks = matrix.split_into_blocks(self.block_size);
        
        // 行列の元のサイズを保存
        self.matrix_rows = matrix.rows;
        self.matrix_cols = matrix.cols;
        
        // 分割した行列を保持
        self.prepared_matrix = Some(matrix_blocks);
        
        Ok(())
    }

    /// 準備済みの行列とベクトルの乗算を実行
    pub fn compute_with_prepared_matrix(
        &mut self,
        vector: &impl ComputeInput
    ) -> Result<FpgaVector, AcceleratorError> {
        // 準備済み行列の確認
        let matrix_blocks = self.prepared_matrix.as_ref().ok_or_else(|| 
            AcceleratorError::DataConversionError("Matrix not prepared".to_string())
        )?;

        // ベクトルの次元チェック
        let vector = vector.as_vector().ok_or_else(|| 
            AcceleratorError::DataConversionError("Expected vector input".to_string())
        )?;
        if vector.dimension != self.matrix_cols {
            return Err(AcceleratorError::InvalidDimension(vector.dimension));
        }

        self.compute_matrix_vector_multiply_internal(matrix_blocks, vector)
    }

    pub fn compute(
        &mut self, 
        input: &impl ComputeInput, 
        computation_type: ComputationType
    ) -> Result<FpgaVector, AcceleratorError> {
        match computation_type {
            ComputationType::MatrixVectorMultiply => {
                if let Some(matrix) = input.as_matrix() {
                    // 一時的な行列ベクトル乗算
                    let matrix_blocks = matrix.split_into_blocks(self.block_size);
                    // ダミーベクトルを作成（この部分は要修正）
                    let dummy_vector = FpgaVector::from_numpy(
                        &vec![0.0; matrix.cols],
                        crate::core::data_types::VectorConversionType::Full
                    )?;
                    self.compute_matrix_vector_multiply_internal(&matrix_blocks, &dummy_vector)
                } else {
                    Err(AcceleratorError::DataConversionError(
                        "Expected matrix input for matrix-vector multiplication".to_string()
                    ))
                }
            },
            _ => {
                if let Some(vector) = input.as_vector() {
                    self.scalar_compute(vector, computation_type)
                } else {
                    Err(AcceleratorError::UnsupportedComputationType(
                        "Input type not supported for this computation".to_string()
                    ))
                }
            }
        }
    }

    // 内部実装用のメソッド
    fn compute_matrix_vector_multiply_internal(
        &mut self,
        matrix_blocks: &Vec<Vec<FpgaMatrix>>,
        input_vector: &FpgaVector
    ) -> Result<FpgaVector, AcceleratorError> {
        let mut result_vector = Vec::new();

        for row_blocks in matrix_blocks {
            let mut row_result = vec![CompressedNum::Full(0.0); self.block_size];

            for block in row_blocks {
                let unit_id = self.select_unit()?;
                let block_result = self.compute_matrix_block(block, input_vector)?;
                
                // 部分結果を累積
                for (i, val) in block_result.data.iter().enumerate() {
                    row_result[i] = match (row_result[i], val) {
                        (CompressedNum::Full(a), CompressedNum::Full(b)) => 
                            CompressedNum::Full(a + b),
                        (CompressedNum::FixedPoint1s31(a), CompressedNum::FixedPoint1s31(b)) => 
                            CompressedNum::FixedPoint1s31(a + b),
                        _ => CompressedNum::Full(0.0),
                    };
                }

                self.release_unit(unit_id);
            }

            result_vector.extend_from_slice(&row_result);
        }

        FpgaVector::from_numpy(
            &result_vector.iter().map(|x| match x {
                CompressedNum::Full(val) => *val,
                CompressedNum::FixedPoint1s31(val) => 
                    CompressedNum::from_fixed_point_1s31(*val),
                CompressedNum::Trinary(val) => 
                    CompressedNum::from_trinary(*val),
            }).collect::<Vec<f32>>(),
            crate::core::data_types::VectorConversionType::Full
        )
    }

    fn compute_matrix_block(
        &mut self, 
        matrix_block: &FpgaMatrix,
        input_vector: &FpgaVector
    ) -> Result<FpgaVector, AcceleratorError> {
        // 行列ブロックとベクトルの乗算
        let mut block_result = Vec::new();
        for row in &matrix_block.data {
            let dot_product = row.iter()
                .zip(input_vector.data.iter())
                .map(|(a, b)| match (a, b) {
                    (CompressedNum::Full(a_val), CompressedNum::Full(b_val)) => 
                        CompressedNum::Full(a_val * b_val),
                    (CompressedNum::FixedPoint1s31(a_val), CompressedNum::FixedPoint1s31(b_val)) => 
                        CompressedNum::FixedPoint1s31(a_val * b_val),
                    _ => CompressedNum::Full(0.0),
                })
                .fold(CompressedNum::Full(0.0), |acc, x| match (acc, x) {
                    (CompressedNum::Full(a), CompressedNum::Full(b)) => 
                        CompressedNum::Full(a + b),
                    (CompressedNum::FixedPoint1s31(a), CompressedNum::FixedPoint1s31(b)) => 
                        CompressedNum::FixedPoint1s31(a + b),
                    _ => CompressedNum::Full(0.0),
                });
            block_result.push(dot_product);
        }

        FpgaVector::from_numpy(&block_result.iter().map(|x| match x {
            CompressedNum::Full(val) => *val,
            CompressedNum::FixedPoint1s31(val) => 
                CompressedNum::from_fixed_point_1s31(*val),
            CompressedNum::Trinary(val) => 
                CompressedNum::from_trinary(*val),
        }).collect::<Vec<f32>>(), 
        crate::core::data_types::VectorConversionType::Full)
    }

    fn scalar_compute(
        &mut self, 
        input: &FpgaVector, 
        computation_type: ComputationType
    ) -> Result<FpgaVector, AcceleratorError> {
        let unit_id = self.select_unit()?;
        
        let result_data: Vec<f32> = match computation_type {
            ComputationType::Add => input.data.iter()
                .map(|x| match x {
                    CompressedNum::Full(val) => val + 1.0,
                    CompressedNum::FixedPoint1s31(val) => 
                        CompressedNum::from_fixed_point_1s31(*val) + 1.0,
                    CompressedNum::Trinary(val) => 
                        CompressedNum::from_trinary(*val) + 1.0,
                })
                .collect(),
            ComputationType::Multiply => input.data.iter()
                .map(|x| match x {
                    CompressedNum::Full(val) => val * 2.0,
                    CompressedNum::FixedPoint1s31(val) => 
                        CompressedNum::from_fixed_point_1s31(*val) * 2.0,
                    CompressedNum::Trinary(val) => 
                        CompressedNum::from_trinary(*val) * 2.0,
                })
                .collect(),
            ComputationType::Tanh => input.data.iter()
                .map(|x| match x {
                    CompressedNum::Full(val) => val.tanh(),
                    CompressedNum::FixedPoint1s31(val) => 
                        CompressedNum::from_fixed_point_1s31(*val).tanh(),
                    CompressedNum::Trinary(val) => 
                        CompressedNum::from_trinary(*val).tanh(),
                })
                .collect(),
            ComputationType::ReLU => input.data.iter()
                .map(|x| match x {
                    CompressedNum::Full(val) => val.max(0.0),
                    CompressedNum::FixedPoint1s31(val) => 
                        CompressedNum::from_fixed_point_1s31(*val).max(0.0),
                    CompressedNum::Trinary(val) => 
                        CompressedNum::from_trinary(*val).max(0.0),
                })
                .collect(),
            _ => return Err(AcceleratorError::UnsupportedComputationType(
                format!("Unsupported computation type: {:?}", computation_type)
            )),
        };

        self.release_unit(unit_id);
        FpgaVector::from_numpy(&result_data, 
            crate::core::data_types::VectorConversionType::Full)
    }

    fn select_unit(&mut self) -> Result<usize, AcceleratorError> {
        if let Some(unit_id) = self.available_units.iter().position(|&x| x) {
            self.available_units[unit_id] = false;
            Ok(unit_id)
        } else {
            error!("No available units for computation");
            Err(AcceleratorError::NoAvailableUnits)
        }
    }

    pub fn release_unit(&mut self, unit_id: usize) {
        if unit_id < self.total_units {
            self.available_units[unit_id] = true;
        }
    }
}

impl Default for FpgaAccelerator {
    fn default() -> Self {
        Self::new()
    }
}