//! 統合テスト
//!
//! アクセラレータの主要機能の統合テストを提供します。

use nn_accel::prelude::*;

#[tokio::test]
async fn test_vector_operations() -> Result<()> {
    // アクセラレータの初期化
    let accelerator = Accelerator::new().await?;

    // ベクトルの作成
    let mut vec1 = Vector::new(32)?;
    let mut vec2 = Vector::new(32)?;

    // データの設定
    for i in 0..32 {
        vec1.set(i, i as f32).await?;
        vec2.set(i, (i * 2) as f32).await?;
    }

    // ユニットへのバインド
    vec1.bind_to_unit(UnitId::new(0)?).await?;
    vec2.bind_to_unit(UnitId::new(1)?).await?;

    // コピー演算
    accelerator.execute(
        Operation::Copy { 
            source: UnitId::new(1)? 
        }, 
        UnitId::new(0)?
    ).await?;

    // 加算演算
    accelerator.execute(
        Operation::Add { 
            source: UnitId::new(1)? 
        }, 
        UnitId::new(0)?
    ).await?;

    // ReLU活性化関数の適用
    vec1.apply_activation(ActivationFunction::ReLU).await?;

    // 結果の検証
    for i in 0..32 {
        let val = vec1.get(i).await?;
        assert!(val >= 0.0, "ReLU変換後の値は0以上である必要があります");
    }

    Ok(())
}

#[tokio::test]
async fn test_multiple_unit_operations() -> Result<()> {
    let accelerator = Accelerator::new().await?;

    // 複数のベクトルを作成
    let mut vectors: Vec<Vector<f32>> = (0..4)
        .map(|_| Vector::new(32).expect("ベクトル作成に失敗"))
        .collect();

    // データの設定とユニットへのバインド
    for (i, vec) in vectors.iter_mut().enumerate() {
        for j in 0..32 {
            vec.set(j, (i * j) as f32).await?;
        }
        vec.bind_to_unit(UnitId::new(i as u8)?).await?;
    }

    // 連続した演算の実行
    for i in 1..4 {
        accelerator.execute(
            Operation::Add { 
                source: UnitId::new((i-1) as u8)? 
            }, 
            UnitId::new(i as u8)?
        ).await?;
    }

    // 最後のベクトルに活性化関数を適用
    let last_vec = vectors.last_mut().unwrap();
    last_vec.apply_activation(ActivationFunction::Tanh).await?;

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // 無効なユニットIDでの演算テスト
    let accelerator = Accelerator::new().await?;
    let invalid_unit = UnitId::new(255)?;

    // 無効なユニットでの演算は失敗することを確認
    let result = accelerator.execute(
        Operation::Copy { 
            source: invalid_unit 
        }, 
        invalid_unit
    ).await;

    assert!(result.is_err(), "無効なユニットでの演算は失敗する必要があります");

    Ok(())
}

#[tokio::test]
async fn test_system_status() -> Result<()> {
    let accelerator = Accelerator::new().await?;

    // システムステータスの取得
    let status = accelerator.status().await?;

    // ステータスの基本的な検証
    assert!(status.fpga.ready, "FPGAは準備済みである必要があります");
    assert!(status.performance.ops_per_second >= 0.0, "演算数は0以上である必要があります");

    Ok(())
}
