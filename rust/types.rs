// types.rs

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Nop,
    Load,
    Store,
    Compute,
    Copy,    // 追加：ベクトルコピー操作
    AddVec,  // 追加：ベクトル加算操作
}

#[derive(Debug, Clone, Copy)]
pub enum Activation {
    Tanh,
    ReLU,
}

// ブロックサイズは16x16のまま
pub const BLOCK_SIZE: usize = 16;
pub const UNIT_COUNT: usize = 256;

#[derive(Debug, Clone, Copy)]
pub struct UnitConfig {
    pub target_unit: usize,   // ターゲットユニットID
    pub source_unit: usize,   // 追加：ソースユニットID
    pub operation: Operation,
    pub activation: Option<Activation>,
}

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

#[derive(Debug, Clone, Copy)]
pub struct MatrixIndex {
    pub row: usize,
    pub col: usize,
}

impl MatrixIndex {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

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