//! Affichage des dates : JJ/mois/AAAA (ex. 14/juin/2026).

use chrono::{Datelike, Timelike};

pub const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

pub fn month_label_fr(month: u32) -> &'static str {
    MONTHS_FR
        .get((month as usize).saturating_sub(1))
        .copied()
        .unwrap_or("mois")
}

pub fn format_date_parts(day: u32, month: u32, year: i32) -> String {
    format!("{:02}/{}/{}", day, month_label_fr(month), year)
}

pub fn format_naive_date(d: chrono::NaiveDate) -> String {
    format_date_parts(d.day(), d.month(), d.year())
}

pub fn format_naive_datetime(dt: chrono::NaiveDateTime) -> String {
    format!(
        "{} {:02}:{:02}:{:02}",
        format_naive_date(dt.date()),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

pub fn format_local_now_date() -> String {
    let now = chrono::Local::now();
    format_date_parts(now.day(), now.month(), now.year())
}

/// Date compacte compteur (jjmmaaaa, ex. 14062026).
pub fn format_jjmmaaaa(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.len() != 8 || !s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let day: u32 = s[0..2].parse().ok()?;
    let month: u32 = s[2..4].parse().ok()?;
    let year: i32 = s[4..8].parse().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some(format_date_parts(day, month, year))
}

pub fn format_iso_date_str(s: &str) -> String {
    if s == "—" || s.is_empty() {
        return s.to_string();
    }
    if let Some(formatted) = format_jjmmaaaa(s) {
        return formatted;
    }
    if let Some(day) = s.get(0..10) {
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") {
            return format_naive_date(parsed);
        }
    }
    format_iso_datetime_str(s)
}

pub fn format_iso_datetime_str(s: &str) -> String {
    if s == "—" || s.is_empty() {
        return s.to_string();
    }
    let trimmed = s.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return format_naive_datetime(dt.naive_local());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f") {
        return format_naive_datetime(dt);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return format_naive_datetime(dt);
    }
    if let Some(day) = trimmed.get(0..10) {
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") {
            return format_naive_date(parsed);
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jjmmaaaa_to_french_date() {
        assert_eq!(format_jjmmaaaa("14062026"), Some("14/juin/2026".into()));
    }

    #[test]
    fn iso_date_to_french() {
        assert_eq!(format_iso_date_str("2026-06-14"), "14/juin/2026");
    }
}
