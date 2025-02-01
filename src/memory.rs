//! メモリ管理モジュール
//!
//! 安全で効率的なメモリブロック管理を提供します。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Serialize, Deserialize};

use crate::types::{
    UnitId, 
    constants::{VECTOR_WIDTH, BLOCK_SIZE}
};
use crate::error::{Result, DomainError};

/// メモリブロックID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(u64);

/// メモリブロックの状態
#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockStatus {
    /// 空き状態
    Free,
    /// 割り当て済み
    Allocated {
        /// 割り当てられたユニット
        unit: UnitId,
    },
    /// ロック中
    Locked {
        /// ロックしているユニット
        unit: UnitId,
        /// ロック理由
        reason: LockReason,
    },
}

/// ロックの理由
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LockReason {
    /// 読み取り中
    Reading,
    /// 書き込み中
    Writing,
    /// データ転送中
    Transferring,
}

/// メモリ使用状況
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// 総メモリサイズ
    pub total_size: usize,
    /// 使用中メモリサイズ
    pub used_size: usize,
    /// ブロックサイズ
    pub block_size: usize,
    /// 総ブロック数
    pub total_blocks: usize,
    /// 使用中ブロック数
    pub used_blocks: usize,
    /// ロック中ブロック数
    pub locked_blocks: usize,
}

/// メモリマネージャ
pub struct MemoryManager {
    /// 総メモリサイズ
    total_size: usize,
    /// ブロックサイズ
    block_size: usize,
    /// メモリブロック管理マップ
    blocks: Arc<RwLock<HashMap<BlockId, BlockStatus>>>,
    /// 次のブロックID生成用
    next_block_id: Arc<Mutex<u64>>,
    /// 解放待ちブロックキュー
    free_queue: Arc<Mutex<VecDeque<BlockId>>>,
}

