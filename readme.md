# FPGA Accelerator Rust Library

## 概要
FPGAアクセラレータのRustライブラリ。NumPyとの高度な相互運用を提供します。

## 主な機能
- 16次元ブロック分割による大規模行列演算
- 固定小数点(1s.31)および三値化データ型のサポート
- NumPyとのシームレスな変換
- 各種計算タイプ（加算、乗算、Tanh、ReLU）

## インストール
```bash
pip install maturin
maturin develop --release
```

## 使用例

### ベクトル計算
```python
import numpy as np
from fpga_accelerator import FpgaAccelerator

accelerator = FpgaAccelerator()
vector = np.random.randn(16).astype(np.float32)

# Tanh計算
result = accelerator.compute_vector(vector, 'tanh')

# データ型変換
fixed_point_vector = accelerator.convert_vector(vector, 'fixed_point_1s31')
```

### 行列ベクトル乗算
```python
matrix = np.random.randn(64, 128).astype(np.float32)
vector = np.random.randn(128).astype(np.float32)

# 大規模行列ベクトル乗算
result = accelerator.compute_matrix_vector_multiply(matrix, vector)
```

## 開発者向け情報
- Rust 1.70以降
- Python 3.8以降
- NumPy

## ライセンス
MIT License
