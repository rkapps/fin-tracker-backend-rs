use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use storage_core::core::RepoModel;
use crate::ticker::deserialize_flexible_datetime;
use crate::ticker::{serialize_as_bson_datetime};
use crate::ticker::indicator_serde;

use crate::ticker::TICKER_INDICATOR_COLLECTION_NAME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerIndicator {
    pub id: String,

    pub symbol: String,
    #[serde(
        deserialize_with = "deserialize_flexible_datetime",
        serialize_with = "serialize_as_bson_datetime"
    )]    
    pub date: DateTime<Utc>,

    #[serde(with = "indicator_serde")]
    pub values: HashMap<String, Decimal>, // "sma_20": 182.3, "rsi_14": 68.4 etc
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct IndicatorMetadata {
//     pub symbol: String,
//     pub r#type: String,
//     pub exchange: String,
//     pub granularity: String,
// }

// Indicator type constants
pub mod indicator_type {
    pub const SMA: &str = "sma";
    pub const EMA: &str = "ema";
    pub const RSI: &str = "rsi";
    pub const MACD: &str = "macd";
    pub const MACD_SIGNAL: &str = "macd_signal";
    pub const MACD_HISTOGRAM: &str = "macd_histogram";
    pub const BB_UPPER: &str = "bb_upper";
    pub const BB_MIDDLE: &str = "bb_middle";
    pub const BB_LOWER: &str = "bb_lower";
    pub const ATR: &str = "atr";
    pub const STOCHASTIC_K: &str = "stochastic_k";  // %K (fast line)
    pub const STOCHASTIC_D: &str = "stochastic_d";  // %D (slow/signal line)
    pub const VOLUME_RATIO: &str = "volume_ratio";
}

impl RepoModel<String> for TickerIndicator {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn collection(&self) -> &'static str {
        TICKER_INDICATOR_COLLECTION_NAME
    }
}

impl TickerIndicator {

    pub fn new(
        date: DateTime<Utc>,
        symbol: &str,
        // exchange: &str,
        // granularity: &str,
        values: HashMap<String, Decimal>
    ) -> TickerIndicator {
        Self {
            id: format!("{}_{}", symbol, date.format("%Y%m%d")),
            symbol: symbol.to_string(),
            // metadata: IndicatorMetadata {
            //     symbol: symbol.to_string(),
            //     exchange: exchange.to_string(),
            //     r#type: indicator_type::SMA.to_string(),
            //     granularity: granularity.to_string(),
            // },
            date,
            values
        }
    }

}
