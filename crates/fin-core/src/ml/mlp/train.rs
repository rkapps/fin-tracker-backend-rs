use anyhow::Result;
use candle_core::{DType, Device, Tensor, Var};
use candle_nn::{Linear, Module, Optimizer, VarBuilder, VarMap, linear};
use fin_domain::tickers::TickerAlpha;
use rand::{seq::SliceRandom, thread_rng};
use tracing::{debug, trace, warn};

use crate::ml::common::{metrics::log_metrics_from_vecs, models::ModelMetrics};

pub struct MlpResult {
    pub metrics: ModelMetrics,
    pub mean_down: f64,
    pub mean_neutral: f64,
    pub mean_up: f64,
    pub weights: Vec<u8>,
    pub label_mean: f64,
    pub label_std: f64,
}

pub fn train_models_for_mlp(
    labeled_data: &[(f64, Vec<f64>)],
    train_data: &[(f64, Vec<f64>)],
    test_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<MlpResult> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let n_features = train_data[0].1.len();
    let model = IndicatorMlp::new(n_features, vb)?;

    // Balance training set only — never touch test set
    let balanced_train = balance_labels(train_data);
    // After balancing, recompute and force center to zero
    let raw_mean =
        balanced_train.iter().map(|(l, _)| *l).sum::<f64>() / balanced_train.len() as f64;

    // Shift all labels so mean is exactly 0.0
    let balanced_train: Vec<(f64, Vec<f64>)> = balanced_train
        .into_iter()
        .map(|(l, feats)| (l - raw_mean, feats))
        .collect();

    // Now compute stats on centered data
    let label_mean = raw_mean; // store original mean for unscaling at inference
    let label_std = {
        let var = balanced_train
            .iter()
            .map(|(l, _)| l.powi(2)) // mean is 0 so variance = mean of squares
            .sum::<f64>()
            / balanced_train.len() as f64;
        var.sqrt().max(1e-8)
    };
    debug!(
        "          Label mean: {:.2}%  std: {:.2}%",
        label_mean, label_std
    );

    // --- Build tensors ---
    let (train_x, train_y, train_w) =
        to_tensors(train_data, means, stds, label_mean, label_std, &device)?;
    let (test_x, _, _) = to_tensors(test_data, means, stds, label_mean, label_std, &device)?;

    // --- Train ---
    // let mut optimizer = SGD::new(varmap.all_vars(), 0.01)?;
    let mut optimizer = candle_nn::AdamW::new(
        varmap.all_vars(),
        candle_nn::ParamsAdamW {
            lr: 0.001,
            ..Default::default()
        },
    )?;

    for epoch in 0..300 {
        let pred = model.forward(&train_x)?.squeeze(1)?; // [N]
        // MSE base
        let diff = (&pred - &train_y)?;
        let mse = (diff.sqr()? * &train_w)?;

        // relu(-direction) is positive when direction is negative (wrong sign)
        let penalty = (&pred * &train_y)?.neg()?.relu()?;
        let weighted_penalty = ((penalty * &train_w)? * 10.0)?; // [N]

        // Combine first, then reduce to scalar
        let loss = (mse + weighted_penalty)?.mean_all()?; // scalar []
        optimizer.backward_step(&loss)?;

        if epoch % 50 == 0 {
            trace!("  Epoch {}: loss = {:.6}", epoch, loss.to_scalar::<f32>()?);
        }
    }

    // --- Evaluate on test set ---
    let test_pred = model.forward(&test_x)?.squeeze(1)?; // [N]
    let pred_scaled = test_pred.to_vec1::<f32>()?;

    // Unscale back to real % returns
    let pred_vals: Vec<f64> = pred_scaled
        .iter()
        .map(|&v| (v as f64 * label_std) + label_mean)
        .collect();

    let actuals: Vec<f64> = test_data.iter().map(|(l, _)| *l).collect();

    let up_preds = pred_vals.iter().filter(|&&v| v > 0.0).count();
    let down_preds = pred_vals.iter().filter(|&&v| v < 0.0).count();

    trace!(
        "         Pred UP: {}  Pred DOWN: {}  Actual UP: {}  Actual DOWN: {}",
        up_preds,
        down_preds,
        actuals.iter().filter(|&&v| v > 0.0).count(),
        actuals.iter().filter(|&&v| v < 0.0).count(),
    );

    // --- Metrics — same function as LR and RF ---
    let (directional_accuracy, bullish_precision, bearish_precision, mae, r2) =
        log_metrics_from_vecs(&pred_vals, &actuals)?;

    let metrics = ModelMetrics {
        directional_accuracy,
        bullish_precision,
        bearish_precision,
        mae,
        r2,
    };

    // --- Class means (same pattern as RF) ---
    let (mean_down, mean_neutral, mean_up) = compute_class_means(labeled_data);

    // // --- Serialize weights ---
    let weights = varmap_to_bytes(&varmap)?;
    Ok(MlpResult {
        metrics,
        mean_down,
        mean_neutral,
        mean_up,
        weights,
        label_mean,
        label_std,
    })
}

