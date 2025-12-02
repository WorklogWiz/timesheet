//! Date utility functions for CLI
//!
//! This module contains date parsing and formatting utilities
//! that are specific to the CLI application.

use chrono::offset::TimeZone;
use chrono::{
    DateTime, Datelike, Days, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseResult, Weekday,
};
use clap::builder::TypedValueParser;
use regex::Regex;
use std::ops::{Add, Sub};
use std::sync::LazyLock;

/// Parses a date, a time or a datetime, which has been supplied as:
/// - `08:00` implicitly indicating today's date
/// - `2023-05-26` implicitly indicating 08:00 on that date
/// - `2023-05-26T09:00` exact specification
///
/// # Errors
/// Returns a parse error if the input string doesn't match any expected format
pub fn str_to_date_time(s: &str) -> ParseResult<DateTime<Local>> {
    static DATE_EXPR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());
    static TIME_EXPR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{1,2}:\d{2}$").unwrap());
    static DATE_TIME_EXPR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{1,2}:\d{2}$").unwrap());

    if DATE_EXPR.is_match(s) {
        let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
        let naive_date_time = naive_date.and_hms_opt(8, 0, 0).unwrap();
        Ok(Local.from_local_datetime(&naive_date_time).unwrap())
    } else if TIME_EXPR.is_match(s) {
        let nt = NaiveTime::parse_from_str(s, "%H:%M")?;
        let local_now = Local::now().date_naive().and_time(nt);
        Ok(Local.from_local_datetime(&local_now).unwrap())
    } else if DATE_TIME_EXPR.is_match(s) {
        let dt = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")?;
        Ok(Local.from_local_datetime(&dt).unwrap())
    } else {
        // Return a parse error for invalid format
        Err(NaiveDate::parse_from_str("invalid", "%Y-%m-%d").unwrap_err())
    }
}

/// Converts seconds to a formatted hour:minute string (HH:MM)
///
/// # Examples
/// ```
/// # use cli::date_utils::seconds_to_hour_and_min;
/// assert_eq!(seconds_to_hour_and_min(3661), "01:01");
/// assert_eq!(seconds_to_hour_and_min(7200), "02:00");
/// ```
#[must_use]
pub fn seconds_to_hour_and_min(seconds: i32) -> String {
    let hour = seconds / 3600;
    let min = seconds % 3600 / 60;
    format!("{hour:02}:{min:02}")
}

/// Returns the first date (Monday) of the week for the given date
#[must_use]
pub fn first_date_in_week_for(dt: DateTime<Local>) -> DateTime<Local> {
    let days = dt.weekday().num_days_from_monday();
    dt.sub(Days::new(u64::from(days)))
}

/// Returns the last date (Sunday) of the week for the given date
#[must_use]
pub fn last_date_in_week_for(dt: DateTime<Local>) -> DateTime<Local> {
    // Monday is 0 and Sunday is 6
    let days = 6 - dt.weekday().num_days_from_monday();
    dt.add(Days::new(u64::from(days)))
}

/// Custom value parser for clap that uses `str_to_date_time`
///
/// This allows date/time arguments to be parsed directly by clap,
/// providing immediate validation and helpful error messages.
#[derive(Clone)]
#[allow(dead_code)]
pub struct DateTimeParser;

impl TypedValueParser for DateTimeParser {
    type Value = DateTime<Local>;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "Invalid UTF-8 in date/time argument",
            )
        })?;

        str_to_date_time(s).map_err(|e| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!(
                    "Invalid date/time format: {e}\n\
                     Expected formats:\n\
                       - YYYY-MM-DD (date only, defaults to 08:00)\n\
                       - HH:MM (time only, uses today's date)\n\
                       - YYYY-MM-DDTHH:MM (exact date and time)"
                ),
            )
        })
    }
}

/// Represents a parsed duration entry with optional weekday prefix
///
/// Examples:
/// - `"4h"` -> `DurationEntry { weekday: None, seconds: 14400 }`
/// - `"mon:4h"` -> `DurationEntry { weekday: Some(Weekday::Mon), seconds: 14400 }`
#[derive(Debug, Clone)]
pub struct DurationEntry {
    pub weekday: Option<Weekday>,
    pub seconds: i64,
}

