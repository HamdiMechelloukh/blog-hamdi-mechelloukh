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

fn year_from_days(days: i64) -> i64 {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    year_of_era + era * 400 + i64::from(month_index >= 10)
}

pub fn current_year() -> i32 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).expect("horloge avant 1970").as_secs();
    year_from_days((seconds / 86_400) as i64) as i32
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
    fn year_round_trips() {
        assert_eq!(year_from_days(days_from_civil(2026, 12, 31)), 2026);
        assert_eq!(year_from_days(days_from_civil(2027, 1, 1)), 2027);
    }
}
