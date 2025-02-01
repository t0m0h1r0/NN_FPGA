import numpy as np
from fpga_accelerator import FpgaAccelerator

def main():
    # FPGAアクセラレータの初期化
    accelerator = FpgaAccelerator()

    # 16の倍数のベクトルを作成
    vector = np.random.randn(16).astype(np.float32)

    # 各計算タイプのデモ
    computation_types = ['add', 'mul', 'tanh', 'relu']
    print("ベクトル計算デモ:")
    for comp_type in computation_types:
        try:
            result = accelerator.compute_vector(vector, comp_type)
            print(f"{comp_type.upper()} 計算結果:")
            print("入力:", vector)
            print("出力:", result)
            print("---")
        except Exception as e:
            print(f"{comp_type.upper()} 計算中にエラー: {e}")

    # 変換タイプのデモ
    conversion_types = ['full', 'trinary', 'fixed_point_1s31']
    print("\nベクトル変換デモ:")
    for conv_type in conversion_types:
        try:
            converted = accelerator.convert_vector(vector, conv_type)
            print(f"{conv_type.upper()}変換:")
            print("入力:", vector)
            print("出力:", converted)
            print("---")
        except Exception as e:
            print(f"{conv_type.upper()}変換中にエラー: {e}")

if __name__ == "__main__":
    main()
