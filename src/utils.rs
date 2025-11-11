use chrono::{DateTime, Utc};

pub fn is_today(date: &DateTime<Utc>) -> bool {
    let now = Utc::now();
    date.date_naive() == now.date_naive()
}

pub fn is_in_past(date: &DateTime<Utc>) -> bool {
    Utc::now() > *date
}
