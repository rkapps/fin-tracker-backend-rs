use anyhow::Result;
use fin_domain::ticker::{FeatureSnapshot, IndicatorSnapshot, TickerIndicator};
use rust_decimal::prelude::ToPrimitive;
use tracing::trace;

pub fn build_labels(indicators: &[TickerIndicator], period: usize) -> Result<Vec<(f64, Vec<f64>)>> {
    let mut labels: Vec<(f64, Vec<f64>)> = Vec::new();
    let length = indicators.len();

    for index in 1..length {
        if index + period >= length {
            continue;
        }

        let isnapshot = IndicatorSnapshot::from(&indicators[index]);
        let prev_snapshot = Some(IndicatorSnapshot::from(&indicators[index - 1]));
        let fsnapshot =
            FeatureSnapshot::from_indicator_with_prev(&isnapshot, prev_snapshot.as_ref())?;

        // get the future period snapshot
        let future = IndicatorSnapshot::from(&indicators[index + period]);

        let Some(current_price) = isnapshot.price else {
            continue;
        };
        let Some(future_price) = future.price else {
            continue;
        };
        if current_price.is_zero() {
            continue;
        }

        let return_pct = ((future_price - current_price) / current_price)
            .to_f64()
            .unwrap_or(0.0)
            * 100.0;

        trace!(
            "Price: {:.2} - {:.2}   pct: {:.2}   --RSI: {:?}:{:?}:{:?} - divergence: {:?}   --SMA: {:?}:{:?} - : {:?} {:?}",
            current_price,
            future_price,
            return_pct,
            isnapshot.rsi_10,
            isnapshot.rsi_14,
            isnapshot.rsi_26,
            fsnapshot.values().get(1).unwrap(),
            // isnapshot.sma_20,
            isnapshot.sma_50,
            isnapshot.sma_200,
            // fsnapshot.values().get(2).unwrap(),
            fsnapshot.values().get(3).unwrap(),
            fsnapshot.values().get(4).unwrap()

        );

        let mut tvalues = Vec::new();
        tvalues.push(fsnapshot.values().get(1).unwrap().clone());
        tvalues.push(fsnapshot.values().get(3).unwrap().clone());

        labels.push((return_pct, tvalues));

    }

    Ok(labels)
}
