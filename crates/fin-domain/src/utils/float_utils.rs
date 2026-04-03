pub fn float_round_to_6_decimals(value: f64) -> f64 {
    (value * 1000000.0).round() / 1000000.0
}
