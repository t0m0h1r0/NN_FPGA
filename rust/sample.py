# examples/basic_operations.py
import numpy as np
from nn_accel import Accelerator, Vector

def test_basic_operations():
    """基本的な演算操作のデモ"""
    print("基本的な演算操作のテスト開始...")

    # アクセラレータの初期化
    accel = Accelerator()
    accel.initialize()

    # テストデータの作成
    data1 = np.array([1.0, 2.0, 3.0, 4.0] * 8, dtype=np.float32)  # 32要素
    data2 = np.array([0.5, 1.0, 1.5, 2.0] * 8, dtype=np.float32)  # 32要素

    # ベクトルの作成とユニットへの割り当て
    vec1 = Vector.from_numpy(data1)
    vec2 = Vector.from_numpy(data2)

    vec1.bind_to_unit(0)  # ユニット0に割り当て
    vec2.bind_to_unit(1)  # ユニット1に割り当て

    print(f"入力ベクトル1: {data1[:4]}...")
    print(f"入力ベクトル2: {data2[:4]}...")

    # コピー操作
    accel.copy(vec1, vec2)
    result = vec2.to_numpy()
    print(f"コピー結果: {result[:4]}...")

    # 加算操作
    vec2 = Vector.from_numpy(data2)  # 元のデータに戻す
    vec2.bind_to_unit(1)
    accel.add(vec1, vec2)
    result = vec2.to_numpy()
    print(f"加算結果: {result[:4]}...")

    # ReLU活性化関数
    vec1 = Vector.from_numpy(np.array([-1.0, -0.5, 0.0, 1.0] * 8, dtype=np.float32))
    vec1.bind_to_unit(0)
    accel.relu(vec1)
    result = vec1.to_numpy()
    print(f"ReLU結果: {result[:4]}...")

    # tanh活性化関数
    vec1 = Vector.from_numpy(np.array([-1.0, -0.5, 0.0, 1.0] * 8, dtype=np.float32))
    vec1.bind_to_unit(0)
    accel.tanh(vec1)
    result = vec1.to_numpy()
    print(f"tanh結果: {result[:4]}...")

# examples/parallel_processing.py
def test_parallel_processing():
    """並列処理のデモ"""
    print("\n並列処理のテスト開始...")

    # アクセラレータの初期化
    accel = Accelerator()
    accel.initialize()

    # 複数のベクトルを作成
    vectors = []
    for i in range(4):
        data = np.ones(32, dtype=np.float32) * (i + 1)
        vec = Vector.from_numpy(data)
        vec.bind_to_unit(i)
        vectors.append(vec)

    print("4つのベクトルを作成し、別々のユニットに割り当て")

    # 順次加算処理
    for i in range(1, 4):
        accel.add(vectors[i-1], vectors[i])
        result = vectors[i].to_numpy()
        print(f"ユニット{i}の加算結果: {result[:4]}...")

# examples/error_handling.py
def test_error_handling():
    """エラーハンドリングのデモ"""
    print("\nエラーハンドリングのテスト開始...")

    accel = Accelerator()
    accel.initialize()

    try:
        # 不正なサイズのベクトルを作成
        Vector.from_numpy(np.ones(15, dtype=np.float32))  # 16の倍数でない
    except ValueError as e:
        print(f"期待されたエラー（サイズ不正）: {e}")

    try:
        # 無効なユニットIDを使用
        vec = Vector.from_numpy(np.ones(32, dtype=np.float32))
        vec.bind_to_unit(256)  # 有効範囲外
    except ValueError as e:
        print(f"期待されたエラー（無効なユニットID）: {e}")

    try:
        # バインドされていないベクトルで演算を実行
        vec1 = Vector.from_numpy(np.ones(32, dtype=np.float32))
        vec2 = Vector.from_numpy(np.ones(32, dtype=np.float32))
        accel.add(vec1, vec2)  # バインドされていない
    except RuntimeError as e:
        print(f"期待されたエラー（未バインド）: {e}")

if __name__ == "__main__":
    print("FPGAニューラルネットワークアクセラレータのデモ\n")
    
    print("===== 基本演算テスト =====")
    test_basic_operations()
    
    print("\n===== 並列処理テスト =====")
    test_parallel_processing()
    
    print("\n===== エラー処理テスト =====")
    test_error_handling()