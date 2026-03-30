pub fn current_date_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days_since_epoch = secs / 86400;
    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_lengths = month_lengths_for_year(year);
    let mut month = 1u32;
    for &length in &month_lengths {
        if remaining_days < length {
            break;
        }
        remaining_days -= length;
        month += 1;
    }

    let day = remaining_days + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn month_lengths_for_year(year: u32) -> [u32; 12] {
    let feb = if is_leap_year(year) { 29 } else { 28 };
    [31, feb, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
}

pub fn unix_timestamp_to_date(timestamp: i64) -> String {
    let secs = timestamp.max(0) as u64;
    let days_since_epoch = secs / 86400;
    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        year += 1;
    }
    let month_lengths = month_lengths_for_year(year);
    let mut month = 1u32;
    for &length in &month_lengths {
        if remaining_days < length { break; }
        remaining_days -= length;
        month += 1;
    }
    let day = remaining_days + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

pub fn unix_timestamp_to_hhmm(timestamp: i64) -> String {
    let secs_in_day = timestamp.rem_euclid(86400);
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_timestamp_midnight_gives_0000() {
        assert_eq!(unix_timestamp_to_hhmm(0), "00:00");
    }

    #[test]
    fn unix_timestamp_1430_gives_correct_hhmm() {
        let ts = 14 * 3600 + 30 * 60 + 45;
        assert_eq!(unix_timestamp_to_hhmm(ts), "14:30");
    }

    #[test]
    fn current_date_utc_returns_valid_format() {
        let date = current_date_utc();
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().nth(4), Some('-'));
        assert_eq!(date.chars().nth(7), Some('-'));
    }
}
