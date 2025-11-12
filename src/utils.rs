use chrono::{DateTime, Utc};
use crunchyroll_rs::categories::Category;

pub fn is_today(date: &DateTime<Utc>) -> bool {
    let now = Utc::now();
    date.date_naive() == now.date_naive()
}

pub fn is_in_past(date: &DateTime<Utc>) -> bool {
    Utc::now() > *date
}

pub fn format_number(n: u32) -> String {
    let (v, s) = match n {
        n if n >= 1_000_000_000 => (n as f32 / 1_000_000_000.0, "B"),
        n if n >= 1_000_000 => (n as f32 / 1_000_000.0, "M"),
        n if n >= 1_000 => (n as f32 / 1_000.0, "K"),
        _ => return n.to_string(),
    };

    let mut f = format!("{:.1}", v);
    if f.ends_with(".0") {
        f.truncate(f.len() - 2);
    }
    format!("{}{}", f, s)
}

pub fn parse_categories(categories: &[Category]) -> String {
    categories
        .iter()
        .map(|category| {
            format!(
                "#{}",
                category
                    .to_string()
                    .to_lowercase()
                    .replace(" ", "_")
                    .replace("-", "_")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
