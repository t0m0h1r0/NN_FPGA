// store.rs

use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::block::MatrixBlock;
use crate::types::BlockIndex;
use crate::error::{Result, NNError};

/// 行列ブロックのストレージ管理
#[derive(Debug, Default)]
pub struct Store {
    blocks: RwLock<HashMap<String, Arc<MatrixBlock>>>,
}

impl Store {
    /// 新しいストレージを作成
    pub fn new() -> Self {
        Self {
            blocks: RwLock::new(HashMap::new()),
        }
    }

    /// ブロックを保存
    pub fn store(&self, name: &str, block: MatrixBlock) -> Result<()> {
        let mut storage = self.blocks.write().map_err(|e| 
            NNError::Storage(format!("Lock error: {}", e)))?;
        storage.insert(name.to_string(), Arc::new(block));
        Ok(())
    }

    /// ブロックを取得
    pub fn get(&self, name: &str) -> Result<Arc<MatrixBlock>> {
        let storage = self.blocks.read().map_err(|e|
            NNError::Storage(format!("Lock error: {}", e)))?;
        storage.get(name).cloned().ok_or_else(|| 
            NNError::NotFound(format!("Block '{}' not found", name)))
    }

    /// ブロックを削除
    pub fn remove(&self, name: &str) -> Result<()> {
        let mut storage = self.blocks.write().map_err(|e|
            NNError::Storage(format!("Lock error: {}", e)))?;
        storage.remove(name).ok_or_else(|| 
            NNError::NotFound(format!("Block '{}' not found", name)))?;
        Ok(())
    }

    /// ブロック名の一覧を取得
    pub fn list(&self) -> Result<Vec<String>> {
        let storage = self.blocks.read().map_err(|e|
            NNError::Storage(format!("Lock error: {}", e)))?;
        Ok(storage.keys().cloned().collect())
    }
}

/// ブロック名の生成
pub fn make_block_name(base_name: &str, index: BlockIndex) -> String {
    format!("{}_{:04x}_{:04x}", base_name, index.row, index.col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_operations() {
        let store = Store::new();
        let block = MatrixBlock::new();
        
        // ブロックの保存と取得
        store.store("test", block.clone()).unwrap();
        let retrieved = store.get("test").unwrap();
        
        // 存在しないブロックの取得
        assert!(store.get("nonexistent").is_err());
        
        // ブロック一覧の取得
        let blocks = store.list().unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks.contains(&"test".to_string()));
    }

    #[test]
    fn test_block_name_generation() {
        let index = BlockIndex::new(1, 2);
        let name = make_block_name("matrix", index);
        assert_eq!(name, "matrix_0001_0002");
    }
}