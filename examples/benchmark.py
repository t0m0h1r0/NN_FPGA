import numpy as np
import time
from fpga_accelerator import FpgaAccelerator
from typing import Tuple, List, Dict
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path

class BenchmarkResults:
    def __init__(self):
        self.results: List[Dict] = []
    
    def add_result(self, operation: str, size: int, time_taken: float, gflops: float = None):
        result = {
            'operation': operation,
            'size': size,
            'time': time_taken
        }
        if gflops is not None:
            result['gflops'] = gflops
        self.results.append(result)
    
    def to_dataframe(self) -> pd.DataFrame:
        return pd.DataFrame(self.results)
    
    def plot_results(self, save_path: str = None):
        df = self.to_dataframe()
        
        # 時間のプロット
        plt.figure(figsize=(12, 6))
        sns.lineplot(data=df, x='size', y='time', hue='operation', marker='o')
        plt.xlabel('行列/ベクトルサイズ')
        plt.ylabel('実行時間 (秒)')
        plt.title('性能ベンチマーク結果（実行時間）')
        plt.grid(True)
        if save_path:
            plt.savefig(f"{save_path}_time.png")
        plt.close()
        
        # GFLOPSのプロット（存在する場合）
        if 'gflops' in df.columns:
            plt.figure(figsize=(12, 6))
            sns.lineplot(data=df, x='size', y='gflops', hue='operation', marker='o')
            plt.xlabel('行列/ベクトルサイズ')
            plt.ylabel('GFLOPS')
            plt.title('性能ベンチマーク結果（GFLOPS）')
            plt.grid(True)
            if save_path:
                plt.savefig(f"{save_path}_gflops.png")
            plt.close()

def create_test_data(size: int) -> Tuple[np.ndarray, np.ndarray]:
    """テストデータの生成"""
    matrix = np.random.randn(size, size).astype(np.float32)
    vector = np.random.randn(size).astype(np.float32)
    return matrix, vector

def calculate_gflops(size: int, time_taken: float, operation: str) -> float:
    """GFLOPS（1秒あたりの浮動小数点演算数）を計算"""
    if operation == 'matrix_vector_multiply':
        # 行列ベクトル乗算の演算数: 2 * N * N (乗算と加算)
        flops = 2 * size * size
    elif operation in ['add', 'mul']:
        # ベクトル演算の演算数: N
        flops = size
    elif operation in ['tanh', 'relu']:
        # 活性化関数の演算数: 約4N (近似)
        flops = 4 * size
    else:
        return 0.0
    
    return (flops / time_taken) / 1e9  # Convert to GFLOPS

def benchmark_matrix_operations(sizes: List[int], num_trials: int = 5) -> BenchmarkResults:
    """行列演算のベンチマーク"""
    results = BenchmarkResults()
    accelerator = FpgaAccelerator()
    
    for size in sizes:
        # 複数回の試行の平均を取る
        prep_times = []
        compute_times = []
        numpy_times = []
        
        for _ in range(num_trials):
            matrix, vector = create_test_data(size)
            
            # 行列準備のベンチマーク
            start_time = time.perf_counter()
            accelerator.prepare_matrix(matrix)
            prep_time = time.perf_counter() - start_time
            prep_times.append(prep_time)
            
            # 行列ベクトル乗算のベンチマーク
            start_time = time.perf_counter()
            result = accelerator.compute_with_prepared_matrix(vector)
            compute_time = time.perf_counter() - start_time
            compute_times.append(compute_time)
            
            # NumPyとの比較
            start_time = time.perf_counter()
            np.dot(matrix, vector)
            numpy_time = time.perf_counter() - start_time
            numpy_times.append(numpy_time)
        
        # 平均時間を計算
        avg_prep_time = np.mean(prep_times)
        avg_compute_time = np.mean(compute_times)
        avg_numpy_time = np.mean(numpy_times)
        
        # GFLOPSを計算
        compute_gflops = calculate_gflops(size, avg_compute_time, 'matrix_vector_multiply')
        numpy_gflops = calculate_gflops(size, avg_numpy_time, 'matrix_vector_multiply')
        
        # 結果を記録
        results.add_result('FPGA_preparation', size, avg_prep_time)
        results.add_result('FPGA_computation', size, avg_compute_time, compute_gflops)
        results.add_result('NumPy', size, avg_numpy_time, numpy_gflops)
    
    return results

