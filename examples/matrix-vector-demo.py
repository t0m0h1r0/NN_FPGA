import numpy as np
from fpga_accelerator import FpgaAccelerator

def main():
    # FPGAアクセラレータの初期化
    accelerator = FpgaAccelerator()

    # 64x128次元の行列と、128次元のベクトル
    matrix = np.random.randn(64, 128).astype(np.float32)
    vector1 = np.random.randn(128).astype(np.float32)
    vector2 = np.random.randn(128).astype(np.float32)

    print("大規模行列ベクトル乗算のテスト:")
    print("行列サイズ:", matrix.shape)
    print("ベクトルサイズ:", vector1.shape)

    # 行列の準備
    print("\n行列を準備中...")
    accelerator.prepare_matrix(matrix)
    print("行列の準備完了")

    # 1つ目のベクトルとの乗算
    print("\n1つ目のベクトルとの乗算:")
    # NumPy計算（比較用）
    numpy_result1 = np.dot(matrix, vector1)
    print("NumPy計算結果の最初の5要素:", numpy_result1[:5])

    # FPGAアクセラレータによる計算
    fpga_result1 = accelerator.compute_with_prepared_matrix(vector1)
    print("FPGAアクセラレータ計算結果の最初の5要素:", fpga_result1[:5])

    # 2つ目のベクトルとの乗算
    print("\n2つ目のベクトルとの乗算:")
    # NumPy計算（比較用）
    numpy_result2 = np.dot(matrix, vector2)
    print("NumPy計算結果の最初の5要素:", numpy_result2[:5])

    # FPGAアクセラレータによる計算
    fpga_result2 = accelerator.compute_with_prepared_matrix(vector2)
    print("FPGAアクセラレータ計算結果の最初の5要素:", fpga_result2[:5])

    # 結果の比較
    try:
        np.testing.assert_almost_equal(
            fpga_result1, 
            numpy_result1, 
            decimal=3, 
            err_msg="1つ目のベクトル: FPGAアクセラレータの計算結果がNumPyと大きく異なります"
        )
        np.testing.assert_almost_equal(
            fpga_result2, 
            numpy_result2, 
            decimal=3, 
            err_msg="2つ目のベクトル: FPGAアクセラレータの計算結果がNumPyと大きく異なります"
        )
        print("\n計算結果の検証に成功しました。")
    except AssertionError as e:
        print(f"結果検証エラー: {e}")

    # 行列変換のデモ
    conversion_types = ['full', 'trinary', 'fixed_point_1s31']
    print("\n行列変換デモ:")
    for conv_type in conversion_types:
        try:
            converted_matrix = accelerator.convert_matrix(matrix, conv_type)
            print(f"{conv_type.upper()}変換:")
            print("入力（最初の3x3）:", matrix[:3, :3])
            print("出力（最初の3x3）:", converted_matrix[:3, :3])
            print("---")
        except Exception as e:
            print(f"{conv_type.upper()}変換中にエラー: {e}")

if __name__ == "__main__":
    main()