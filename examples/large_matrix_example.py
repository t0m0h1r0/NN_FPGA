import numpy as np
import time
from fpga_accelerator import FpgaAccelerator

def process_large_matrix(size: int = 1024):
    """大規模行列の処理デモ
    
    Args:
        size: 行列のサイズ（デフォルト: 1024）。16の倍数である必要があります。
    """
    if size % 16 != 0:
        raise ValueError("サイズは16の倍数である必要があります")
    
    print(f"=== {size}×{size} 行列の処理デモ ===")
    
    # アクセラレータの初期化
    accelerator = FpgaAccelerator()
    
    # 大規模テストデータの生成
    print("\nテストデータを生成中...")
    matrix = np.random.randn(size, size).astype(np.float32)
    vector = np.random.randn(size).astype(np.float32)
    
    print(f"行列サイズ: {matrix.shape} ({matrix.nbytes / 1024 / 1024:.2f} MB)")
    print(f"ベクトルサイズ: {vector.shape} ({vector.nbytes / 1024 / 1024:.2f} MB)")
    
    # 行列の準備（ブロック分割）
    print("\n行列を準備中（ブロック分割とキャッシュ）...")
    start_time = time.time()
    accelerator.prepare_matrix(matrix)
    prep_time = time.time() - start_time
    print(f"準備時間: {prep_time:.4f}秒")
    
    # 複数回の乗算テスト
    num_tests = 5
    fpga_times = []
    numpy_times = []
    
    print(f"\n{num_tests}回の乗算テストを実行:")
    
    for i in range(num_tests):
        print(f"\nテスト {i+1}/{num_tests}:")
        
        # スケーリングしたテストベクトル
        scale = np.random.uniform(0.5, 1.5)
        test_vector = vector * scale
        
        # FPGAでの計算
        start_time = time.time()
        fpga_result = accelerator.compute_with_prepared_matrix(test_vector)
        fpga_time = time.time() - start_time
        fpga_times.append(fpga_time)
        
        # NumPyでの計算
        start_time = time.time()
        numpy_result = np.dot(matrix, test_vector)
        numpy_time = time.time() - start_time
        numpy_times.append(numpy_time)
        
        # 結果の検証
        max_diff = np.max(np.abs(fpga_result - numpy_result))
        print(f"最大誤差: {max_diff}")
        print(f"FPGA時間: {fpga_time:.4f}秒")
        print(f"NumPy時間: {numpy_time:.4f}秒")
        print(f"速度比: {numpy_time/fpga_time:.2f}倍")
    
    # 性能統計
    print("\n=== 性能統計 ===")
    print(f"行列サイズ: {size}×{size}")
    print(f"演算数: {2 * size * size} FLOPS/乗算")
    
    avg_fpga_time = np.mean(fpga_times)
    avg_numpy_time = np.mean(numpy_times)
    
    print("\nFPGA統計:")
    print(f"平均時間: {avg_fpga_time:.4f}秒")
    print(f"最小時間: {np.min(fpga_times):.4f}秒")
    print(f"最大時間: {np.max(fpga_times):.4f}秒")
    print(f"GFLOPS: {(2 * size * size) / (avg_fpga_time * 1e9):.2f}")
    
    print("\nNumPy統計:")
    print(f"平均時間: {avg_numpy_time:.4f}秒")
    print(f"最小時間: {np.min(numpy_times):.4f}秒")
    print(f"最大時間: {np.max(numpy_times):.4f}秒")
    print(f"GFLOPS: {(2 * size * size) / (avg_numpy_time * 1e9):.2f}")
    
    print(f"\n平均速度比: {avg_numpy_time/avg_fpga_time:.2f}倍")

def block_size_experiment():
    """異なるブロックサイズでの性能比較"""
    print("\n=== ブロックサイズ実験 ===")
    
    sizes = [16, 32, 64, 128, 256]
    results = []
    
    for size in sizes:
        print(f"\nブロックサイズ {size}×{size} のテスト:")
        
        # テストデータ生成
        matrix = np.random.randn(size, size).astype(np.float32)
        vector = np.random.randn(size).astype(np.float32)
        
        # FPGAでの計算
        accelerator = FpgaAccelerator()
        
        # 準備時間の計測
        start_time = time.time()
        accelerator.prepare_matrix(matrix)
        prep_time = time.time() - start_time
        
        # 乗算時間の計測（5回の平均）
        mult_times = []
        for _ in range(5):
            start_time = time.time()
            accelerator.compute_with_prepared_matrix(vector)
            mult_times.append(time.time() - start_time)
        
        avg_mult_time = np.mean(mult_times)
        
        # 結果の記録
        results.append({
            'size': size,
            'prep_time': prep_time,
            'mult_time': avg_mult_time,
            'gflops': (2 * size * size) / (avg_mult_time * 1e9)
        })
        
        print(f"準備時間: {prep_time:.4f}秒")
        print(f"乗算時間: {avg_mult_time:.4f}秒")
        print(f"GFLOPS: {results[-1]['gflops']:.2f}")
    
    # 結果の表示
    print("\n=== ブロックサイズの影響 ===")
    print(f"{'サイズ':>8} {'準備時間':>12} {'乗算時間':>12} {'GFLOPS':>10}")
    print("-" * 44)
    
    for result in results:
        print(f"{result['size']:8d} {result['prep_time']:12.4f} "
              f"{result['mult_time']:12.4f} {result['gflops']:10.2f}")

def main():
    np.random.seed(42)  # 再現性のため
    
    # 大規模行列の処理
    process_large_matrix(1024)  # 1024×1024
    
    # ブロックサイズの実験
    block_size_experiment()

if __name__ == "__main__":
    main()