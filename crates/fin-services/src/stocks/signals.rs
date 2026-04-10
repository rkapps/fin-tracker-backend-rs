use std::collections::HashMap;

use crate::stocks::StocksService;
use fin_domain::ticker::{IndicatorWindow, TickerAlpha};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const ZERO: Decimal = dec!(0);
const RSI_OVERSOLD: Decimal = dec!(30);
const RSI_OVERSOLD_35: Decimal = dec!(35);
const RSI_OVERBOUGHT: Decimal = dec!(70);
const RSI_OVERBOUGHT_65: Decimal = dec!(65);
const RSI_PULLBACK_LOW: Decimal = dec!(40);
const RSI_PULLBACK_HIGH: Decimal = dec!(55);
const RSI_RALLY_LOW: Decimal = dec!(45);
const RSI_RALLY_HIGH: Decimal = dec!(60);
const RSI_MEAN_REVERSION: Decimal = dec!(40);

const STOCHASTIC_MEAN_REVERSION: Decimal = dec!(30);
const STOCHASTIC_D_LOWER_THRESHOLD: Decimal = dec!(20);
const STOCHASTIC_D_UPPER_THRESHOLD: Decimal = dec!(80);
const STOCHASTIC_K_LOWER_THRESHOLD: Decimal = dec!(20);
const STOCHASTIC_K_THRESHOLD: Decimal = dec!(50);
const STOCHASTIC_K_UPPER_THRESHOLD: Decimal = dec!(50);

impl StocksService {
    pub(crate) fn calculate_sma_stack(&self, window: &IndicatorWindow) -> Option<String> {
        let s20 = window.curr.sma_20?;
        let s50 = window.curr.sma_50?;
        let s100 = window.curr.sma_100?;
        let s200 = window.curr.sma_200?;

        if s20 > s50 && s50 > s100 && s100 > s200 {
            return Some("SMA Stack Bullish".to_string());
        }
        if s20 < s50 && s50 < s100 && s100 < s200 {
            return Some("SMA Stack Bearish".to_string());
        }

        None
    }

