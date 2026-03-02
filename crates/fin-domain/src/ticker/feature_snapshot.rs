use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::{
    ticker::IndicatorSnapshot,
    utils::dec_utils::decimal_to_float,
};

const RSI_DEFAULT: f64 = 45.0;

pub struct FeatureSnapshot {
    pub price: f64,
    pub pr_sma20_pct: f64,
    pub pr_sma50_pct: f64,
    pub pr_sma200_pct: f64,
    pub rsi_10: f64,
    pub rsi_14: f64,
    pub rsi_26: f64,
    pub macd_histogram: f64,
    pub stochastic_k: f64,
    pub stochastic_d: f64,
    pub volume_ratio: f64,
    pub atr_price_pct: f64,
    pub bb_width_pct: f64,
    pub bb_position_pct: f64,
}

impl FeatureSnapshot {

    pub fn from_indcator(price: Decimal, value: IndicatorSnapshot) -> Self {
        Self {
            price: decimal_to_float(value.price, 0.0),
            pr_sma20_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_20),
            pr_sma50_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_50),
            pr_sma200_pct: FeatureSnapshot::price_vs_sma_pct(price, value.sma_200),
            rsi_10: decimal_to_float(value.rsi_10, RSI_DEFAULT),
            rsi_14: decimal_to_float(value.rsi_14, RSI_DEFAULT),
            rsi_26: decimal_to_float(value.rsi_26, RSI_DEFAULT),
            macd_histogram: decimal_to_float(value.macd_histogram, 0.0),
            stochastic_k: decimal_to_float(value.stochastic_k_14, 50.0),
            stochastic_d: decimal_to_float(value.stochastic_d, 50.0),
            volume_ratio: decimal_to_float(value.volume_ratio, 1.0),
            atr_price_pct: FeatureSnapshot::atr_as_price_pct(value.atr, price),
            bb_width_pct: FeatureSnapshot::bb_width_pct(value.bb_upper, value.bb_middle, value.bb_lower),
            bb_position_pct: FeatureSnapshot::bb_position_pct(price, value.bb_upper, value.bb_lower)
        }
    }

    // price as percentage of sma_50, sma_200
    pub fn price_vs_sma_pct(price: Decimal, sma: Option<Decimal>) -> f64 {
        match (price, sma) {
            (p, Some(s)) if s > Decimal::ZERO => {
                ((p - s) / s * Decimal::from(100)).to_f64().unwrap_or(0.0)
            }
            _ => 0.0,
        }
    }

    // atr as percentage of price
    pub fn atr_as_price_pct(atr: Option<Decimal>, price: Decimal) -> f64 {
        match (atr, price) {
            (Some(atr), (p)) if atr > Decimal::ZERO => {
                ((atr) / p * Decimal::from(100)).to_f64().unwrap_or(0.0)
            }
            _ => 1.0,
        }
    }

    // bb_width 
    pub fn bb_width_pct(
        bb_upper: Option<Decimal>,
        bb_middle: Option<Decimal>,
        bb_lower: Option<Decimal>,
    ) -> f64 {
        match (bb_upper, bb_middle, bb_lower) {
            (Some(bb_u), Some(bb_m), Some(bb_l))
                if bb_u > Decimal::ZERO
                    && bb_m > Decimal::ZERO
                    && bb_l > Decimal::ZERO =>
            {
                (bb_u - bb_l / bb_m * Decimal::from(100)).to_f64().unwrap_or(0.0)
            }
            _ => 1.0,
        }
    }

    // bb_position
    pub fn bb_position_pct(
        price: Decimal,
        bb_upper: Option<Decimal>,
        bb_lower: Option<Decimal>,
    ) -> f64 {
        match (price, bb_upper, bb_lower) {
            (p, Some(bb_u), Some(bb_l))
                if p > Decimal::ZERO 
                    && bb_u > Decimal::ZERO
                    && bb_l > Decimal::ZERO =>
            {
                ((p - bb_l) / (bb_u -bb_l) * Decimal::from(100)).to_f64().unwrap_or(0.0)
            }
            _ => 50.0,
        }
    }
    
}