impl MemoryManager {
    /// 新規メモリマネージャの作成
    ///
    /// # Errors
    /// ブロックサイズが不正な場合はエラーを返します
    pub fn new(total_size: usize, block_size: usize) -> Result<Self> {
        // ブロックサイズのバリデーション
        if block_size == 0 || block_size % 16 != 0 {
            return Err(DomainError::memory_error(
                "ブロックサイズは16の倍数である必要があります"
            ));
        }

        Ok(Self {
            total_size,
            block_size,
            blocks: Arc::new(RwLock::new(HashMap::new())),
            next_block_id: Arc::new(Mutex::new(0)),
            free_queue: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    /// メモリブロックの確保
    ///
    /// # Errors
    /// メモリ不足や不正なサイズの場合はエラーを返します
    pub async fn allocate(&self, size: usize, unit: UnitId) -> Result<BlockId> {
        // サイズのバリデーション
        if size == 0 || size % self.block_size != 0 {
            return Err(DomainError::memory_error(
                "サイズはブロックサイズの倍数である必要があります"
            ));
        }

        let num_blocks = size / self.block_size;
        let mut blocks = self.blocks.write().await;

        // メモリ空き容量のチェック
        let used_blocks = blocks.len();
        let max_blocks = self.total_size / self.block_size;
        if used_blocks + num_blocks > max_blocks {
            return Err(DomainError::resource_error(
                "メモリ",
                format!(
                    "メモリ不足: {} ブロック要求、利用可能は {} ブロック", 
                    num_blocks, 
                    max_blocks - used_blocks
                )
            ));
        }

        // 解放待ちキューの確認
        let mut free_queue = self.free_queue.lock().await;
        let block_id = if let Some(id) = free_queue.pop_front() {
            id
        } else {
            // 新規ブロックID生成
            let mut next_id = self.next_block_id.lock().await;
            let id = BlockId(*next_id);
            *next_id += 1;
            id
        };

        // ブロックステータスの更新
        blocks.insert(block_id, BlockStatus::Allocated { unit });

        Ok(block_id)
    }

    /// メモリブロックの解放
    ///
    /// # Errors
    /// ブロックが見つからないか、ロック中の場合はエラーを返します
    pub async fn free(&self, block_id: BlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.remove(&block_id) {
            None => Err(DomainError::memory_error("ブロックが見つかりません")),
            Some(BlockStatus::Locked { .. }) => Err(DomainError::resource_error(
                "メモリブロック",
                "ブロックはロック中です"
            )),
            Some(_) => {
                // 解放待ちキューに追加
                let mut free_queue = self.free_queue.lock().await;
                free_queue.push_back(block_id);
                Ok(())
            }
        }
    }

    /// メモリブロックのロック
    ///
    /// # Errors
    /// ブロックが見つからないか、すでにロックされている場合はエラーを返します
    pub async fn lock(
        &self,
        block_id: BlockId,
        unit: UnitId,
        reason: LockReason,
    ) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.get(&block_id) {
            None => Err(DomainError::memory_error("ブロックが見つかりません")),
            Some(BlockStatus::Locked { .. }) => Err(DomainError::resource_error(
                "メモリブロック",
                "ブロックは既にロックされています"
            )),
            Some(_) => {
                blocks.insert(block_id, BlockStatus::Locked { unit, reason });
                Ok(())
            }
        }
    }

    /// メモリブロックのロック解除
    ///
    /// # Errors
    /// ブロックが見つからない場合はエラーを返します
    pub async fn unlock(&self, block_id: BlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;
        
        match blocks.get(&block_id) {
            None => Err(DomainError::memory_error("ブロックが見つかりません")),
            Some(BlockStatus::Locked { unit, .. }) => {
                blocks.insert(block_id, BlockStatus::Allocated { unit: *unit });
                Ok(())
            },
            Some(_) => Ok(()),
        }
    }

    /// メモリ使用状況の取得
    pub async fn usage(&self) -> MemoryUsage {
        let blocks = self.blocks.read().await;
        let used_blocks = blocks.len();
        let locked_blocks = blocks.values()
            .filter(|status| matches!(status, BlockStatus::Locked { .. }))
            .count();

        MemoryUsage {
            total_size: self.total_size,
            used_size: used_blocks * self.block_size,
            block_size: self.block_size,
            total_blocks: self.total_size / self.block_size,
            used_blocks,
            locked_blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_memory_allocation() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let manager = MemoryManager::new(1024, 16).unwrap();
            let unit = UnitId::new(0).unwrap();

            // 正常な確保
            let block_id = manager.allocate(32, unit).await.unwrap();
            
            // ブロックのロック
            manager.lock(block_id, unit, LockReason::Writing).await.unwrap();
            
            // ロック中ブロックの解放テスト
            assert!(manager.free(block_id).await.is_err());
            
            // ロック解除と解放
            manager.unlock(block_id).await.unwrap();
            manager.free(block_id).await.unwrap();
        });
    }

    #[test]
    fn test_memory_limits() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let manager = MemoryManager::new(32, 16).unwrap();
            let unit = UnitId::new(0).unwrap();

            // メモリ超過の確保テスト
            assert!(manager.allocate(64, unit).await.is_err());

            // 不正なブロックサイズテスト
            assert!(manager.allocate(10, unit).await.is_err());
        });
    }

    #[test]
    fn test_memory_usage() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let manager = MemoryManager::new(1024, 16).unwrap();
            let unit = UnitId::new(0).unwrap();

            // 初期状態の確認
            let usage = manager.usage().await;
            assert_eq!(usage.total_size, 1024);
            assert_eq!(usage.used_size, 0);
            assert_eq!(usage.block_size, 16);

            // メモリ確保とロック
            let block_id = manager.allocate(32, unit).await.unwrap();
            manager.lock(block_id, unit, LockReason::Reading).await.unwrap();

            // 使用状況の確認
            let usage = manager.usage().await;
            assert_eq!(usage.used_blocks, 1);
            assert_eq!(usage.locked_blocks, 1);
        });
    }
}