//! Calcul de dates sans dépendance : conversions jours <-> calendrier civil (algorithmes de Howard Hinnant).

use std::time::{SystemTime, UNIX_EPOCH};

/// Nombre de jours depuis le 1970-01-01.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// (année, mois, jour) du jour numéro `days` depuis le 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

fn now() -> std::time::Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("horloge avant 1970")
}

pub fn current_year() -> i32 {
    civil_from_days((now().as_secs() / 86_400) as i64).0 as i32
}

/// Instant présent au format de `Date.toISOString()` : "2026-04-16T10:35:04.969Z".
pub fn now_iso8601() -> String {
    format_iso8601(now())
}

pub fn format_iso8601(since_epoch: std::time::Duration) -> String {
    let seconds = since_epoch.as_secs();
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let time_of_day = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        time_of_day / 3600,
        time_of_day / 60 % 60,
        time_of_day % 60,
        since_epoch.subsec_millis()
    )
}

/// "2026-06-17" -> "Wed, 17 Jun 2026 00:00:00 +0000" (format exigé par RSS 2.0).
pub fn rfc822(iso_date: &str) -> String {
    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let parts: Vec<i64> = iso_date.split('-').filter_map(|part| part.parse().ok()).collect();
    let [year, month, day] = parts[..] else {
        return iso_date.to_string();
    };
    let weekday = WEEKDAYS[days_from_civil(year, month, day).rem_euclid(7) as usize];
    format!("{weekday}, {day:02} {} {year} 00:00:00 +0000", MONTHS[(month - 1) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc822_has_the_right_weekday() {
        assert_eq!(rfc822("2026-06-17"), "Wed, 17 Jun 2026 00:00:00 +0000");
        assert_eq!(rfc822("2024-02-29"), "Thu, 29 Feb 2024 00:00:00 +0000");
    }

    #[test]
    fn civil_date_round_trips() {
        assert_eq!(civil_from_days(days_from_civil(2026, 12, 31)), (2026, 12, 31));
        assert_eq!(civil_from_days(days_from_civil(2027, 1, 1)), (2027, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
    }

    #[test]
    fn iso8601_matches_to_iso_string() {
        // 2026-04-16T10:35:04.969Z, valeur réelle du state du crossposter.
        let instant = std::time::Duration::from_millis(1_776_335_704_969);
        assert_eq!(format_iso8601(instant), "2026-04-16T10:35:04.969Z");
    }
}