fn balance_labels(data: &[(f64, Vec<f64>)]) -> Vec<(f64, Vec<f64>)> {
    let mut up: Vec<_> = data.iter().filter(|(l, _)| *l > 0.0).cloned().collect();
    let mut down: Vec<_> = data.iter().filter(|(l, _)| *l < 0.0).cloned().collect();

    let mut rng = thread_rng();
    up.shuffle(&mut rng);
    down.shuffle(&mut rng);

    let min_count = up.len().min(down.len());
    up.truncate(min_count);
    down.truncate(min_count);

    // Compute mean absolute value per side
    let mean_up_abs = up.iter().map(|(l, _)| l.abs()).sum::<f64>() / up.len() as f64;
    let mean_down_abs = down.iter().map(|(l, _)| l.abs()).sum::<f64>() / down.len() as f64;

    trace!(
        "         Mean UP magnitude: {:.2}%  Mean DOWN magnitude: {:.2}%",
        mean_up_abs, mean_down_abs
    );

    // Scale UP labels down to match DOWN magnitude
    // so MSE treats both directions equally
    let scale = mean_down_abs / mean_up_abs.max(1e-8);

    trace!("         UP magnitude scale factor: {:.4}", scale);

    let mut balanced = Vec::new();
    balanced.extend(up.into_iter().map(|(l, feats)| (l * scale, feats)));
    balanced.extend(down);

    balanced
}

const HIDDEN1: usize = 32;
const HIDDEN2: usize = 16;
const OUTPUT: usize = 1; // 0=Down, 1=Neutral, 2=Up

pub struct IndicatorMlp {
    fc1: Linear,
    fc2: Linear,
    fc3: Linear,
}

impl IndicatorMlp {
    fn new(n_features: usize, vb: VarBuilder) -> candle_core::Result<Self> {
        Ok(Self {
            fc1: linear(n_features, HIDDEN1, vb.pp("fc1"))?,
            fc2: linear(HIDDEN1, HIDDEN2, vb.pp("fc2"))?,
            fc3: linear(HIDDEN2, OUTPUT, vb.pp("fc3"))?,
        })
    }
}

impl Module for IndicatorMlp {
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let x = self.fc1.forward(x)?.relu()?;
        let x = self.fc2.forward(&x)?.relu()?;
        self.fc3.forward(&x)
    }
}

/// Normalize with pre-computed means/stds (same as LR/RF)
fn normalize_single(features: &[f64], means: &[f64], stds: &[f64]) -> Vec<f32> {
    features
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let std = if stds[i] == 0.0 { 1.0 } else { stds[i] };
            ((v - means[i]) / std) as f32
        })
        .collect()
}

// label f64 → class index u32
// fn label_to_class(label: f64) -> u32 {
//     if label > 0.0 {
//         2
//     }
//     // Up
//     else if label < 0.0 {
//         0
//     }
//     // Down
//     else {
//         1
//     } // Neutral
// }

// to_tensors
pub fn to_tensors(
    data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
    label_mean: f64,
    label_std: f64,
    device: &Device,
) -> Result<(Tensor, Tensor, Tensor)> {
    let n = data.len();
    let x_rows: Vec<Vec<f32>> = data
        .iter()
        .map(|(_, feats)| normalize_single(feats, means, stds))
        .collect();

    // Scale labels to zero mean, unit variance
    let y_vals: Vec<f32> = data
        .iter()
        .map(|(label, _)| ((label - label_mean) / label_std) as f32)
        .collect();

    // Recency weights — oldest: 0.5, newest: 1.5
    let w_vals: Vec<f32> = (0..n).map(|i| 0.5 + (i as f32 / n as f32)).collect();
    let x = Tensor::new(x_rows, device)?;
    let y = Tensor::new(y_vals.as_slice(), device)?;
    let w = Tensor::new(w_vals.as_slice(), device)?;

    Ok((x, y, w))
}

pub fn compute_mlp_metrics(
    preds: &[u32],                 // argmax output (0=Down, 1=Neutral, 2=Up)
    test_data: &[(f64, Vec<f64>)], // raw labels for actuals
) -> Result<ModelMetrics> {
    assert_eq!(preds.len(), test_data.len());

    // Raw actual labels
    let actuals: Vec<f64> = test_data.iter().map(|(label, _)| *label).collect();

    // Map class index → f64 sign value to match LR/RF prediction shape
    // Use mean_up / mean_down if you have them, or simple ±1 as proxy
    let predictions: Vec<f64> = preds
        .iter()
        .map(|&p| match p {
            2 => 1.0,  // Up
            0 => -1.0, // Down
            _ => 0.0,  // Neutral
        })
        .collect();

    // Reuse exact same metrics function as LR and RF
    let (directional_accuracy, bullish_precision, bearish_precision, mae, r2) =
        log_metrics_from_vecs(&predictions, &actuals)?;

    Ok(ModelMetrics {
        directional_accuracy,
        bullish_precision,
        bearish_precision,
        mae,
        r2,
    })
}