def benchmark_vector_operations(sizes: List[int], num_trials: int = 5) -> BenchmarkResults:
    """ベクトル演算のベンチマーク"""
    results = BenchmarkResults()
    accelerator = FpgaAccelerator()
    operations = ['add', 'mul', 'tanh', 'relu']
    
    for size in sizes:
        for op in operations:
            times = []
            numpy_times = []
            
            for _ in range(num_trials):
                vector = np.random.randn(size).astype(np.float32)
                
                # FPGAでの計算
                start_time = time.perf_counter()
                accelerator.compute_vector(vector, op)
                fpga_time = time.perf_counter() - start_time
                times.append(fpga_time)
                
                # NumPyでの計算
                start_time = time.perf_counter()
                if op == 'relu':
                    np.maximum(vector, 0)
                elif op == 'tanh':
                    np.tanh(vector)
                elif op == 'add':
                    vector + 1
                else:  # mul
                    vector * 2
                numpy_time = time.perf_counter() - start_time
                numpy_times.append(numpy_time)
            
            # 平均時間を計算
            avg_time = np.mean(times)
            avg_numpy_time = np.mean(numpy_times)
            
            # GFLOPSを計算
            fpga_gflops = calculate_gflops(size, avg_time, op)
            numpy_gflops = calculate_gflops(size, avg_numpy_time, op)
            
            # 結果を記録
            results.add_result(f'FPGA_{op}', size, avg_time, fpga_gflops)
            results.add_result(f'NumPy_{op}', size, avg_numpy_time, numpy_gflops)
    
    return results

def benchmark_data_conversion(sizes: List[int], num_trials: int = 5) -> BenchmarkResults:
    """データ変換のベンチマーク"""
    results = BenchmarkResults()
    accelerator = FpgaAccelerator()
    conversion_types = ['trinary', 'fixed_point_1s31']
    
    for size in sizes:
        matrix, vector = create_test_data(size)
        
        # ベクトル変換のベンチマーク
        for conv_type in conversion_types:
            times = []
            for _ in range(num_trials):
                start_time = time.perf_counter()
                accelerator.convert_vector(vector, conv_type)
                conv_time = time.perf_counter() - start_time
                times.append(conv_time)
            avg_time = np.mean(times)
            results.add_result(f'vector_{conv_type}', size, avg_time)
        
        # 行列変換のベンチマーク
        for conv_type in conversion_types:
            times = []
            for _ in range(num_trials):
                start_time = time.perf_counter()
                accelerator.convert_matrix(matrix, conv_type)
                conv_time = time.perf_counter() - start_time
                times.append(conv_time)
            avg_time = np.mean(times)
            results.add_result(f'matrix_{conv_type}', size, avg_time)
    
    return results

def main():
    # 結果保存用のディレクトリを作成
    results_dir = Path("benchmark_results")
    results_dir.mkdir(exist_ok=True)
    
    # ベンチマークサイズの設定
    sizes = [16, 32, 64, 128, 256]  # 16の倍数
    num_trials = 5
    
    print("行列演算のベンチマークを実行中...")
    matrix_results = benchmark_matrix_operations(sizes, num_trials)
    matrix_results.plot_results(str(results_dir / "matrix_benchmark"))
    matrix_df = matrix_results.to_dataframe()
    matrix_df.to_csv(results_dir / "matrix_results.csv", index=False)
    
    print("ベクトル演算のベンチマークを実行中...")
    vector_results = benchmark_vector_operations(sizes, num_trials)
    vector_results.plot_results(str(results_dir / "vector_benchmark"))
    vector_df = vector_results.to_dataframe()
    vector_df.to_csv(results_dir / "vector_results.csv", index=False)
    
    print("データ変換のベンチマークを実行中...")
    conversion_results = benchmark_data_conversion(sizes, num_trials)
    conversion_results.plot_results(str(results_dir / "conversion_benchmark"))
    conversion_df = conversion_results.to_dataframe()
    conversion_df.to_csv(results_dir / "conversion_results.csv", index=False)
    
    print("\nベンチマーク結果の概要:")
    print("\n1. 行列演算の結果:")
    print(matrix_df.groupby('operation').agg({'time': ['mean', 'min', 'max'], 'gflops': 'max'}).round(4))
    
    print("\n2. ベクトル演算の結果:")
    print(vector_df.groupby('operation').agg({'time': ['mean', 'min', 'max'], 'gflops': 'max'}).round(4))
    
    print("\n3. データ変換の結果:")
    print(conversion_df.groupby('operation').agg({'time': ['mean', 'min', 'max']}).round(4))
    
    print(f"\nすべての結果は {results_dir} ディレクトリに保存されました。")

if __name__ == "__main__":
    main()