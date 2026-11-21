//! Local dates for the list and the prompts.

use chrono::{DateTime, Local, NaiveDate, TimeZone};

fn local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

/// `2026-11-20 14:00`, for prompts.
pub fn date_time(ms: i64) -> String {
    local(ms).map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
}

/// `2026-11-20`, for file names.
pub fn date(ms: i64) -> String {
    local(ms).map(|t| t.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// `14:00`.
pub fn clock(ms: i64) -> String {
    local(ms).map(|t| t.format("%H:%M").to_string()).unwrap_or_default()
}

/// `Today`, `Yesterday`, `Thu 13 Nov`, or `13 Nov 2025` in another year.
pub fn day_label(ms: i64, today: NaiveDate) -> String {
    let Some(day) = local(ms).map(|t| t.date_naive()) else { return String::new() };
    match (today - day).num_days() {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        2..7 => day.format("%A").to_string(),
        _ if day.format("%Y").to_string() == today.format("%Y").to_string() => day.format("%a %-d %b").to_string(),
        _ => day.format("%-d %b %Y").to_string(),
    }
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// `Thu 20 Nov, 14:00`.
pub fn long(ms: i64) -> String {
    local(ms).map(|t| t.format("%a %-d %b, %H:%M").to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_read_like_a_person_would_say_them() {
        let now = Local::now();
        let today = now.date_naive();
        let ms = now.timestamp_millis();
        let day = 86_400_000;
        assert_eq!(day_label(ms, today), "Today");
        assert_eq!(day_label(ms - day, today), "Yesterday");
        assert!(!day_label(ms - 3 * day, today).contains(' '), "a weekday name within the week");
        assert!(day_label(ms - 400 * day, today).len() >= 10, "an older year names the year");
    }
}
