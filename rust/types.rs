// types.rs

/// ブロックのサイズ（16x16）
pub const BLOCK_SIZE: usize = 16;

/// アクティベーション関数の種類
#[derive(Debug, Clone, Copy)]
pub enum Activation {
    /// 双曲線正接関数
    Tanh,
    /// 正規化線形関数
    ReLU,
}

/// ブロックのインデックス
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BlockIndex {
    pub row: usize,
    pub col: usize,
}

impl BlockIndex {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// 行と列のインデックス
#[derive(Debug, Clone, Copy)]
pub struct MatrixIndex {
    pub row: usize,
    pub col: usize,
}

impl MatrixIndex {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// ブロックインデックスとブロック内インデックスに分解
    pub fn to_block_indices(&self) -> (BlockIndex, MatrixIndex) {
        let block = BlockIndex::new(
            self.row / BLOCK_SIZE,
            self.col / BLOCK_SIZE,
        );
        let inner = MatrixIndex::new(
            self.row % BLOCK_SIZE,
            self.col % BLOCK_SIZE,
        );
        (block, inner)
    }
}