#!/usr/bin/env python3
"""
ニューラルネットワークアクセラレータの基本的な使用例
"""

import numpy as np
import nn_accel

def basic_vector_operations():
    """
    基本的なベクトル演算のデモンストレーション
    """
    print("基本的なベクトル演算")
    
    # アクセラレータの初期化
    accel = nn_accel.Accelerator()

    # データの準備
    data1 = np.array([1.0, 2.0, 3.0, 4.0] * 8, dtype=np.float32)
    data2 = np.array([0.5, 1.0, 1.5, 2.0] * 8, dtype=np.float32)

    # ベクトルの作成
    vec1 = nn_accel.Vector.from_numpy(data1)
    vec2 = nn_accel.Vector.from_numpy(data2)

    # ユニットへのバインド
    vec1.bind_to_unit(0)  # ユニット0
    vec2.bind_to_unit(1)  # ユニット1

    print(f"入力ベクトル1: {data1[:4]}...")
    print(f"入力ベクトル2: {data2[:4]}...")

    # コピー演算
    accel.execute("copy", source_unit=1, target_unit=0)
    result = vec1.to_numpy()
    print(f"コピー結果: {result[:4]}...")

    # 加算演算
    vec2 = nn_accel.Vector.from_numpy(data2)  # 元のデータに戻す
    vec2.bind_to_unit(1)
    accel.execute("add", source_unit=1, target_unit=0)
    result = vec1.to_numpy()
    print(f"加算結果: {result[:4]}...")

    # ReLU活性化関数
    vec1 = nn_accel.Vector.from_numpy(np.array([-1.0, -0.5, 0.0, 1.0] * 8, dtype=np.float32))
    vec1.bind_to_unit(0)
    vec1.relu()
    result = vec1.to_numpy()
    print(f"ReLU結果: {result[:4]}...")

    # tanh活性化関数
    vec1 = nn_accel.Vector.from_numpy(np.array([-1.0, -0.5, 0.0, 1.0] * 8, dtype=np.float32))
    vec1.bind_to_unit(0)
    vec1.tanh()
    result = vec1.to_numpy()
    print(f"Tanh結果: {result[:4]}...")

def error_handling_demo():
    """
    エラーハンドリングのデモンストレーション
    """
    print("\nエラーハンドリングデモ")

    try:
        # 不正なサイズのベクトル作成
        nn_accel.Vector.from_numpy(np.ones(15, dtype=np.float32))
    except ValueError as e:
        print(f"期待されるエラー（サイズ不正）: {e}")

    try:
        # 無効なユニットIDの使用
        vec = nn_accel.Vector.from_numpy(np.ones(32, dtype=np.float32))
        vec.bind_to_unit(256)  # 有効範囲外
    except ValueError as e:
        print(f"期待されるエラー（無効なユニットID）: {e}")

def main():
    """
    メイン関数
    """
    print("ニューラルネットワークアクセラレータのデモ\n")
    
    basic_vector_operations()
    error_handling_demo()

    # システムステータスの表示
    accel = nn_accel.Accelerator()
    print("\nシステムステータス:")
    print(accel.status())

if __name__ == "__main__":
    main()
