# FPGA高速演算アクセラレータ

## 概要
このライブラリは、FPGAを活用した高速な行列・ベクトル演算アクセラレータを提供します。NumPyとの優れた相互運用性を備え、機械学習や数値計算のワークロードを効率的に処理できます。

## 主な特長

- **高速な行列演算**: 16次元ブロック分割による大規模行列演算の最適化
- **柔軟なデータ型**: 完全精度、固定小数点(1s.31)、三値化方式をサポート
- **豊富な演算機能**: 加算、乗算、Tanh、ReLU等の基本演算に対応
- **NumPyとの連携**: NumPy配列との円滑なデータ変換が可能
- **メモリ効率**: 最適化された行列表現による省メモリ設計

## システム要件

- Python 3.8以上
- Rust 1.70以上
- NumPy
- FPGAボード（対応機種については後述）

## インストール方法

1. Rustツールチェインのインストール（未導入の場合）
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Maturinのインストール
```bash
pip install maturin
```

3. ライブラリのビルドとインストール
```bash
git clone [リポジトリURL]
cd fpga-accelerator
maturin develop --release
```

## 基本的な使い方

### 1. ベクトル演算

```python
import numpy as np
from fpga_accelerator import FpgaAccelerator

# アクセラレータの初期化
accelerator = FpgaAccelerator()

# 16の倍数サイズのベクトルを作成（必須条件）
vector = np.random.randn(16).astype(np.float32)

# 各種演算の実行
result_tanh = accelerator.compute_vector(vector, 'tanh')
result_relu = accelerator.compute_vector(vector, 'relu')
```

### 2. 行列ベクトル乗算

```python
# 64x128の行列と128次元ベクトルを用意
matrix = np.random.randn(64, 128).astype(np.float32)
vector = np.random.randn(128).astype(np.float32)

# 行列の準備（キャッシュ効果により複数回の演算が高速化）
accelerator.prepare_matrix(matrix)

# 行列ベクトル乗算の実行
result = accelerator.compute_with_prepared_matrix(vector)
```

### 3. データ型変換

```python
# 三値化変換（-1, 0, 1の離散値に変換）
trinary_vector = accelerator.convert_vector(vector, 'trinary')

# 固定小数点変換（1s.31形式）
fixed_point_vector = accelerator.convert_vector(vector, 'fixed_point_1s31')
```

## 性能最適化のヒント

1. **ベクトルサイズ**: すべての入力ベクトルは16の倍数サイズである必要があります
2. **行列の前処理**: 同じ行列を用いた複数回の演算時は`prepare_matrix`を活用
3. **データ型の選択**: 
   - 精度が重要な場合: 完全精度モード
   - メモリ効率重視: 三値化モード
   - バランス重視: 固定小数点モード

## 制限事項

- 入力ベクトル・行列のサイズは16の倍数である必要があります
- 行列サイズは64×128までサポート
- 単精度浮動小数点数（float32）のみ対応

## トラブルシューティング

1. **InvalidDimension エラー**
   - 入力サイズが16の倍数でない場合に発生
   - パディングによる調整を検討

2. **MatrixNotPrepared エラー**
   - `compute_with_prepared_matrix`前に`prepare_matrix`が必要

3. **メモリ不足エラー**
   - 三値化モードの使用を検討
   - バッチサイズの調整

## ライセンス

MIT License

## 開発者向け情報

- コードスタイル: `cargo fmt`に準拠
- テスト実行: `cargo test`
- ドキュメント生成: `cargo doc`

## 対応FPGA

- Xilinx Alveo U200/U250
- Xilinx Ultrascale+シリーズ
- Intel Stratix 10