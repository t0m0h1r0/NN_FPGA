//! ベクトル計算モジュール
//!
//! 高性能で安全なベクトル演算を提供します。

use std::ops::{Add, Mul, Index, IndexMut};
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use num_traits::{Float, Zero, One};

use crate::types::{
    constants::{BLOCK_SIZE, VECTOR_WIDTH},
    UnitId, 
    ActivationFunction,
};
use crate::error::{Result, DomainError};

/// ベクトルブロック
///
/// メモリ効率と並列処理を考慮したベクトルブロック実装
#[derive(Debug, Clone)]
struct VectorBlock<T> 
where 
    T: Float + Serialize + for<'de> Deserialize<'de>,
{
    /// データストレージ
    data: Box<[T; BLOCK_SIZE]>,
}

impl<T> VectorBlock<T> 
where 
    T: Float + Serialize + for<'de> Deserialize<'de>,
{
    /// 新規ブロックの作成
    fn new() -> Self {
        Self {
            data: Box::new([T::zero(); BLOCK_SIZE]),
        }
    }

    /// インデックスによる値の取得
    fn get(&self, index: usize) -> Option<T> {
        self.data.get(index).copied()
    }

    /// インデックスによる値の設定
    fn set(&mut self, index: usize, value: T) -> bool {
        if let Some(elem) = self.data.get_mut(index) {
            *elem = value;
            true
        } else {
            false
        }
    }
}

/// 高性能ベクトル実装
#[derive(Debug)]
pub struct Vector<T> 
where 
    T: Float + Serialize + for<'de> Deserialize<'de>,
{
    /// ベクトルサイズ
    size: usize,
    /// データブロック
    blocks: Vec<Arc<RwLock<VectorBlock<T>>>>,
    /// 割り当てられたユニットID
    unit_id: Option<UnitId>,
}

impl<T> Vector<T> 
where 
    T: Float + Serialize + for<'de> Deserialize<'de>,
{
    /// 新規ベクトルの作成
    ///
    /// # Errors
    /// サイズが不正な場合はエラーを返します
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 || size % BLOCK_SIZE != 0 {
            return Err(DomainError::memory_error(
                format!("ベクトルサイズは{}の倍数である必要があります", BLOCK_SIZE)
            ));
        }

        let num_blocks = size / BLOCK_SIZE;
        let blocks = (0..num_blocks)
            .map(|_| Arc::new(RwLock::new(VectorBlock::new())))
            .collect();

        Ok(Self {
            size,
            blocks,
            unit_id: None,
        })
    }

    /// ユニットへのバインド
    pub async fn bind_to_unit(&mut self, unit: UnitId) -> Result<()> {
        self.unit_id = Some(unit);
        Ok(())
    }

    /// バインドされたユニットIDの取得
    pub fn unit_id(&self) -> Option<UnitId> {
        self.unit_id
    }

    /// ベクトルサイズの取得
    pub fn size(&self) -> usize {
        self.size
    }

    /// 値の取得
    pub async fn get(&self, index: usize) -> Result<T> {
        let (block_idx, inner_idx) = self.validate_index(index)?;
        let block = self.blocks[block_idx].read().await;
        block.get(inner_idx).ok_or_else(|| 
            DomainError::memory_error("インデックスが範囲外です")
        )
    }

    /// 値の設定
    pub async fn set(&mut self, index: usize, value: T) -> Result<()> {
        let (block_idx, inner_idx) = self.validate_index(index)?;
        let mut block = self.blocks[block_idx].write().await;
        if !block.set(inner_idx, value) {
            return Err(DomainError::memory_error("インデックスが範囲外です"));
        }
        Ok(())
    }

    /// インデックスの検証
    fn validate_index(&self, index: usize) -> Result<(usize, usize)> {
        if index >= self.size {
            return Err(DomainError::memory_error(
                format!("インデックス {} は範囲外です", index)
            ));
        }
        Ok((index / BLOCK_SIZE, index % BLOCK_SIZE))
    }

    /// 活性化関数の適用
    pub async fn apply_activation(&mut self, function: ActivationFunction) -> Result<()> {
        for block in &self.blocks {
            let mut block = block.write().await;
            for value in block.data.iter_mut() {
                *value = match function {
                    ActivationFunction::ReLU => value.max(T::zero()),
                    ActivationFunction::Tanh => value.tanh(),
                    ActivationFunction::Sigmoid => {
                        T::one() / (T::one() + (-*value).exp())
                    }
                };
            }
        }
        Ok(())
    }

    /// ベクトル間の加算
    pub async fn add(&mut self, other: &Self) -> Result<()> {
        if self.size != other.size {
            return Err(DomainError::memory_error("ベクトルサイズが一致しません"));
        }

        for (self_block, other_block) in self.blocks.iter().zip(other.blocks.iter()) {
            let mut self_guard = self_block.write().await;
            let other_guard = other_block.read().await;

            for (s, o) in self_guard.data.iter_mut().zip(other_guard.data.iter()) {
                *s = *s + *o;
            }
        }

        Ok(())
    }
}

/// デバッグ表示のための実装
impl<T> fmt::Display for Vector<T> 
where 
    T: Float + Serialize + for<'de> Deserialize<'de> + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vector(size={}, unit={:?})", 
            self.size, 
            self.unit_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_vector_creation() {
        // 正常なサイズ
        assert!(Vector::<f32>::new(16).is_ok());
        assert!(Vector::<f64>::new(32).is_ok());

        // 不正なサイズ
        assert!(Vector::<f32>::new(0).is_err());
        assert!(Vector::<f32>::new(15).is_err());
    }

    #[test]
    fn test_vector_operations() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut vec = Vector::<f32>::new(32).unwrap();
            
            // 値の設定と取得
            vec.set(0, 1.0).await.unwrap();
            assert_eq!(vec.get(0).await.unwrap(), 1.0);

            // バインド
            let unit = UnitId::new(0).unwrap();
            vec.bind_to_unit(unit).await.unwrap();
            assert_eq!(vec.unit_id(), Some(unit));

            // 活性化関数
            vec.set(1, -1.0).await.unwrap();
            vec.apply_activation(ActivationFunction::ReLU).await.unwrap();
            assert_eq!(vec.get(1).await.unwrap(), 0.0);
        });
    }

    #[test]
    fn test_vector_addition() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut vec1 = Vector::<f32>::new(32).unwrap();
            let mut vec2 = Vector::<f32>::new(32).unwrap();

            // 値の設定
            for i in 0..32 {
                vec1.set(i, i as f32).await.unwrap();
                vec2.set(i, (i * 2) as f32).await.unwrap();
            }

            // 加算
            vec1.add(&vec2).await.unwrap();

            // 結果の検証
            for i in 0..32 {
                let expected = (i + i * 2) as f32;
                assert_eq!(vec1.get(i).await.unwrap(), expected);
            }
        });
    }
}