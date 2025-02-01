//! 高度な演算と並列処理のデモンストレーション

use nn_accel::prelude::*;
use std::time::Instant;

/// 複雑な演算シナリオのデモ
#[tokio::main]
async fn main() -> Result<()> {
    // アクセラレータの初期化
    let accelerator = Accelerator::new().await?;

    // 複数のベクトルを作成
    let mut vectors: Vec<Vector<f32>> = (0..4)
        .map(|_| Vector::new(64).expect("ベクトル作成に失敗"))
        .collect();

    // ベクトルへのデータ設定
    for (i, vec) in vectors.iter_mut().enumerate() {
        for j in 0..64 {
            vec.set(j, (i * j) as f32).await?;
        }
        // ユニットへのバインド
        vec.bind_to_unit(UnitId::new(i as u8)?).await?;
    }

    // 開始時間の記録
    let start_time = Instant::now();

    // 並列演算の実行
    for i in 1..4 {
        // 前のベクトルから現在のベクトルへの加算
        accelerator.execute(
            Operation::Add { 
                source: UnitId::new((i-1) as u8)? 
            }, 
            UnitId::new(i as u8)?
        ).await?;
    }

    // 活性化関数の適用
    for (i, vec) in vectors.iter_mut().enumerate().skip(2) {
        // 最後の2つのベクトルにReLUとTanh
        match i {
            2 => vec.apply_activation(ActivationFunction::ReLU).await?,
            3 => vec.apply_activation(ActivationFunction::Tanh).await?,
            _ => {}
        }
    }

    // 実行時間の表示
    println!("演算完了時間: {:?}", start_time.elapsed());

    // システムステータスの取得と表示
    let status = accelerator.status().await?;
    println!("システムステータス: {:?}", status);

    // 結果の表示
    for (i, vec) in vectors.iter().enumerate() {
        println!("ベクトル {} の最初の5要素:", i);
        for j in 0..5 {
            println!("  {}: {}", j, vec.get(j).await?);
        }
    }

    Ok(())
}
