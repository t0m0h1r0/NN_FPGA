# sample.py
import numpy as np
from nn_accel import NNVector

def test_vector_operations():
    # 2つのベクトルを作成
    vec1 = NNVector.from_numpy(np.ones(32, dtype=np.float32))
    vec2 = NNVector.from_numpy(np.ones(32, dtype=np.float32) * 2)

    # ユニットへのバインド
    vec1.bind_to_unit(0)  # ユニット0に割り当て
    vec2.bind_to_unit(1)  # ユニット1に割り当て

    # ユニット間のデータ転送
    vec2.copy_from_unit(0)  # ユニット0からユニット1へコピー
    
    # ベクトル加算
    vec2.add_from_unit(0)   # ユニット0のデータを加算

    # 結果の確認
    result = vec2.to_numpy()
    print("Result shape:", result.shape)
    print("First few values:", result[:5])

def test_parallel_operations():
    # 複数ベクトルの並列処理
    vectors = []
    for i in range(4):
        vec = NNVector.from_numpy(np.ones(32, dtype=np.float32) * (i + 1))
        vec.bind_to_unit(i)
        vectors.append(vec)

    # ベクトル間の演算
    for i in range(1, 4):
        vectors[i].add_from_unit(i - 1)  # 前のユニットのデータを加算

    # 結果の表示
    for i, vec in enumerate(vectors):
        result = vec.to_numpy()
        print(f"Vector {i} result:", result[:5])

def test_activation_functions():
    # アクティベーション関数のテスト
    vec = NNVector.from_numpy(np.array([-2.0, -1.0, 0.0, 1.0, 2.0], dtype=np.float32))
    vec.bind_to_unit(0)
    
    # ReLU
    relu_result = vec.relu()
    print("ReLU result:", relu_result.to_numpy())
    
    # tanh
    tanh_result = vec.tanh()
    print("tanh result:", tanh_result.to_numpy())

def test_error_handling():
    try:
        # 無効なユニットIDでのバインドを試みる
        vec = NNVector.from_numpy(np.ones(32, dtype=np.float32))
        vec.bind_to_unit(256)  # UNITCOUNTは256
    except ValueError as e:
        print("Expected error:", e)

    try:
        # バインドされていないベクトルでの操作を試みる
        vec = NNVector.from_numpy(np.ones(32, dtype=np.float32))
        vec.add_from_unit(0)
    except ValueError as e:
        print("Expected error:", e)

if __name__ == "__main__":
    print("Testing basic vector operations:")
    test_vector_operations()
    
    print("\nTesting parallel operations:")
    test_parallel_operations()
    
    print("\nTesting activation functions:")
    test_activation_functions()
    
    print("\nTesting error handling:")
    test_error_handling()