use crate::ticker::IndicatorSnapshot;

pub struct IndicatorWindow {
    pub curr: IndicatorSnapshot,
    pub prev: IndicatorSnapshot,
}

impl IndicatorWindow {
    pub fn new(curr: IndicatorSnapshot, prev: IndicatorSnapshot) -> Self {
        Self { curr, prev }
    }
}