    // Bullish Pullback — SMA50 above SMA200 (uptrend intact), price above SMA50, RSI between 40-55. Healthy pullback within uptrend.
    // Bearish Rally — SMA50 below SMA200 (downtrend intact), price below SMA50, RSI between 45-60. Dead cat bounce within downtrend.
    pub(crate) fn calculate_sma_50(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<Vec<String>> {
        let s50 = window.curr.sma_50?;
        let s200 = window.curr.sma_200?;
        let r_14 = window.curr.rsi_14?;

        let mut signals = Vec::new();
        if price > s50 {
            signals.push("Above SMA50".to_string());
        }
        if price < s50 {
            signals.push("Below SMA50".to_string());
        }

        if s50 > s200 && price > s50 && r_14 >= RSI_PULLBACK_LOW && r_14 <= RSI_PULLBACK_HIGH {
            signals.push("Bullish Pullback".to_string());
        }
        if s50 < s200 && price < s50 && r_14 > RSI_RALLY_LOW && r_14 <= RSI_RALLY_HIGH {
            signals.push("Bearish Rally".to_string());
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    pub(crate) fn calculate_sma_crossover(&self, window: &IndicatorWindow) -> Option<String> {
        let s50 = window.curr.sma_50?;
        let s200 = window.curr.sma_200?;

        let ps50 = window.prev.sma_50?;
        let ps200 = window.prev.sma_200?;

        if s50 > s200 && ps50 <= ps200 {
            return Some("Golden Cross".to_string());
        }
        if s50 < s200 && ps50 >= ps200 {
            return Some("Death Cross".to_string());
        }

        None
    }

    pub(crate) fn calculate_macd_crossover(&self, window: &IndicatorWindow) -> Option<String> {
        let m = window.curr.macd?;
        let s = window.curr.macd_signal?;
        let pm = window.prev.macd?;
        let ps = window.prev.macd_signal?;

        if m > s && pm <= ps {
            return Some("MACD Bullish Crossover".to_string());
        }
        if m < s && pm >= ps {
            return Some("MACD Bearish Crossover".to_string());
        }

        None
    }

    //  MACD Histogram Expanding — curr histogram greater than prev histogram on same side of zero.
    //  Momentum increasing in current direction.
    //  MACD Histogram Weakening — curr histogram smaller than prev histogram, same side of zero.
    //  Momentum fading before crossover happens.
    pub(crate) fn calculate_macd_histogram(&self, window: &IndicatorWindow) -> Option<String> {
        let h = window.curr.macd_histogram?;
        let ph = window.prev.macd_histogram?;

        if ((h > ZERO && ph > ZERO) || (h < ZERO && ph < ZERO)) && h > ph {
            return Some("MACD Histogram Expanding".to_string());
        }
        if ((h > ZERO && ph > ZERO) || (h < ZERO && ph < ZERO)) && h < ph {
            return Some("MACD Histogram Weakening".to_string());
        }

        None
    }

    // BB Squeeze — band width as percentage of middle band below threshold. Contraction preceding a move.
    pub(crate) fn calculate_bollinger_bands(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<Vec<String>> {
        let bb_upper = window.curr.bb_upper?;
        let bb_middle = window.curr.bb_middle?;
        let bb_lower = window.curr.bb_lower?;

        let mut signals = Vec::new();
        if price > bb_upper {
            signals.push("BB Breakout Upper".to_string());
        }
        if price < bb_lower {
            signals.push("BB Breakout Lower".to_string());
        }

        if bb_middle > Decimal::from(0) {
            let width = (bb_upper - bb_lower) / bb_middle * Decimal::from(100);
            if width < Decimal::from(4) {
                signals.push("BB Squeeze".to_string());
            }
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    pub(crate) fn calculate_stochastic(&self, window: &IndicatorWindow) -> Option<String> {
        let k = window.curr.stochastic_k_14?;
        let d = window.curr.stochastic_d?;
        let pk = window.prev.stochastic_k_14?;
        let pd = window.prev.stochastic_d?;

        if k > d && pk <= pd {
            return Some("Stochastic Bullish".to_string());
        }
        if k < d && pk >= pd {
            return Some("Stochastic Bearish".to_string());
        }

        None
    }

    // RSI Recovering from Oversold — prev RSI below 30, curr RSI above 30. The turn has begun, more actionable than just being oversold.
    // RSI Multi-Period Oversold — rsi_10, rsi_14, and rsi_26 all below 35 simultaneously. Strong agreement across timeframes.
    pub(crate) fn calculate_rsi(&self, window: &IndicatorWindow) -> Option<Vec<String>> {
        let r_10 = window.curr.rsi_10?;
        let r_14 = window.curr.rsi_14?;
        let r_26 = window.curr.rsi_26?;
        let pr_14 = window.prev.rsi_14?;

        let mut signals = Vec::new();

        // RSI
        if r_14 < RSI_OVERSOLD {
            signals.push("RSI Oversold".to_string());
        }
        if pr_14 < RSI_OVERSOLD && r_14 > RSI_OVERSOLD {
            signals.push("RSI Recovering from Oversold".to_string());
        }
        if r_10 < RSI_OVERSOLD_35 && r_14 < RSI_OVERSOLD_35 && r_26 < RSI_OVERSOLD_35 {
            signals.push("RSI Multi-period Oversold".to_string());
        }

        if r_14 > RSI_OVERBOUGHT {
            signals.push("RSI Overbought".to_string());
        }
        if pr_14 > RSI_OVERBOUGHT && r_14 < RSI_OVERBOUGHT {
            signals.push("RSI Recovering from Overbought".to_string());
        }
        if r_10 > RSI_OVERBOUGHT_65 && r_14 > RSI_OVERBOUGHT_65 && r_26 > RSI_OVERBOUGHT_65 {
            signals.push("RSI Multi-period Overbought".to_string());
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    // Oversold Confluence — RSI14 below 35, price below lower BB, Stochastic %D below 20. All three agree on oversold.
    // Overbought Confluence — RSI14 above 65, price above upper BB, Stochastic %D above 80. All three agree on overbought.
    pub(crate) fn calculate_confluence(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<Vec<String>> {
        let rsi_14 = window.curr.rsi_14?;
        let bb_lower = window.curr.bb_lower?;
        let bb_upper = window.curr.bb_upper?;
        let s_d = window.curr.stochastic_d?;
        let s_k = window.curr.stochastic_k_14?;

        let mut signals = Vec::new();

        // RSI
        if rsi_14 < RSI_OVERSOLD_35 && price < bb_lower && s_d < STOCHASTIC_D_LOWER_THRESHOLD {
            signals.push("Oversold Confluence".to_string());

            if s_k > s_d {
                signals.push("Oversold Reversal Setup".to_string());
            }
        }
        if rsi_14 > RSI_OVERBOUGHT_65 && price > bb_upper && s_d > STOCHASTIC_D_UPPER_THRESHOLD {
            signals.push("Overbought Confluence".to_string());
            if s_k < s_d {
                signals.push("Overbought Reversal Setup".to_string());
            }
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    // Price above SMA20, SMA50, and SMA200 — above all key averages
    // SMA Stack Bullish — full trend alignment
    // MACD histogram positive and greater than prev (expanding) — momentum increasing
    // Stochastic %K above 50 and %K greater than %D — momentum confirmed and rising
    pub(crate) fn calculate_momentum(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<String> {
        let s20 = window.curr.sma_20?;
        let s50 = window.curr.sma_50?;
        let s100 = window.curr.sma_100?;
        let s200 = window.curr.sma_200?;
        let h = window.curr.macd_histogram?;
        let ph = window.prev.macd_histogram?;
        let k = window.curr.stochastic_k_14?;
        let d = window.curr.stochastic_d?;
        if price > s20
            && price > s50
            && price > s100
            && price > s200
            && h > ZERO
            && h > ph
            && k > STOCHASTIC_K_THRESHOLD
            && k > d
        {
            return Some("Momentum Breakout".to_string());
        }
        None
    }

    // Price at or above upper BB (or at/below lower BB for bearish exhaustion) — price is stretched
    // RSI above 65 — momentum has been strong
    // MACD histogram positive but smaller than prev (weakening) — momentum fading
    // Stochastic %K above 80 but %K less than %D — stochastic turning over at elevated levels
    pub(crate) fn calculate_trend_exhaustion(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<String> {
        let bb_upper = window.curr.bb_upper?;
        let bb_lower = window.curr.bb_lower?;
        let r_14 = window.curr.rsi_14?;
        let h = window.curr.macd_histogram?;
        let ph = window.prev.macd_histogram?;
        let k = window.curr.stochastic_k_14?;
        let d = window.curr.stochastic_d?;

        if price >= bb_upper
            && r_14 > RSI_OVERBOUGHT_65
            && h > ZERO
            && h < ph
            && k > STOCHASTIC_K_UPPER_THRESHOLD
            && k < d
        {
            return Some("Bullish Trend Exhaustion".to_string());
        }
        if price < bb_lower
            && r_14 < RSI_OVERSOLD_35
            && h < ZERO
            && h > ph
            && k < STOCHASTIC_K_LOWER_THRESHOLD
            && k > d
        {
            return Some("Bearish Trend Exhaustion".to_string());
        }

        None
    }

    // Mean Reversion Candidate — broader, earlier signal. RSI below 40 rather than 35, no BB requirement, but adds the ATR condition to filter out stocks still in active decline.
    pub(crate) fn calculate_mean_reversion(
        &self,
        price: Decimal,
        window: &IndicatorWindow,
    ) -> Option<String> {
        let r_14 = window.curr.rsi_14?;
        let s20 = window.curr.sma_20?;
        let k = window.curr.stochastic_k_14?;
        let atr = window.curr.atr?;
        let prev_atr = window.prev.atr?;

        if r_14 < RSI_MEAN_REVERSION
            && price < s20
            && k < STOCHASTIC_MEAN_REVERSION
            && atr <= prev_atr
        {
            return Some("Mean Reversion Candidate".to_string());
        }

        None
    }

    // Volatility Expanding, Volatility Contracting
    pub(crate) fn calculate_volatility(&self, window: &IndicatorWindow) -> Option<String> {
        let atr = window.curr.atr?;
        let prev_atr = window.prev.atr?;

        if atr > prev_atr {
            return Some("Volatility Expanding".to_string());
        }
        if atr < prev_atr {
            return Some("Volatility Contracting".to_string());
        }

        None
    }

    pub(crate) fn calculate_ml_signals(
        &self,
        lr_returns: &HashMap<String, f64>,
        rf_returns: Option<&HashMap<String, f64>>,
        mlp_returns: &HashMap<String, f64>,
        mlp_alphas: &HashMap<String, &TickerAlpha>,
        min_accuracy: f64,
    ) -> Vec<String> {
        let mut signals = Vec::new();
        let rf_available = rf_returns.is_some();

        for period in ["5", "10", "20", "60"] {
            let lr = lr_returns.get(period).copied().unwrap_or(0.0);
            let rf = rf_returns
                .and_then(|m| m.get(period))
                .copied()
                .unwrap_or(0.0);
            let mlp = mlp_returns.get(period).copied().unwrap_or(0.0);

            // --- MLP only — unique signal not captured by LR/RF ---
            if let Some(alpha) = mlp_alphas.get(period) {
                if mlp != 0.0 && alpha.directional_accuracy >= min_accuracy {
                    let bullish_ok = mlp > 0.0 && alpha.bullish_precision >= 0.55;
                    let bearish_ok = mlp < 0.0 && alpha.bearish_precision >= 0.55;

                    if bullish_ok {
                        signals.push(format!(
                            "MLP{} Bullish ({:.1}%  precision: {:.0}%)",
                            period,
                            mlp,
                            alpha.bullish_precision * 100.0
                        ));
                    } else if bearish_ok {
                        signals.push(format!(
                            "MLP{} Bearish ({:.1}%  precision: {:.0}%)",
                            period,
                            mlp,
                            alpha.bearish_precision * 100.0
                        ));
                    }
                }
            }

            // --- Confluence — only emit when models agree ---
            let lr_bullish = lr > 0.5;
            let lr_bearish = lr < -0.5;
            let rf_bullish = rf_available && rf > 0.0;
            let rf_bearish = rf_available && rf < 0.0;
            let mlp_bullish = mlp > 0.0
                && mlp_alphas
                    .get(period)
                    .map(|a| a.bullish_precision >= 0.55 && a.directional_accuracy >= min_accuracy)
                    .unwrap_or(false);
            let mlp_bearish = mlp < 0.0
                && mlp_alphas
                    .get(period)
                    .map(|a| a.bearish_precision >= 0.55 && a.directional_accuracy >= min_accuracy)
                    .unwrap_or(false);

            let bull_votes = [lr_bullish, rf_bullish, mlp_bullish]
                .iter()
                .filter(|&&v| v)
                .count();
            let bear_votes = [lr_bearish, rf_bearish, mlp_bearish]
                .iter()
                .filter(|&&v| v)
                .count();

            let total = 2 + if rf_available { 1 } else { 0 };

            // Only emit confluence when at least 2 models agree
            if bull_votes >= 2 {
                signals.push(format!(
                    "ML{} Bullish — {}/{} Confirmed",
                    period, bull_votes, total
                ));
            } else if bear_votes >= 2 {
                signals.push(format!(
                    "ML{} Bearish — {}/{} Confirmed",
                    period, bear_votes, total
                ));
            }
            // Skip conflicting and single-model signals — too noisy
        }

        // --- Cross-period summary — single most important signal ---
        let trusted_mlp_bullish = |p: &str| {
            mlp_returns.get(p).copied().unwrap_or(0.0) > 0.0
                && mlp_alphas
                    .get(p)
                    .map(|a| a.bullish_precision >= 0.55 && a.directional_accuracy >= min_accuracy)
                    .unwrap_or(false)
        };
        let trusted_mlp_bearish = |p: &str| {
            mlp_returns.get(p).copied().unwrap_or(0.0) < 0.0
                && mlp_alphas
                    .get(p)
                    .map(|a| a.bearish_precision >= 0.55 && a.directional_accuracy >= min_accuracy)
                    .unwrap_or(false)
        };

        let all_lr_bullish = ["5", "10", "20", "60"]
            .iter()
            .all(|p| lr_returns.get(*p).copied().unwrap_or(0.0) > 0.5);
        let all_lr_bearish = ["5", "10", "20", "60"]
            .iter()
            .all(|p| lr_returns.get(*p).copied().unwrap_or(0.0) < -0.5);
        let all_mlp_bullish = ["5", "10", "20", "60"]
            .iter()
            .all(|p| trusted_mlp_bullish(p));
        let all_mlp_bearish = ["5", "10", "20", "60"]
            .iter()
            .all(|p| trusted_mlp_bearish(p));
        let all_rf_bullish = !rf_available
            || ["5", "10", "20", "60"]
                .iter()
                .all(|p| rf_returns.unwrap().get(*p).copied().unwrap_or(0.0) > 0.0);
        let all_rf_bearish = !rf_available
            || ["5", "10", "20", "60"]
                .iter()
                .all(|p| rf_returns.unwrap().get(*p).copied().unwrap_or(0.0) < 0.0);

        if all_lr_bullish && all_rf_bullish && all_mlp_bullish {
            signals.push("ML Strong Bull — All Periods All Models Confirmed".to_string());
        } else if all_lr_bullish && all_rf_bullish {
            signals.push("ML Strong Bull — All Periods Confirmed".to_string());
        } else if all_lr_bullish && all_mlp_bullish {
            signals.push("ML Strong Bull — All Periods LR+MLP Confirmed".to_string());
        } else if all_lr_bearish && all_rf_bearish && all_mlp_bearish {
            signals.push("ML Strong Bear — All Periods All Models Confirmed".to_string());
        } else if all_lr_bearish && all_rf_bearish {
            signals.push("ML Strong Bear — All Periods Confirmed".to_string());
        } else if all_lr_bearish && all_mlp_bearish {
            signals.push("ML Strong Bear — All Periods LR+MLP Confirmed".to_string());
        }

        signals
    }
    // Before:                          After:
    // ────────────────────────────     ────────────────────────────
    // LR5/10/20/60 signals (4)    ←   removed — LR captured in confluence
    // RF5/10/20/60 signals (4)    ←   removed — RF captured in confluence
    // MLP Weak Bullish/Bearish    ←   removed — below precision threshold, not actionable
    // ML Conflicting              ←   removed — noise, no clear action
    // ML Caution LR/RF mismatch   ←   removed — too granular

    // Kept:
    // MLP{period} Bullish/Bearish      ← unique magnitude + precision info
    // ML{period} Bullish/Bearish N/M   ← confluence per period
    // ML Strong Bull/Bear summary      ← most important cross-period signal
}
