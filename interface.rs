use std::sync::{Arc, Mutex};
use thiserror::Error;
use fixed::{types::extra::U31, FixedI32};
use futures::future::join_all;
use tokio;
use std::marker::PhantomData;

/// 固定小数点数型の定義（s1.31形式）
pub type Fixed = FixedI32<U31>;

/// 行列要素の値を表現する列挙型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatrixValue {
    Zero = 0b00,
    One = 0b01,
    NegativeOne = 0b11,
}

/// オペコード
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Nop   = 0b0000,
    Load  = 0b0001,
    Store = 0b0010,
    StoreMat = 0b0011,
    Clear = 0b0100,
    ClearMat = 0b0101,
    Add   = 0b0110,
    Sub   = 0b0111,
    Mul   = 0b1000,
    Square = 0b1001,
    Tanh  = 0b1010,
    Relu  = 0b1011,
    Copy  = 0b1100,
}

/// FPGAエラー型
#[derive(Error, Debug)]
pub enum FpgaError {
    #[error("Unit busy, cannot send instruction")]
    UnitBusy,
    #[error("Queue full, cannot add more instructions")]
    QueueFull,
    #[error("Communication error with FPGA")]
    CommunicationError,
    #[error("Invalid unit selected")]
    InvalidUnit,
    #[error("Invalid matrix value")]
    InvalidMatrixValue,
}

/// 行列サイズを表すトレイト
pub trait MatrixDimension {
    const ROWS: usize;
    const COLS: usize;
    
    fn validate() -> bool {
        (Self::ROWS % 16 == 0) && (Self::COLS % 16 == 0)
    }
    
    fn num_row_blocks() -> usize { Self::ROWS / 16 }
    fn num_col_blocks() -> usize { Self::COLS / 16 }
}

/// サイズを指定するための型
pub struct Dim<const R: usize, const C: usize>;

impl<const R: usize, const C: usize> MatrixDimension for Dim<R, C> {
    const ROWS: usize = R;
    const COLS: usize = C;
}

/// 可変サイズ行列
#[derive(Debug, Clone)]
pub struct Matrix<D: MatrixDimension> {
    data: Vec<Vec<MatrixValue>>,
    _phantom: PhantomData<D>,
}

/// 可変サイズベクトル
#[derive(Debug, Clone)]
pub struct Vector<const N: usize> {
    data: Vec<Fixed>,
}

impl<D: MatrixDimension> Matrix<D> {
    pub fn new() -> Self {
        assert!(D::validate(), "Matrix dimensions must be multiples of 16");
        Self {
            data: vec![vec![MatrixValue::Zero; D::COLS]; D::ROWS],
            _phantom: PhantomData,
        }
    }

    /// 16x16の部分行列を取得してバイト列に変換
    fn get_submatrix(&self, row: usize, col: usize) -> [u8; 32] {
        let mut result = [0u8; 32];
        for i in 0..16 {
            for j in 0..8 {
                let base_idx = i * 16 + j * 2;
                let r = row * 16 + i;
                let c = col * 16 + j * 2;
                if r < D::ROWS && c < D::COLS {
                    let value1 = self.data[r][c] as u8;
                    let value2 = self.data[r][c + 1] as u8;
                    result[base_idx / 8] |= value1 << (6 - (base_idx % 8));
                    result[base_idx / 8] |= value2 << (4 - (base_idx % 8));
                }
            }
        }
        result
    }
}

impl<const N: usize> Vector<N> {
    pub fn new() -> Self {
        assert!(N % 16 == 0, "Vector dimension must be multiple of 16");
        Self {
            data: vec![Fixed::ZERO; N],
        }
    }

    /// 16要素の部分ベクトルを取得
    fn get_subvector(&self, index: usize) -> [u8; 64] {
        let mut result = [0u8; 64];
        for i in 0..16 {
            let idx = index * 16 + i;
            if idx < N {
                let value = self.data[idx].to_bits();
                result[i*4..(i+1)*4].copy_from_slice(&value.to_le_bytes());
            }
        }
        result
    }

    /// 部分結果を設定
    fn add_subresult(&mut self, block_row: usize, _block_col: usize, data: [u8; 64]) {
        for i in 0..16 {
            let idx = block_row * 16 + i;
            if idx < N {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&data[i*4..(i+1)*4]);
                let value = Fixed::from_bits(i32::from_le_bytes(bytes));
                self.data[idx] += value;
            }
        }
    }
}

/// FPGAコントローラ
pub struct FpgaController {
    device: Arc<Mutex<Box<dyn FpgaDevice>>>,
}

trait FpgaDevice {
    fn send_instruction(&mut self, unit: u8, opcode: u8, data: [u8; 64]) -> Result<(), FpgaError>;
    fn read_output(&mut self, unit: u8) -> Result<[u8; 64], FpgaError>;
    fn is_unit_busy(&mut self, unit: u8) -> Result<bool, FpgaError>;
}

impl FpgaController {
    pub fn new(device: Box<dyn FpgaDevice>) -> Self {
        Self { 
            device: Arc::new(Mutex::new(device)) 
        }
    }

    pub fn send_instruction(
        &self, 
        unit: u8, 
        opcode: OpCode, 
        data: [u8; 64]
    ) -> Result<(), FpgaError> {
        let mut device = self.device.lock().map_err(|_| FpgaError::CommunicationError)?;
        
        if device.is_unit_busy(unit)? {
            return Err(FpgaError::UnitBusy);
        }

        device.send_instruction(unit, opcode as u8, data)
    }

    pub fn read_output(&self, unit: u8) -> Result<[u8; 64], FpgaError> {
        let mut device = self.device.lock().map_err(|_| FpgaError::CommunicationError)?;
        device.read_output(unit)
    }

