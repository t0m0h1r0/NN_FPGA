import numpy as np
from fpga_accelerator import FpgaAccelerator

def demonstrate_vector_operations():
    print("=== ベクトル演算のデモ ===")
    
    # アクセラレータの初期化
    accelerator = FpgaAccelerator()
    
    # テストベクトルの作成（16の倍数サイズ）
    vector = np.random.randn(16).astype(np.float32)
    print("入力ベクトル:", vector)
    
    # 各種演算のテスト
    operations = ['add', 'mul', 'tanh', 'relu']
    for op in operations:
        try:
            result = accelerator.compute_vector(vector, op)
            print(f"\n{op}演算の結果:")
            print(result)
            
            # NumPyでの計算結果と比較
            if op == 'relu':
                numpy_result = np.maximum(vector, 0)
            elif op == 'tanh':
                numpy_result = np.tanh(vector)
            elif op == 'add':
                numpy_result = vector + 1
            else:  # mul
                numpy_result = vector * 2
                
            print(f"NumPy結果との最大差: {np.max(np.abs(result - numpy_result))}")
        except Exception as e:
            print(f"{op}演算でエラー: {e}")

def demonstrate_matrix_operations():
    print("\n=== 行列演算のデモ ===")
    
    accelerator = FpgaAccelerator()
    
    # 16x16のテスト行列作成
    matrix = np.random.randn(16, 16).astype(np.float32)
    vector = np.random.randn(16).astype(np.float32)
    
    print("入力行列の形状:", matrix.shape)
    print("入力ベクトルの形状:", vector.shape)
    
    try:
        # 行列の準備
        accelerator.prepare_matrix(matrix)
        print("行列の準備完了")
        
        # 行列ベクトル乗算
        result = accelerator.compute_with_prepared_matrix(vector)
        print("\n行列ベクトル乗算の結果:")
        print(result)
        
        # NumPyでの計算結果と比較
        numpy_result = np.dot(matrix, vector)
        print("\nNumPyでの計算結果との差の最大値:")
        print(np.max(np.abs(result - numpy_result)))
        
    except Exception as e:
        print(f"行列演算でエラー: {e}")

def demonstrate_data_conversion():
    print("\n=== データ変換のデモ ===")
    
    accelerator = FpgaAccelerator()
    
    # テストデータの作成
    vector = np.random.randn(16).astype(np.float32)
    matrix = np.random.randn(16, 16).astype(np.float32)
    
    # ベクトル変換のテスト
    conversion_types = ['full', 'trinary', 'fixed_point_1s31']
    for conv_type in conversion_types:
        try:
            converted_vector = accelerator.convert_vector(vector, conv_type)
            print(f"\n{conv_type}ベクトル変換の結果:")
            print("入力:", vector)
            print("変換後:", converted_vector)
            if conv_type == 'trinary':
                unique_vals = np.unique(converted_vector)
                print("ユニークな値:", unique_vals)
        except Exception as e:
            print(f"{conv_type}変換でエラー: {e}")
    
    # 行列変換のテスト
    print("\n行列変換のテスト:")
    for conv_type in conversion_types:
        try:
            converted_matrix = accelerator.convert_matrix(matrix, conv_type)
            print(f"\n{conv_type}行列変換の結果（左上3x3のみ表示）:")
            print("入力:\n", matrix[:3, :3])
            print("変換後:\n", converted_matrix[:3, :3])
        except Exception as e:
            print(f"{conv_type}変換でエラー: {e}")

def main():
    np.set_printoptions(precision=4, suppress=True)
    demonstrate_vector_operations()
    demonstrate_matrix_operations()
    demonstrate_data_conversion()

if __name__ == "__main__":
    main()