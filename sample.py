import numpy as np
from fpga_accelerator import FpgaAccelerator

def main():
    # FPGAアクセラレータの初期化
    accelerator = FpgaAccelerator()

    # デモ用のベクトルと行列
    vector = np.random.randn(16).astype(np.float32)
    matrix = np.random.randn(16, 16).astype(np.float32)

    # 変換タイプ
    conversion_types = ['full', 'trinary', 'fixed_point']

    # ベクトル変換のデモ
    print("ベクトル変換のデモ:")
    for conv_type in conversion_types:
        try:
            converted_vector = accelerator.convert_vector(vector, conv_type)
            print(f"{conv_type.upper()}変換:")
            print("元のベクトル:", vector[:5])
            print("変換後のベクトル:", converted_vector[:5])
            print("---")
        except Exception as e:
            print(f"{conv_type.upper()}変換中にエラー: {e}")

    # 行列変換のデモ
    print("\n行列変換のデモ:")
    for conv_type in conversion_types:
        try:
            converted_matrix = accelerator.convert_matrix(matrix, conv_type)
            print(f"{conv_type.upper()}変換:")
            print("元の行列（最初の3x3）:")
            print(matrix[:3, :3])
            print("変換後の行列（最初の3x3）:")
            print(converted_matrix[:3, :3])
            print("---")
        except Exception as e:
            print(f"{conv_type.upper()}変換中にエラー: {e}")

if __name__ == "__main__":
    main()