    /// 単一ブロックの乗算（非同期）
    async fn multiply_block(
        &self,
        matrix_unit: u8,
        vector_unit: u8,
        result_unit: u8,
        submatrix: [u8; 32],
        subvector: [u8; 64],
    ) -> Result<[u8; 64], FpgaError> {
        self.send_instruction(matrix_unit, OpCode::StoreMat, extend_to_64(submatrix))?;
        self.send_instruction(vector_unit, OpCode::Store, subvector)?;
        self.matrix_multiply(matrix_unit, vector_unit, result_unit)?;
        self.read_output(result_unit)
    }

    /// 一般化された行列ベクトル乗算（並列版）
    pub async fn matrix_multiply_parallel<D: MatrixDimension>(
        &self,
        matrix: &Matrix<D>,
        vector: &Vector<D::COLS>,
    ) -> Result<Vector<D::ROWS>, FpgaError> {
        let mut result = Vector::<D::ROWS>::new();
        
        for block_row in 0..D::num_row_blocks() {
            let mut block_futures = Vec::new();

            for block_col in 0..D::num_col_blocks() {
                let matrix_unit = (block_row * D::num_col_blocks() + block_col) % 16;
                let vector_unit = 16 + matrix_unit;
                let result_unit = 32 + matrix_unit;

                let submatrix = matrix.get_submatrix(block_row, block_col);
                let subvector = vector.get_subvector(block_col);

                let future = self.multiply_block(
                    matrix_unit,
                    vector_unit,
                    result_unit,
                    submatrix,
                    subvector,
                );
                block_futures.push(future);
            }

            let results = join_all(block_futures).await;
            for (col, result_block) in results.into_iter().enumerate() {
                let block_result = result_block?;
                result.add_subresult(block_row, col, block_result);
            }
        }
        
        Ok(result)
    }

    fn matrix_multiply(
        &self,
        matrix_unit: u8,
        vector_unit: u8,
        result_unit: u8
    ) -> Result<(), FpgaError> {
        let mut device = self.device.lock().map_err(|_| FpgaError::CommunicationError)?;
        
        if device.is_unit_busy(matrix_unit)? || 
           device.is_unit_busy(vector_unit)? || 
           device.is_unit_busy(result_unit)? {
            return Err(FpgaError::UnitBusy);
        }

        device.send_instruction(result_unit, OpCode::Mul as u8, [0; 64])
    }
}

// 32バイトのデータを64バイトに拡張するヘルパー関数
fn extend_to_64(data: [u8; 32]) -> [u8; 64] {
    let mut result = [0u8; 64];
    result[..32].copy_from_slice(&data);
    result
}

/// PCIeデバイス実装
struct PcieDevice {
    handle: usize,
}

impl FpgaDevice for PcieDevice {
    fn send_instruction(&mut self, unit: u8, opcode: u8, data: [u8; 64]) -> Result<(), FpgaError> {
        let result = unsafe { 
            fpga_sys_write(self.handle, unit, opcode, &data) 
        };
        
        match result {
            0 => Ok(()),
            _ => Err(FpgaError::CommunicationError)
        }
    }

    fn read_output(&mut self, unit: u8) -> Result<[u8; 64], FpgaError> {
        let mut output = [0u8; 64];
        let result = unsafe { 
            fpga_sys_read(self.handle, unit, &mut output) 
        };
        
        match result {
            0 => Ok(output),
            _ => Err(FpgaError::CommunicationError)
        }
    }

    fn is_unit_busy(&mut self, unit: u8) -> Result<bool, FpgaError> {
        let mut busy_flag = 0u8;
        let result = unsafe { 
            fpga_sys_get_busy_status(self.handle, unit, &mut busy_flag) 
        };
        
        match result {
            0 => Ok(busy_flag != 0),
            _ => Err(FpgaError::CommunicationError)
        }
    }
}

extern "C" {
    fn fpga_sys_write(handle: usize, unit: u8, opcode: u8, data: *const u8) -> i32;
    fn fpga_sys_read(handle: usize, unit: u8, output: *mut u8) -> i32;
    fn fpga_sys_get_busy_status(handle: usize, unit: u8, busy_flag: *mut u8) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fixed_point_calculation() {
        let device = PcieDevice { handle: 0 };
        let controller = FpgaController::new(Box::new(device));

        type Mat16x16 = Dim<16, 16>;
        let mut matrix = Matrix::<Mat16x16>::new();
        for i in 0..16 {
            matrix.data[i][i] = MatrixValue::One;
        }

        let mut vector = Vector::<16>::new();
        for i in 0..16 {
            vector.data[i] = Fixed::from_num(0.5);
        }

        let result = controller.matrix_multiply_parallel(&matrix, &vector)
            .await
            .unwrap();

        for i in 0..16 {
            assert!((result.data[i].to_num::<f64>() - 0.5).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_various_matrix_sizes() {
        let device = PcieDevice { handle: 0 };
        let controller = FpgaController::new(Box::new(device));

        type Mat64x32 = Dim<64, 32>;
        let matrix64x32 = Matrix::<Mat64x32>::new();
        let vector32 = Vector::<32>::new();
        let result64 = controller.matrix_multiply_parallel(&matrix64x32, &vector32)
            .await
            .unwrap();
        assert_eq!(result64.data.len(), 64);

        type Mat32x128 = Dim<32, 128>;
        let matrix32x128 = Matrix::<Mat32x128>::new();
        let vector128 = Vector::<128>::new();
        let result32 = controller.matrix_multiply_parallel(&matrix32x128, &vector128)
            .await
            .unwrap();
        assert_eq!(result32.data.len(), 32);
    }
}