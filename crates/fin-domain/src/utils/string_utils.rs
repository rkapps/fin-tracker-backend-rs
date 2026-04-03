use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;

pub fn string_to_float(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or_default()
}

pub fn string_to_decimal(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or(Decimal::ZERO)
}

pub fn string_to_int64(s: String) -> i64 {
    s.parse::<i64>().unwrap_or_default()
}

pub fn string_to_int32(s: String) -> i32 {
    s.parse::<i32>().unwrap_or_default()
}

pub fn string_to_naivedate(s: &str) -> Option<NaiveDate> {
    // AlphaVantage standard format is YYYY-MM-DD
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn string_to_utc_datetime(date_str: &str) -> Option<DateTime<Utc>> {
    if date_str.is_empty() || date_str == "None" {
        return None;
    }

    // Parse "YYYY-MM-DD"
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)? // Standardize to midnight
        .and_local_timezone(Utc)
        .single()
}

pub fn alpha_string_to_utc_datetime(date_str: &str) -> DateTime<Utc> {
    let format = "%Y%m%dT%H%M%S";
    NaiveDateTime::parse_from_str(date_str, format)
        .expect("Failed to parse AlphaVantage date")
        .and_utc()
}
