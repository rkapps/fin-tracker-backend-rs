use crate::{
    tickers::IndicatorSnapshot,
    utils::{dec_utils::decimal_to_float, float_utils::float_round_to_6_decimals},
};
use anyhow::Result;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use tracing::trace;

const RSI_DEFAULT: f64 = 45.0;
const STOCH_DEFAULT: f64 = 50.0;

pub struct FeatureSnapshot {
    pub price: f64,

    // RSI — keep rsi_14 as primary, replace raw rsi_10/rsi_26 with divergence
    pub rsi_14: f64,
    pub rsi_divergence: f64, // FIX 1: rsi_10 - rsi_26 (short vs long momentum gap)

    // Price vs moving averages
    pub pr_sma20_pct: f64,
    pub pr_sma50_pct: f64,
    pub pr_sma200_pct: f64,

    // Bollinger Bands
    pub bb_position_pct: f64,
    pub bb_width_pct: f64,

    // MACD
    pub macd_histogram: f64,
    pub macd_histogram_slope: f64, // FIX 3: direction of MACD momentum (today - yesterday)

    // Stochastic — replace raw k/d with primary + divergence
    pub stochastic_k: f64,
    pub stoch_divergence: f64, // FIX 1: stoch_k - stoch_d (momentum crossover signal)

    // Volatility & volume
    pub atr_price_pct: f64,
    pub volume_ratio: f64,
}

impl FeatureSnapshot {
    /// Standard constructor from a single IndicatorSnapshot.
    /// macd_histogram_slope defaults to 0.0 — use from_indicator_with_prev
    /// when you have the previous day's snapshot available.
    pub fn from_indicator(value: &IndicatorSnapshot) -> Result<Self> {
        Self::from_indicator_with_prev(value, None)
    }

    /// Preferred constructor — pass previous day's snapshot to compute
    /// macd_histogram_slope accurately.
    pub fn from_indicator_with_prev(
        value: &IndicatorSnapshot,
        prev: Option<&IndicatorSnapshot>,
    ) -> Result<Self> {
        let Some(price) = value.price else {
            return Err(anyhow::anyhow!("Price is not available"));
        };

        let rsi_10 = decimal_to_float(value.rsi_10, RSI_DEFAULT);
        let rsi_26 = decimal_to_float(value.rsi_26, RSI_DEFAULT);

        let stoch_k = decimal_to_float(value.stochastic_k_14, STOCH_DEFAULT);
        let stoch_d = decimal_to_float(value.stochastic_d, STOCH_DEFAULT);

        let macd_histogram = decimal_to_float(value.macd_histogram, 0.0);

        // FIX 3: slope = today - yesterday; 0.0 if no previous snapshot
        let macd_histogram_slope = prev
            .map(|p| macd_histogram - decimal_to_float(p.macd_histogram, 0.0))
            .unwrap_or(0.0);

        Ok(Self {
            price: decimal_to_float(value.price, 0.0),

            // FIX 2: drop raw rsi_10 / rsi_26, keep rsi_14 + divergence
            rsi_14: decimal_to_float(value.rsi_14, RSI_DEFAULT),
            rsi_divergence: rsi_10 - rsi_26,

            pr_sma20_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_20),
            pr_sma50_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_50),
            pr_sma200_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_200),

            bb_position_pct: FeatureSnapshot::bb_position_pct(
                price,
                value.bb_upper,
                value.bb_lower,
            ),
            bb_width_pct: FeatureSnapshot::bb_width_pct(
                value.bb_upper,
                value.bb_middle,
                value.bb_lower,
            ),

            macd_histogram,
            macd_histogram_slope,

            // FIX 2: drop raw stoch_d, keep stoch_k + divergence
            stochastic_k: stoch_k,
            stoch_divergence: stoch_k - stoch_d,

