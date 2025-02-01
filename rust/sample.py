# example.py
import numpy as np
from nn_accel import NNMatrix, NNVector

def test_matrix_operations():
    # NumPy配列から行列を作成
    np_array = np.ones((32, 32), dtype=np.float32)
    matrix = NNMatrix.from_numpy(np_array)

    # ベクトルの作成
    vector = NNVector.from_numpy(np.ones(32, dtype=np.float32))

    # 行列ベクトル積の計算
    result = matrix.multiply(vector)

    # 結果をNumPy配列に変換
    result_np = result.to_numpy()
    print("Result shape:", result_np.shape)
    print("First few values:", result_np[:5])

    # ReLUの適用
    activated = result.relu()
    activated_np = activated.to_numpy()
    print("Activated values:", activated_np[:5])

def test_matrix_storage():
    # 単位行列の作成と保存
    matrix = NNMatrix.identity(32)
    matrix.save("identity_matrix")

    # 行列の読み込み
    loaded = NNMatrix.load("identity_matrix", 32, 32)
    loaded_np = loaded.to_numpy()
    print("Loaded matrix diagonal:", np.diag(loaded_np))

if __name__ == "__main__":
    test_matrix_operations()
    test_matrix_storage()