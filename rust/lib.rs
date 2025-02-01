// lib.rs

mod types;
mod error;
mod block;
mod store;
mod vector;
mod matrix;

pub use types::{Activation, BLOCK_SIZE, BlockIndex, MatrixIndex};
pub use error::{Result, NNError};
pub use vector::Vector;
pub use matrix::Matrix;
pub use store::Store;

/// ニューラルネットワークアクセラレータライブラリ
/// 
/// このライブラリは16次元ブロックを基本単位とする大規模な行列演算を提供します。
///
/// # 特徴
///
/// - 16の倍数サイズの行列・ベクトル演算
/// - ブロック単位の並列処理
/// - 効率的なメモリ管理
/// - スレッドセーフな行列ストレージ
///
/// # 例
///
/// ```
/// use nn_accel::{Matrix, Vector, Store, MatrixIndex, Activation};
///
/// // ストレージの初期化
/// let store = Store::new();
///
/// // 32x32行列の作成と初期化
/// let mut matrix = Matrix::new(32, 32).unwrap();
/// matrix.set(MatrixIndex::new(0, 0), 1.0).unwrap();
///
/// // 32次元ベクトルの作成
/// let mut vector = Vector::new(32).unwrap();
/// vector.set(0, 2.0).unwrap();
///
/// // 行列の保存
/// matrix.store("weights", &store).unwrap();
///
/// // 行列ベクトル積の計算
/// let result = matrix.multiply(&vector).unwrap();
///
/// // アクティベーション関数の適用
/// let activated = result.apply_activation(Activation::ReLU);
/// ```