/// Average raw label (f64) per class — same pattern as RF.
/// Returns (mean_down, mean_neutral, mean_up)
pub fn compute_class_means(labeled_data: &[(f64, Vec<f64>)]) -> (f64, f64, f64) {
    let mut down_sum = 0.0;
    let mut neutral_sum = 0.0;
    let mut up_sum = 0.0;

    let mut down_count = 0usize;
    let mut neutral_count = 0usize;
    let mut up_count = 0usize;

    for (label, _) in labeled_data {
        if *label > 0.0 {
            up_sum += label;
            up_count += 1;
        } else if *label < 0.0 {
            down_sum += label;
            down_count += 1;
        } else {
            neutral_sum += label;
            neutral_count += 1;
        }
    }

    let mean_down = if down_count > 0 {
        down_sum / down_count as f64
    } else {
        0.0
    };
    let mean_neutral = if neutral_count > 0 {
        neutral_sum / neutral_count as f64
    } else {
        0.0
    };
    let mean_up = if up_count > 0 {
        up_sum / up_count as f64
    } else {
        0.0
    };

    (mean_down, mean_neutral, mean_up)
}

pub fn varmap_to_bytes(varmap: &VarMap) -> Result<Vec<u8>> {
    let data = varmap.data().lock().unwrap();

    let mut tensors: Vec<(String, Vec<f32>)> = data
        .iter()
        .map(|(name, var)| {
            let floats = var.as_tensor().flatten_all()?.to_vec1::<f32>()?;
            Ok((name.clone(), floats))
        })
        .collect::<candle_core::Result<_>>()?;

    tensors.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(bincode::serialize(&tensors)?)
}

pub fn bytes_to_tensors(bytes: &[u8]) -> Result<Vec<(String, Vec<f32>)>> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn predict_mlp(alpha: &TickerAlpha, normalized: Vec<f64>) -> Result<f64> {
    let device = Device::Cpu;

    let weights = alpha.mlp_weights.as_ref().ok_or_else(|| {
        anyhow::anyhow!("No MLP weights in TickerAlpha {}:{}", alpha.key, alpha.n)
    })?;

    let normalized_f32: Vec<f32> = normalized.iter().map(|f| *f as f32).collect();
    let model = bytes_to_varbuilder(weights, alpha.feature_count as usize, &device)?;

    let input = Tensor::new(vec![normalized_f32], &device)?;
    let output = model.forward(&input)?.squeeze(1)?;
    let scaled = output.to_vec1::<f32>()?[0] as f64;

    // Unscale: scaled value → real % return
    let predicted_pct = (scaled * alpha.label_std) + alpha.label_mean;
    // If bearish precision was low in training, suppress weak DOWN signals
    if predicted_pct < 0.0 && alpha.bearish_precision < 0.50 {
        warn!(
            "        Suppressing weak DOWN signal for {}:{} (bearish precision: {:.1}%)",
            alpha.key,
            alpha.n,
            alpha.bearish_precision * 100.0
        );
        return Ok(0.0); // treat as neutral
    }
    trace!(
        "     MLP {}:{} — predicted: {:.2}%",
        alpha.key, alpha.n, predicted_pct
    );

    Ok(predicted_pct) // e.g. +3.7 means predicted +3.7% move
}

pub fn bytes_to_varbuilder(
    bytes: &[u8],
    n_features: usize,
    device: &Device,
) -> Result<IndicatorMlp> {
    let map: Vec<(String, Vec<f32>)> = bincode::deserialize(bytes)?;

    let varmap = VarMap::new();
    {
        let mut data = varmap.data().lock().unwrap();
        for (name, floats) in map {
            // Reconstruct tensor shape from name
            let tensor = match name.as_str() {
                "fc1.weight" => Tensor::from_vec(floats, (HIDDEN1, n_features), device)?,
                "fc1.bias" => Tensor::from_vec(floats, (HIDDEN1,), device)?,
                "fc2.weight" => Tensor::from_vec(floats, (HIDDEN2, HIDDEN1), device)?,
                "fc2.bias" => Tensor::from_vec(floats, (HIDDEN2,), device)?,
                "fc3.weight" => Tensor::from_vec(floats, (OUTPUT, HIDDEN2), device)?,
                "fc3.bias" => Tensor::from_vec(floats, (OUTPUT,), device)?,
                other => return Err(anyhow::anyhow!("Unknown tensor: {}", other)),
            };
            data.insert(name, Var::from_tensor(&tensor)?);
        }
    }

    let vb = VarBuilder::from_varmap(&varmap, DType::F32, device);
    Ok(IndicatorMlp::new(n_features, vb)?)
}

pub fn directional_loss(pred: &Tensor, actual: &Tensor) -> candle_core::Result<Tensor> {
    // MSE base
    let diff = (pred - actual)?;
    let mse = diff.sqr()?.mean_all()?;

    // Directional penalty — pred * actual < 0 means wrong direction
    let direction = (pred * actual)?;

    // relu(-direction) is positive when direction is negative (wrong sign)
    let penalty = direction.neg()?.relu()?.mean_all()?;

    // MSE + 2x directional penalty
    mse + (penalty * 5.0)?
}
