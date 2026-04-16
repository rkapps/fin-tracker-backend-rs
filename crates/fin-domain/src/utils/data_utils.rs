use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use rust_decimal::Decimal;

use crate::{
    tickers::{Ticker, TickerHistory},
    utils::date_utils::same_date,
};

pub fn format_market_cap(mcap: Option<i64>) -> String {
    if let Some(cap) = mcap {
        if cap >= 1_000_000_000_000 {
            format!("${:.2}T", cap as f64 / 1_000_000_000_000.0)
        } else if cap >= 1_000_000_000 {
            format!("${:.2}B", cap as f64 / 1_000_000_000.0)
        } else if cap >= 1_000_000 {
            format!("${:.2}M", cap as f64 / 1_000_000_000.0)
        } else {
            format!("${}", cap)
        }
    } else {
        String::new()
    }
}

pub fn market_cap_range(market_cap: Option<i64>) -> (i64, i64) {
    match market_cap.unwrap_or(0) {
        c if c >= 200_000_000_000 => (1_000_000_000_000, i64::MAX), // Mega: >$200B
        c if c >= 10_000_000_000 => (10_000_000_000, 200_000_000_000), // Large: $10B-$200B
        c if c >= 2_000_000_000 => (2_000_000_000, 10_000_000_000), // Mid: $2B-$10B
        c if c >= 200_000_000 => (200_000_000, 2_000_000_000),      // Small: $250M-$2B
        _ => (0, 200_000_000),                                      // micro: <$200M
    }
}

pub fn market_cap_label_range(market_cap_label: Option<String>) -> (i64, i64) {
    match market_cap_label.unwrap_or("".to_string()) {
        val if val == "mega" => (200_000_000_000, i64::MAX), // Mega: >$200B
        val if val == "large" => (10_000_000_000, 200_000_000_000), // Large: $10B-$200B
        val if val == "mid" => (2_000_000_000, 10_000_000_000), // Mid: $2B-$10B
        val if val == "small" => (250_000_000, 2_000_000_000), // Small: $250M-$2B
        _ => (0, 250_000_000),                                           // micro: <$200M
    }
}

pub fn market_cap_label(cap: Option<i64>) -> String {
    let cap_str = match cap {
        Some(cap) if cap > 1_000_000_000_000 => "mega cap",
        Some(cap) if cap > 10_000_000_000 => "large cap",
        Some(cap) if cap > 2_000_000_000 => "mid cap",
        _ => "small cap",
    };
    cap_str.to_string()
}

// returns the date for the period "1W", "1M", "3M"...etc
pub fn get_period_start(period: &str) -> Option<DateTime<Utc>> {
    let now = Utc::now();
    match period {
        "1W" => Some(now - Duration::days(7)),
        "1M" => Some(now - Duration::days(30)),
        "3M" => Some(now - Duration::days(90)),
        "6M" => Some(now - Duration::days(180)),
        "1Y" => Some(now - Duration::days(365)),
        "2Y" => Some(now - Duration::days(730)),
        "5Y" => Some(now - Duration::days(1825)),
        "Ytd" => Some(
            Utc::now()
                .with_month(1)?
                .with_day(1)?
                .with_hour(0)?
                .with_minute(0)?
                .with_second(0)?,
        ),
        _ => None,
    }
}

// get period close returns the adjusted close for the stock
pub fn get_period_close(history: &[TickerHistory], target_date: DateTime<Utc>) -> Option<Decimal> {
    for offset in 0..=5 {
        let date = target_date - Duration::days(offset);
        if let Some(entry) = history.iter().find(|h| same_date(h.date, date)) {
            return Some(entry.adj_close);
        }
    }
    None
}

pub fn calculate_performance(current: Decimal, period_close: Decimal) -> Decimal {
    let perf = ((current - period_close) / period_close) * Decimal::from(100);
    perf.round_dp(2)
}

pub fn get_industry_embeddings(tickers: &[Ticker]) -> Vec<(Ticker, Vec<f32>)> {
    tickers
        .iter()
        .filter_map(|t| {
            t.industry_embedding
                .clone()
                .map(|embedding| (t.clone(), embedding))
        })
        .collect()
}

pub fn get_overview_embeddings(tickers: &[Ticker]) -> Vec<(Ticker, Vec<f32>)> {
    tickers
        .iter()
        .filter_map(|t| {
            t.overview_embedding
                .clone()
                .map(|embedding| (t.clone(), embedding))
        })
        .collect()
}
