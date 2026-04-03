use chrono::{DateTime, Datelike, Utc};

pub fn same_date(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    a.year() == b.year() && a.month() == b.month() && a.day() == b.day()
}