/// Custom value parser for clap that parses duration strings
///
/// Supports formats like:
/// - "4h", "1.5h", "30m", "1h30m", "1d" (basic duration)
/// - "mon:4h", "tue:3h" (weekday-prefixed duration)
#[derive(Clone)]
#[allow(dead_code)]
pub struct DurationParser;

impl TypedValueParser for DurationParser {
    type Value = DurationEntry;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "Invalid UTF-8 in duration argument",
            )
        })?;

        parse_duration_entry(s).map_err(|e| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!(
                    "Invalid duration format: {e}\n\
                     Supported formats:\n\
                       - 4h, 1.5h, 7,5h (hours)\n\
                       - 30m, 1h30m (hours and minutes)\n\
                       - 1d (days, 1d = 8h)\n\
                       - mon:4h, tue:3h (weekday prefix)"
                ),
            )
        })
    }
}

/// Parse a duration entry string into weekday and seconds
///
/// # Examples
/// ```
/// # use cli::date_utils::parse_duration_entry;
/// let entry = parse_duration_entry("4h").unwrap();
/// assert_eq!(entry.seconds, 14400);
/// assert_eq!(entry.weekday, None);
///
/// let entry = parse_duration_entry("mon:4h").unwrap();
/// assert_eq!(entry.seconds, 14400);
/// ```
pub fn parse_duration_entry(s: &str) -> Result<DurationEntry, String> {
    // Check if duration has weekday prefix (e.g., "mon:4h", "tue:3h")
    let (weekday, duration_part) = if let Some((day, dur)) = s.split_once(':') {
        (Some(parse_weekday(day)?), dur)
    } else {
        (None, s)
    };

    let seconds = parse_duration_to_seconds(duration_part)?;

    Ok(DurationEntry { weekday, seconds })
}

/// Parse a weekday string (like "mon", "monday", "tue")
fn parse_weekday(day: &str) -> Result<Weekday, String> {
    match day.to_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        _ => Err(format!("Invalid weekday: {day}")),
    }
}

