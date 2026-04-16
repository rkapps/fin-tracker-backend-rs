use rust_decimal::Decimal;

use crate::tickers::TickerIndicator;

#[derive(Debug, Clone)]
pub struct IndicatorSnapshot {
    pub price: Option<Decimal>,
    pub rsi_10: Option<Decimal>,
    pub rsi_14: Option<Decimal>,
    pub rsi_26: Option<Decimal>,
    pub sma_20: Option<Decimal>,
    pub sma_50: Option<Decimal>,
    pub sma_100: Option<Decimal>,
    pub sma_200: Option<Decimal>,
    pub macd: Option<Decimal>,
    pub macd_signal: Option<Decimal>,
    pub macd_histogram: Option<Decimal>,
    pub bb_upper: Option<Decimal>,
    pub bb_middle: Option<Decimal>,
    pub bb_lower: Option<Decimal>,
    pub stochastic_k_14: Option<Decimal>,
    pub stochastic_d: Option<Decimal>,
    pub atr: Option<Decimal>,
    pub volume_ratio: Option<Decimal>,
}

impl From<&TickerIndicator> for IndicatorSnapshot {
    fn from(value: &TickerIndicator) -> Self {
        let values = value.values.clone();
        IndicatorSnapshot {
            price: values.get("price").copied(),
            sma_20: values.get("sma_20").copied(),
            sma_50: values.get("sma_50").copied(),
            sma_100: values.get("sma_100").copied(),
            sma_200: values.get("sma_200").copied(),
            macd: values.get("macd").copied(),
            macd_signal: values.get("macd_signal").copied(),
            macd_histogram: values.get("macd_histogram").copied(),
            bb_upper: values.get("bb_upper").copied(),
            bb_middle: values.get("bb_middle").copied(),
            bb_lower: values.get("bb_lower").copied(),
            stochastic_k_14: values.get("stochastic_k_14").copied(),
            stochastic_d: values.get("stochastic_d").copied(),
            rsi_10: values.get("rsi_10").copied(),
            rsi_14: values.get("rsi_14").copied(),
            rsi_26: values.get("rsi_26").copied(),
            atr: values.get("atr").copied(),
            volume_ratio: values.get("volume_ratio").copied(),
        }
    }
}