            atr_price_pct: FeatureSnapshot::atr_as_price_pct(value.atr, price),
            volume_ratio: decimal_to_float(value.volume_ratio, 1.0),
        })
    }

    /// Returns the feature vector fed into Linfa / Candle.
    /// Order must stay stable — changing order breaks trained models.
    pub fn values(&self) -> Vec<f64> {
        vec![
            float_round_to_6_decimals(self.rsi_14),               // 0
            float_round_to_6_decimals(self.rsi_divergence),       // 1  ← replaces rsi_10 + rsi_26
            float_round_to_6_decimals(self.pr_sma20_pct),         // 2
            float_round_to_6_decimals(self.pr_sma50_pct),         // 3
            float_round_to_6_decimals(self.pr_sma200_pct),        // 4
            float_round_to_6_decimals(self.bb_position_pct),      // 5
            float_round_to_6_decimals(self.bb_width_pct),         // 6
            float_round_to_6_decimals(self.macd_histogram),       // 7
            float_round_to_6_decimals(self.macd_histogram_slope), // 8  ← new
            float_round_to_6_decimals(self.stochastic_k),         // 9
            float_round_to_6_decimals(self.stoch_divergence),     // 10 ← replaces stochastic_d
            float_round_to_6_decimals(self.atr_price_pct),        // 11
            float_round_to_6_decimals(self.volume_ratio),         // 12
        ]
    }

    /// Human-readable feature names — keep in sync with values() order.
    pub fn feature_names() -> Vec<&'static str> {
        vec![
            "rsi_14",
            "rsi_divergence",
            "pr_sma20_pct",
            "pr_sma50_pct",
            "pr_sma200_pct",
            "bb_position_pct",
            "bb_width_pct",
            "macd_histogram",
            "macd_histogram_slope",
            "stochastic_k",
            "stoch_divergence",
            "atr_price_pct",
            "volume_ratio",
        ]
    }

    // ── helpers ─────────────────────────────────────────────────────────────

    pub fn price_vs_sma_pct(price: Decimal, sma: Option<Decimal>) -> f64 {
        match (price, sma) {
            (p, Some(s)) if s > Decimal::ZERO => {
                ((p - s) / s * Decimal::from(100)).to_f64().unwrap_or(0.0)
            }
            _ => 0.0,
        }
    }

    pub fn atr_as_price_pct(atr: Option<Decimal>, price: Decimal) -> f64 {
        match (atr, price) {
            (Some(atr), p) if atr > Decimal::ZERO && price > Decimal::ZERO => {
                (atr / p * Decimal::from(100)).to_f64().unwrap_or(1.0)
            }
            _ => 1.0,
        }
    }

    pub fn bb_width_pct(
        bb_upper: Option<Decimal>,
        bb_middle: Option<Decimal>,
        bb_lower: Option<Decimal>,
    ) -> f64 {
        trace!(
            "bb_upper: {:?} bb_middle: {:?} bb_lower: {:?}",
            bb_upper, bb_middle, bb_lower
        );

        match (bb_upper, bb_middle, bb_lower) {
            (Some(u), Some(m), Some(l))
                if u > Decimal::ZERO && m > Decimal::ZERO && l > Decimal::ZERO =>
            {
                ((u - l) / m * Decimal::from(100)).to_f64().unwrap_or(1.0)
            }
            _ => 1.0,
        }
    }

    pub fn bb_position_pct(
        price: Decimal,
        bb_upper: Option<Decimal>,
        bb_lower: Option<Decimal>,
    ) -> f64 {
        trace!(
            "price: {:?} bb_upper: {:?} bb_lower: {:?}",
            price, bb_upper, bb_lower
        );

        match (price, bb_upper, bb_lower) {
            (p, Some(u), Some(l))
                if p > Decimal::ZERO
                    && u > Decimal::ZERO
                    && l > Decimal::ZERO
                    && u - l > Decimal::ZERO =>
            {
                ((p - l) / (u - l) * Decimal::from(100))
                    .to_f64()
                    .unwrap_or(50.0)
            }
            _ => 50.0,
        }
    }
}