/// Parse a duration string (like "4h", "1.5h", "1h30m", "1d") into seconds
fn parse_duration_to_seconds(duration: &str) -> Result<i64, String> {
    let duration = duration.trim().to_lowercase();

    let mut total_seconds = 0i64;
    let mut current_number = String::new();

    for ch in duration.chars() {
        if ch.is_numeric() || ch == '.' || ch == ',' {
            current_number.push(if ch == ',' { '.' } else { ch });
        } else if ch == 'h' {
            let hours: f64 = current_number
                .parse()
                .map_err(|_| format!("Invalid hours: {current_number}"))?;
            #[allow(clippy::cast_possible_truncation)]
            {
                total_seconds += (hours * 3600.0) as i64;
            }
            current_number.clear();
        } else if ch == 'm' {
            let minutes: f64 = current_number
                .parse()
                .map_err(|_| format!("Invalid minutes: {current_number}"))?;
            #[allow(clippy::cast_possible_truncation)]
            {
                total_seconds += (minutes * 60.0) as i64;
            }
            current_number.clear();
        } else if ch == 'd' {
            let days: f64 = current_number
                .parse()
                .map_err(|_| format!("Invalid days: {current_number}"))?;
            #[allow(clippy::cast_possible_truncation)]
            {
                total_seconds += (days * 8.0 * 3600.0) as i64; // 1 day = 8 hours
            }
            current_number.clear();
        }
    }

    if total_seconds == 0 {
        return Err(format!("Could not parse duration: {duration}"));
    }

    Ok(total_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_str_to_date_time_date_only() {
        let result = str_to_date_time("2023-05-26").unwrap();
        assert_eq!(result.date_naive().to_string(), "2023-05-26");
        assert_eq!(result.hour(), 8);
        assert_eq!(result.minute(), 0);
    }

    #[test]
    fn test_str_to_date_time_time_only() {
        let result = str_to_date_time("14:30").unwrap();
        assert_eq!(result.date_naive(), Local::now().date_naive());
        assert_eq!(result.hour(), 14);
        assert_eq!(result.minute(), 30);
    }

    #[test]
    fn test_str_to_date_time_full() {
        let result = str_to_date_time("2023-05-26T14:30").unwrap();
        assert_eq!(result.date_naive().to_string(), "2023-05-26");
        assert_eq!(result.hour(), 14);
        assert_eq!(result.minute(), 30);
    }

    #[test]
    fn test_seconds_to_hour_and_min() {
        assert_eq!(seconds_to_hour_and_min(0), "00:00");
        assert_eq!(seconds_to_hour_and_min(60), "00:01");
        assert_eq!(seconds_to_hour_and_min(3600), "01:00");
        assert_eq!(seconds_to_hour_and_min(3661), "01:01");
        assert_eq!(seconds_to_hour_and_min(7200), "02:00");
        assert_eq!(seconds_to_hour_and_min(7320), "02:02");
    }

    #[test]
    fn test_first_date_in_week_for() {
        // Test with a Wednesday (2023-05-24)
        let wednesday = str_to_date_time("2023-05-24T10:00").unwrap();
        let monday = first_date_in_week_for(wednesday);
        assert_eq!(monday.date_naive().to_string(), "2023-05-22");
    }

    #[test]
    fn test_last_date_in_week_for() {
        // Test with a Wednesday (2023-05-24)
        let wednesday = str_to_date_time("2023-05-24T10:00").unwrap();
        let sunday = last_date_in_week_for(wednesday);
        assert_eq!(sunday.date_naive().to_string(), "2023-05-28");
    }

    #[test]
    fn test_parse_duration_hours() {
        let entry = parse_duration_entry("4h").unwrap();
        assert_eq!(entry.seconds, 14400); // 4 * 3600
        assert!(entry.weekday.is_none());
    }

    #[test]
    fn test_parse_duration_hours_decimal() {
        let entry = parse_duration_entry("1.5h").unwrap();
        assert_eq!(entry.seconds, 5400); // 1.5 * 3600
        assert!(entry.weekday.is_none());
    }

    #[test]
    fn test_parse_duration_hours_comma() {
        let entry = parse_duration_entry("7,5h").unwrap();
        assert_eq!(entry.seconds, 27000); // 7.5 * 3600
        assert!(entry.weekday.is_none());
    }

    #[test]
    fn test_parse_duration_hours_minutes() {
        let entry = parse_duration_entry("1h30m").unwrap();
        assert_eq!(entry.seconds, 5400); // 3600 + 1800
        assert!(entry.weekday.is_none());
    }

    #[test]
    fn test_parse_duration_days() {
        let entry = parse_duration_entry("1d").unwrap();
        assert_eq!(entry.seconds, 28800); // 8 * 3600
        assert!(entry.weekday.is_none());
    }

    #[test]
    fn test_parse_duration_with_weekday() {
        let entry = parse_duration_entry("mon:4h").unwrap();
        assert_eq!(entry.seconds, 14400);
        assert_eq!(entry.weekday, Some(Weekday::Mon));
    }

    #[test]
    fn test_parse_duration_with_full_weekday() {
        let entry = parse_duration_entry("friday:3.5h").unwrap();
        assert_eq!(entry.seconds, 12600); // 3.5 * 3600
        assert_eq!(entry.weekday, Some(Weekday::Fri));
    }

    #[test]
    fn test_parse_weekday_short() {
        assert_eq!(parse_weekday("mon").unwrap(), Weekday::Mon);
        assert_eq!(parse_weekday("tue").unwrap(), Weekday::Tue);
        assert_eq!(parse_weekday("wed").unwrap(), Weekday::Wed);
        assert_eq!(parse_weekday("thu").unwrap(), Weekday::Thu);
        assert_eq!(parse_weekday("fri").unwrap(), Weekday::Fri);
        assert_eq!(parse_weekday("sat").unwrap(), Weekday::Sat);
        assert_eq!(parse_weekday("sun").unwrap(), Weekday::Sun);
    }

    #[test]
    fn test_parse_weekday_full() {
        assert_eq!(parse_weekday("Monday").unwrap(), Weekday::Mon);
        assert_eq!(parse_weekday("FRIDAY").unwrap(), Weekday::Fri);
    }

    #[test]
    fn test_parse_weekday_invalid() {
        assert!(parse_weekday("invalid").is_err());
        assert!(parse_weekday("").is_err());
    }
}
