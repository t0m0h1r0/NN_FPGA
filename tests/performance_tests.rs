//! パフォーマンステスト
//!
//! アクセラレータの性能評価を行います。

use std::time::Instant;
use nn_accel::prelude::*;

/// 大規模ベクトル演算のパフォーマンステスト
#[tokio::test]
async fn test_large_vector_performance() -> Result<()> {
    let accelerator = Accelerator::new().await?;

    // 大規模ベクトルの作成（1024要素）
    let mut vec1 = Vector::new(1024)?;
    let mut vec2 = Vector::new(1024)?;

    // データの初期化
    for i in 0..1024 {
        vec1.set(i, (i % 100) as f32).await?;
        vec2.set(i, (i * 2 % 100) as f32).await?;
    }

    // ユニットへのバインド
    vec1.bind_to_unit(UnitId::new(0)?).await?;
    vec2.bind_to_unit(UnitId::new(1)?).await?;

    // 演算開始時間の記録
    let start_time = Instant::now();

    // 複数の演算を実行
    let operations = vec![
        Operation::Copy { source: UnitId::new(1)? },
        Operation::Add { source: UnitId::new(1)? },
        Operation::Activate { function: ActivationFunction::ReLU },
    ];

    for op in operations {
        accelerator.execute(op, UnitId::new(0)?).await?;
    }

    // 実行時間の計測
    let duration = start_time.elapsed();
    println!("大規模ベクトル演算の実行時間: {:?}", duration);

    // パフォーマンス基準（任意の閾値）
    assert!(duration.as_millis() < 100, "演算時間が長すぎます");

    Ok(())
}

/// 並列演算のパフォーマンステスト
#[tokio::test]
async fn test_parallel_operations_performance() -> Result<()> {
    let accelerator = Accelerator::new().await?;

    // 複数のベクトルを作成
    let mut vectors: Vec<Vector<f32>> = (0..8)
        .map(|_| Vector::new(256).expect("ベクトル作成に失敗"))
        .collect();

    // データの初期化とユニットへのバインド
    for (i, vec) in vectors.iter_mut().enumerate() {
        for j in 0..256 {
            vec.set(j, (i * j % 100) as f32).await?;
        }
        vec.bind_to_unit(UnitId::new(i as u8)?).await?;
    }

    // 演算開始時間の記録
    let start_time = Instant::now();

    // 連続した演算の実行
    for i in 1..8 {
        accelerator.execute(
            Operation::Add { 
                source: UnitId::new((i-1) as u8)? 
            }, 
            UnitId::new(i as u8)?
        ).await?;
    }

    // 最後の2つのベクトルに活性化関数を適用
    for i in [6, 7] {
        let func = if i == 6 { 
            ActivationFunction::ReLU 
        } else { 
            ActivationFunction::Tanh 
        };
        
        vectors[i].apply_activation(func).await?;
    }

    // 実行時間の計測
    let duration = start_time.elapsed();
    println!("並列演算の実行時間: {:?}", duration);

    // パフォーマンス基準（任意の閾値）
    assert!(duration.as_millis() < 200, "並列演算の時間が長すぎます");

    Ok(())
}

/// メモリ効率性能テスト
#[tokio::test]
async fn test_memory_efficiency() -> Result<()> {
    let accelerator = Accelerator::new().await?;

    // システムステータスの取得
    let status = accelerator.status().await?;

    // メモリ使用率の検証
    println!("メモリ使用状況: {:?}", status.memory);
    
    // メモリ使用率が一定範囲内であることを確認
    assert!(status.memory.used_blocks as f64 / status.memory.total_blocks as f64 <= 0.5, 
        "メモリ使用率が高すぎます");

    Ok(())
}
