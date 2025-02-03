import numpy as np
from fpga_accelerator import FpgaAccelerator

def basic_vector_operations():
    """基本的なベクトル演算のデモ"""
    print("=== 基本的なベクトル演算のデモ ===")
    
    # アクセラレータの初期化
    accelerator = FpgaAccelerator()
    
    # 16要素のテストベクトルを作成
    vector = np.random.randn(16).astype(np.float32)
    print("\n入力ベクトル:")
    print(vector)
    
    # 各種演算の実行と結果の表示
    operations = {
        'add': 'ベクトル + 1',
        'mul': 'ベクトル × 2',
        'tanh': 'tanh(ベクトル)',
        'relu': 'ReLU(ベクトル)'
    }
    
    for op_name, description in operations.items():
        print(f"\n{description}:")
        result = accelerator.compute_vector(vector, op_name)
        print(result)
        
        # NumPyでの計算結果と比較
        if op_name == 'add':
            numpy_result = vector + 1
        elif op_name == 'mul':
            numpy_result = vector * 2
        elif op_name == 'tanh':
            numpy_result = np.tanh(vector)
        else:  # relu
            numpy_result = np.maximum(vector, 0)
        
        max_diff = np.max(np.abs(result - numpy_result))
        print(f"NumPy結果との最大差: {max_diff}")

def matrix_vector_multiply():
    """行列ベクトル乗算のデモ"""
    print("\n=== 行列ベクトル乗算のデモ ===")
    
    accelerator = FpgaAccelerator()
    
    # 32×32のテスト行列を作成
    matrix = np.random.randn(32, 32).astype(np.float32)
    vector = np.random.randn(32).astype(np.float32)
    
    print(f"行列サイズ: {matrix.shape}")
    print(f"ベクトルサイズ: {vector.shape}")
    
    # 行列を準備（内部でブロック分割とキャッシュが行われる）
    print("\n行列を準備中...")
    accelerator.prepare_matrix(matrix)
    
    # 同じ行列で複数回の乗算を実行
    print("\n複数のベクトルとの乗算テスト:")
    for i in range(3):
        # テストベクトルを少しずつ変化させる
        test_vector = vector * (i + 1)
        
        # FPGAでの計算
        start_time = time.time()
        fpga_result = accelerator.compute_with_prepared_matrix(test_vector)
        fpga_time = time.time() - start_time
        
        # NumPyでの計算（比較用）
        start_time = time.time()
        numpy_result = np.dot(matrix, test_vector)
        numpy_time = time.time() - start_time
        
        # 結果の検証と表示
        max_diff = np.max(np.abs(fpga_result - numpy_result))
        print(f"\nテスト {i+1}:")
        print(f"最大誤差: {max_diff}")
        print(f"FPGA実行時間: {fpga_time:.6f}秒")
        print(f"NumPy実行時間: {numpy_time:.6f}秒")
        print(f"速度比: {numpy_time/fpga_time:.2f}倍")

def data_conversion_demo():
    """データ型変換のデモ"""
    print("\n=== データ型変換のデモ ===")
    
    accelerator = FpgaAccelerator()
    
    # テストデータの作成
    vector = np.array([-1.5, -0.5, 0.0, 0.5, 1.5]).astype(np.float32)
    print("\n入力ベクトル:")
    print(vector)
    
    # 各種変換の実行と結果の表示
    conversion_types = ['trinary', 'fixed_point_1s31']
    descriptions = {
        'trinary': '三値化 (-1, 0, 1)',
        'fixed_point_1s31': '固定小数点 (1s.31形式)'
    }
    
    for conv_type in conversion_types:
        print(f"\n{descriptions[conv_type]}変換:")
        result = accelerator.convert_vector(vector, conv_type)
        print(result)

def main():
    # 表示オプションの設定
    np.set_printoptions(precision=4, suppress=True)
    
    # 各デモの実行
    basic_vector_operations()
    matrix_vector_multiply()
    data_conversion_demo()

if __name__ == "__main__":
    main()