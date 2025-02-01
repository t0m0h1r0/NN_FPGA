#!/usr/bin/env python3
"""
ニューラルネットワークアクセラレータの高度な使用例
"""

import numpy as np
import time
import nn_accel

def matrix_multiplication_demo():
    """
    マトリックス乗算のデモンストレーション
    """
    print("\n=== マトリックス乗算デモ ===")
    
    # アクセラレータの初期化
    accel = nn_accel.Accelerator()

    # 大規模なベクトルの作成
    matrix_size = 256  # より大きなサイズ
    vec1 = nn_accel.Vector.from_numpy(
        np.random.rand(matrix_size).astype(np.float32)
    )
    vec2 = nn_accel.Vector.from_numpy(
        np.random.rand(matrix_size).astype(np.float32)
    )

    # ユニットへのバインド
    vec1.bind_to_unit(0)
    vec2.bind_to_unit(1)

    # 開始時間の記録
    start_time = time.time()

    # 演算の連続実行
    operations = [
        ("copy", 1, 0),   # コピー
        ("add", 1, 0),    # 加算
        ("relu", 0, 0)    # ReLU活性化
    ]

    for op, source, target in operations:
        accel.execute(op, source_unit=source, target_unit=target)

    # 実行時間の表示
    print(f"演算完了時間: {time.time() - start_time:.4f}秒")

    # 結果の表示
    result = vec1.to_numpy()
    print("結果の最初の10要素:")
    print(result[:10])

def parallel_processing_demo():
    """
    並列処理のデモンストレーション
    """
    print("\n=== 並列処理デモ ===")
    
    # アクセラレータの初期化
    accel = nn_accel.Accelerator()

    # 複数のベクトルを作成
    vectors = []
    for i in range(4):
        # ランダムなデータで初期化
        data = np.random.rand(64).astype(np.float32)
        vec = nn_accel.Vector.from_numpy(data)
        vec.bind_to_unit(i)
        vectors.append(vec)

    print("4つのベクトルを作成し、異なるユニットに割り当て")

    # 開始時間の記録
    start_time = time.time()

    # 連続した演算の実行
    for i in range(1, 4):
        # 前のベクトルから現在のベクトルへの加算
        accel.execute("add", source_unit=i-1, target_unit=i)

        # 最後の2つのベクトルに異なる活性化関数を適用
        if i == 2:
            vectors[i].relu()
        elif i == 3:
            vectors[i].tanh()

    # 実行時間の表示
    print(f"演算完了時間: {time.time() - start_time:.4f}秒")

    # 結果の表示
    for i, vec in enumerate(vectors):
        result = vec.to_numpy()
        print(f"ベクトル {i} の最初の10要素:")
        print(result[:10])

def main():
    """
    デモのメイン関数
    """
    print("ニューラルネットワークアクセラレータ - 高度なデモ")
    
    matrix_multiplication_demo()
    parallel_processing_demo()

    # システムステータスの表示
    accel = nn_accel.Accelerator()
    print("\nシステムステータス:")
    print(accel.status())

if __name__ == "__main__":
    main()
