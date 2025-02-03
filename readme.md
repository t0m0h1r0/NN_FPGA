# FPGA高速演算アクセラレータ

## 概要
このプロジェクトは、FPGAを活用した高速な行列・ベクトル演算アクセラレータを提供します。VerilogによるFPGA IPとRustによるドライバ、そしてPythonインターフェースを組み合わせることで、高性能な数値計算を実現します。

## 主な特徴

- **高速な行列演算**: 16×16ブロックによる最適化された行列計算
- **ハードウェアアクセラレーション**: FPGA上の専用IPによる高速演算
- **多様なデータ型**: 
  - 完全精度（32ビット浮動小数点）
  - 固定小数点（1s.31形式）
  - 三値化（-1, 0, 1）
- **豊富な演算機能**:
  - 行列ベクトル乗算
  - ベクトル加算/乗算
  - 活性化関数（ReLU, Tanh）
- **Pythonとの連携**: NumPy配列との円滑なデータ変換

## システム要件

- Python 3.8以上
- Rust 1.70以上
- NumPy
- 対応FPGAボード
  - Xilinx Alveo U200/U250
  - Xilinx Ultrascale+シリーズ
  - Intel Stratix 10

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

## 使用例

### 1. 基本的な行列ベクトル乗算

```python
import numpy as np
from fpga_accelerator import FpgaAccelerator

# アクセラレータの初期化
accelerator = FpgaAccelerator()

# テストデータの作成（サイズは16の倍数である必要があります）
matrix = np.random.randn(64, 128).astype(np.float32)
vector = np.random.randn(128).astype(np.float32)

# 行列の準備（キャッシュ効果のため、複数回の演算で高速化）
accelerator.prepare_matrix(matrix)

# 行列ベクトル乗算の実行
result = accelerator.compute_with_prepared_matrix(vector)
```

### 2. ベクトル演算と共有メモリ操作

```python
# ベクトルの作成
vector = np.random.randn(16).astype(np.float32)

# ベクトルをユニット1の共有メモリに送信
accelerator.push_vector_to_memory(vector, unit_id=1)

# 別のユニットで共有メモリからデータを取得
result = accelerator.pull_vector_from_memory(unit_id=1)

# 各種演算の実行
result_add = accelerator.compute_vector(vector, 'add')    # ベクトル + 1
result_mul = accelerator.compute_vector(vector, 'mul')    # ベクトル * 2
result_tanh = accelerator.compute_vector(vector, 'tanh')  # tanh(ベクトル)
result_relu = accelerator.compute_vector(vector, 'relu')  # ReLU(ベクトル)
```

### 3. データ型変換

```python
# 三値化変換
trinary_vector = accelerator.convert_vector(vector, 'trinary')
trinary_matrix = accelerator.convert_matrix(matrix, 'trinary')

# 固定小数点変換
fixed_vector = accelerator.convert_vector(vector, 'fixed_point_1s31')
fixed_matrix = accelerator.convert_matrix(matrix, 'fixed_point_1s31')
```

## 性能最適化のポイント

1. **データサイズ**
   - すべての入力は16の倍数サイズである必要があります
   - 推奨サイズ: 16, 32, 64, 128, 256

2. **メモリ効率**
   - `prepare_matrix`を使用して行列を事前にキャッシュ
   - 同じ行列を使用する場合は再利用を推奨

3. **データ型の選択**
   - 高精度が必要な場合: 完全精度モード
   - メモリ効率重視: 三値化モード
   - バランス重視: 固定小数点モード

## ベンチマークの実行

性能評価用のベンチマークスクリプトを提供しています：

```bash
python examples/benchmark.py
```

これにより以下の項目が測定されます：
- 行列ベクトル乗算の性能（FPGA vs NumPy）
- 各種ベクトル演算の実行時間
- データ型変換のオーバーヘッド

結果は`benchmark_results`ディレクトリに保存され、以下が生成されます：
- CSVファイル形式の詳細な測定データ
- 性能比較グラフ（実行時間とGFLOPS）

## FPGAの設定

1. **ビットストリームの生成**
```bash
cd fpga/
make
```

2. **FPGAへの書き込み**
```bash
# Xilinxボードの場合
vivado_lab -mode batch -source program_fpga.tcl

# Intelボードの場合
quartus_pgm -c 1 -m JTAG -o "p;output_files/fpga_accelerator.sof"
```

## トラブルシューティング

1. **サイズエラー**
   - 入力サイズが16の倍数でない場合に発生
   - データのパディングを検討してください

2. **メモリエラー**
   - 大きな行列で発生する場合は三値化モードを使用
   - ブロックサイズの調整を検討

3. **性能の問題**
   - `prepare_matrix`を使用しているか確認
   - データ型の最適化を検討
   - FPGAの動作周波数を確認

## ライセンス

MIT License

## 開発者向け情報

- テストの実行: `cargo test`
- ドキュメント生成: `cargo doc --no-deps --open`
- Pythonテスト: `python -m pytest tests/`
- コードフォーマット: `cargo fmt`

## 貢献について

バグ報告や機能リクエストは、GitHubのIssueトラッカーをご利用ください。
プルリクエストも歓迎いたします。

## 参考文献

1. FPGA IPの設計資料: `docs/fpga_design.pdf`
2. インターフェース仕様: `docs/interface_spec.pdf`
3. 性能評価レポート: `docs/performance_report.pdf`