//! アクセラレータのベンチマーク
//!
//! クリティカルな演算のパフォーマンスを測定します。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nn_accel::prelude::*;
use tokio::runtime::Runtime;

/// ベクトル加算のベンチマーク
fn benchmark_vector_addition(c: &mut Criterion) {
    // Tokioランタイムの作成
    let rt = Runtime::new().unwrap();

    c.bench_function("vector addition (32 elements)", |b| {
        b.iter(|| {
            rt.block_on(async {
                // アクセラレータの初期化
                let accelerator = Accelerator::new().await.unwrap();

                // ベクトルの作成
                let mut vec1 = Vector::new(32).unwrap();
                let mut vec2 = Vector::new(32).unwrap();

                // データの設定
                for i in 0..32 {
                    vec1.set(i, i as f32).await.unwrap();
                    vec2.set(i, (i * 2) as f32).await.unwrap();
                }

                // ユニットへのバインド
                vec1.bind_to_unit(UnitId::new(0).unwrap()).await.unwrap();
                vec2.bind_to_unit(UnitId::new(1).unwrap()).await.unwrap();

                // 加算演算の実行
                black_box(
                    accelerator.execute(
                        Operation::Add { 
                            source: UnitId::new(1).unwrap() 
                        }, 
                        UnitId::new(0).unwrap()
                    ).await
                ).unwrap();
            });
        })
    });
}

/// 大規模ベクトル演算のベンチマーク
fn benchmark_large_vector_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("large vector operations (1024 elements)", |b| {
        b.iter(|| {
            rt.block_on(async {
                let accelerator = Accelerator::new().await.unwrap();

                // 大規模ベクトルの作成
                let mut vec1 = Vector::new(1024).unwrap();
                let mut vec2 = Vector::new(1024).unwrap();

                // データの初期化
                for i in 0..1024 {
                    vec1.set(i, (i % 100) as f32).await.unwrap();
                    vec2.set(i, (i * 2 % 100) as f32).await.unwrap();
                }

                // ユニットへのバインド
                vec1.bind_to_unit(UnitId::new(0).unwrap()).await.unwrap();
                vec2.bind_to_unit(UnitId::new(1).unwrap()).await.unwrap();

                // 複数の演算を実行
                let operations = vec![
                    Operation::Copy { source: UnitId::new(1).unwrap() },
                    Operation::Add { source: UnitId::new(1).unwrap() },
                    Operation::Activate { function: ActivationFunction::ReLU },
                ];

                for op in operations {
                    black_box(
                        accelerator.execute(op, UnitId::new(0).unwrap()).await
                    ).unwrap();
                }
            });
        })
    });
}

/// 活性化関数のベンチマーク
fn benchmark_activation_functions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("Activation Functions");
    
    // ReLU
    group.bench_function("ReLU activation (512 elements)", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut vec = Vector::new(512).unwrap();
                
                // データの初期化（正負混在）
                for i in 0..512 {
                    vec.set(i, (i as f32 - 256.0) / 32.0).await.unwrap();
                }

                black_box(
                    vec.apply_activation(ActivationFunction::ReLU).await
                ).unwrap();
            });
        })
    });

    // Tanh
    group.bench_function("Tanh activation (512 elements)", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut vec = Vector::new(512).unwrap();
                
                // データの初期化（正負混在）
                for i in 0..512 {
                    vec.set(i, (i as f32 - 256.0) / 32.0).await.unwrap();
                }

                black_box(
                    vec.apply_activation(ActivationFunction::Tanh).await
                ).unwrap();
            });
        })
    });

    group.finish();
}

/// 並列演算のベンチマーク
fn benchmark_parallel_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("Parallel vector operations (8 units)", |b| {
        b.iter(|| {
            rt.block_on(async {
                let accelerator = Accelerator::new().await.unwrap();

                // 複数のベクトルを作成
                let mut vectors: Vec<Vector<f32>> = (0..8)
                    .map(|_| Vector::new(256).expect("ベクトル作成に失敗"))
                    .collect();

                // データの初期化とユニットへのバインド
                for (i, vec) in vectors.iter_mut().enumerate() {
                    for j in 0..256 {
                        vec.set(j, (i * j % 100) as f32).await.unwrap();
                    }
                    vec.bind_to_unit(UnitId::new(i as u8).unwrap()).await.unwrap();
                }

                // 連続した演算の実行
                for i in 1..8 {
                    black_box(
                        accelerator.execute(
                            Operation::Add { 
                                source: UnitId::new((i-1) as u8).unwrap() 
                            }, 
                            UnitId::new(i as u8).unwrap()
                        ).await
                    ).unwrap();
                }
            });
        })
    });
}

/// ベンチマークグループの設定
criterion_group!(
    benches, 
    benchmark_vector_addition, 
    benchmark_large_vector_operations,
    benchmark_activation_functions,
    benchmark_parallel_operations
);

/// メイン関数
criterion_main!(benches);