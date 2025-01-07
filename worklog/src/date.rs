use anyhow::{bail, Context};
use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Weekday};
use chrono::{Days, Month, NaiveDateTime, NaiveTime, ParseResult};

use lazy_static::lazy_static;
use num_traits::cast::FromPrimitive;
use regex::Regex;
use std::error;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Sub};

/// Parses a date, a time or a datetime, which has been supplied
/// as:
/// `08:00` implicitly indicating today's date
/// `2023-05-26` implicitly indicating 08:00 on that date
/// `2023-05-26T09:00` exact specification
///
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub fn str_to_date_time(s: &str) -> ParseResult<DateTime<Local>> {
    lazy_static! {
        static ref DATE_EXPR: Regex = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
        static ref TIME_EXPR: Regex = Regex::new(r"^\d{1,2}:\d{2}$").unwrap();
        static ref DATE_TIME_EXPR: Regex =
            Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{1,2}:\d{2}$").unwrap();
    }

    if DATE_EXPR.is_match(s) {
        let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
        let naive_date_time = naive_date.and_hms_opt(8, 0, 0).unwrap();
        Ok(Local.from_local_datetime(&naive_date_time).unwrap())
    } else if TIME_EXPR.is_match(s) {
        let nt = NaiveTime::parse_from_str(s, "%H:%M").unwrap();
        let local_now = Local::now().date_naive().and_time(nt);
        Ok(Local.from_local_datetime(&local_now).unwrap())
    } else if DATE_TIME_EXPR.is_match(s) {
        let dt = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap();
        Ok(Local.from_local_datetime(&dt).unwrap())
    } else {
        // TODO: don't panic, return an error
        panic!("Unable to parse {s} into a DateTime<Local>");
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidInput(String),
    StartAndDurationExceedsNow {
        start: DateTime<Local>,
        duration: Duration,
        end: DateTime<Local>,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidInput(s) => {
                write!(f, "Invalid input {s}")
            }
            Error::StartAndDurationExceedsNow {
                start,
                duration,
                end,
            } => {
                write!(
                    f,
                    "Starting point {} + duration {} gives {}, which is greater than {} (now)",
                    start,
                    duration,
                    end,
                    Local::now()
                )
            }
        }
    }
}

impl error::Error for Error {}

///
/// Time Entry Formats
/// Hours and minutes: Enter time as "Xh Ym" where X is the number of hours and Y is the number of minutes. For example:
///  - "4h 30m" for 4 hours and 30 minutes
///  - "2h 15m" for 2 hours and 15 minutes
///
/// Decimal hours: Use a decimal point to indicate partial hours. For example:
///  - "3.5h" for 3 hours and 30 minutes
///  - "1.25h" for 1 hour and 15 minutes
///
/// Minutes only: Simply enter the number of minutes followed by "m". For example:
///  - "90m" for 1 hour and 30 minutes
///  - "45m" for 45 minutes
///
/// Hours only: Enter the number of hours followed by "h". For example:
///  - "6h" for 6 hours
///  - 0.5h for 30 minutes
#[derive(Debug, PartialEq)]
pub struct TimeSpent {
    pub time_spent: String,
    pub time_spent_seconds: i32,
}

impl TimeSpent {
    /// `from_str` attempts to parse a user-provided time duration string into a `TimeSpent` instance.
    ///
    /// The string must follow a specific format:
    ///
    /// - **Weeks** (`w`): E.g., "1w"
    /// - **Days** (`d`): E.g., "2.5d" indicates 2 days and 12 hours
    /// - **Hours** (`h`): E.g., "1.5h" indicates 1 hour and 30 minutes
    /// - **Minutes** (`m`): E.g., "30m"
    ///
    /// These components can be combined in the input string in any order.
    ///
    /// Examples of valid input strings:
    /// - `"1w2.5d5.5h30m"`
    /// - `"1,5d2,5h3m"` (comma as a decimal separator is accepted)
    ///
    /// # Parameters
    ///
    /// - `s`: The duration string to parse.
    /// - `work_hours_per_day`: The number of working hours per day. Used to interpret `d` (days) component.
    /// - `working_days_per_week`: The number of working days per week. Used to interpret `w` (weeks) component.
    ///
    /// # Returns
    ///
    /// - On success, returns a `TimeSpent` instance containing both the original string and the parsed duration in seconds.
    /// - On failure, returns a variant of `Error::InvalidInput` describing the error.
    #[allow(
        clippy::missing_errors_doc,
        clippy::missing_panics_doc,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    pub fn from_str(
        s: &str,
        work_hours_per_day: f32,
        working_days_per_week: f32,
    ) -> Result<TimeSpent, Error> {
        lazy_static! {
            static ref TIME_SPEC: Regex =
                Regex::new(r"\b(?:(\d+(?:[.,]\d{1,2})?)w)?(?:(\d+(?:[.,]\d{1,2})?)d)?(?:(\d+(?:[.,]\d{1,2})?)h)?(?:(\d+)m)?\b"
            ).unwrap();
        }
        // Parsing floating point, requires full stop as the decimal point delimiter
        let s = s.to_lowercase().replace(',', ".");
        let cap = TIME_SPEC.captures(&s);
        match cap {
            // There seems to be a bug with Captures(), even with no match, it returns Some()
            Some(captures) if !captures.get(0).unwrap().as_str().is_empty() => {
                let weeks = captures
                    .get(1)
                    .map_or(0.0, |m| m.as_str().parse::<f32>().unwrap_or(0.0));
                let days = captures
                    .get(2)
                    .map_or(0.0, |m| m.as_str().parse::<f32>().unwrap_or(0.0));
                let hours = captures
                    .get(3)
                    .map_or(0.0, |m| m.as_str().parse::<f32>().unwrap_or(0.0));
                let minutes = captures
                    .get(4)
                    .map_or(0, |m| m.as_str().parse::<u32>().unwrap_or(0));

                println!("Parsed time: {days} days, {hours} hours, {minutes} minutes");
                let seconds: f32 = weeks * working_days_per_week * work_hours_per_day * 3600.0
                    + days * work_hours_per_day * 3600.0
                    + hours * 3600.0
                    + minutes as f32 * 60.0;
                Ok(TimeSpent {
                    time_spent: s.to_lowercase(),
                    time_spent_seconds: seconds as i32,
                })
            }
            _ => Err(Error::InvalidInput(format!(
                "Could not obtain duration and unit from '{s}'"
            ))),
        }
    }
}

/// Calculates and verifies the starting point. If no starting point is given,
/// `duration_seconds` is subtracted from the current time, else if a starting
/// point was supplied, we use that as-is.
/// Finally, we ensure that the starting point with the addition of `duration_seconds` does
/// not go past the current time.
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub fn calculate_started_time(
    starting_point: Option<DateTime<Local>>,
    duration_seconds: i32,
) -> Result<DateTime<Local>, Error> {
    let now = Local::now();
    let duration = Duration::seconds(duration_seconds.into());

    // Subtracts duration from current time to find the start time
    let proposed_starting_point =
        starting_point.map_or(now.checked_sub_signed(duration).unwrap(), |v| v);

    //  start + duration > now is an error!
    let end_time = proposed_starting_point
        .checked_add_signed(duration)
        .unwrap();
    if end_time.gt(&now) {
        Err(Error::StartAndDurationExceedsNow {
            start: proposed_starting_point,
            duration,
            end: end_time,
        })
    } else {
        Ok(proposed_starting_point) // It's ok
    }
}

///
/// # Errors
/// Returns error if the input specification could not be parsed
pub fn parse_hour_and_minutes_to_seconds(time_str: &str) -> anyhow::Result<i32> {
    lazy_static! {
        static ref HH_MM_EXPR: Regex = Regex::new(r"^\d{2}:\d{2}$").unwrap();
    }
    if !HH_MM_EXPR.is_match(time_str) {
        bail!("{} cannot be parsed into hours and minutes", time_str);
    }

    // Split the string by ':' to get hours and minutes as strings
    let parts: Vec<&str> = time_str.split(':').collect();

    if parts.len() == 2 {
        // Parse hours and minutes from the parts
        let hours: i32 = parts[0]
            .parse()
            .with_context(|| format!("Unable to parse hours '{}' to i32", parts[0]))?;
        let minutes: i32 = parts[1]
            .parse()
            .with_context(|| format!("Failed to parse minutes '{}' to i32", parts[1]))?;

        // Convert hours and minutes to seconds
        let total_seconds = (hours * 3600) + (minutes * 60);

        Ok(total_seconds)
    } else {
        bail!("Invalid duration '{}' format. Expected HH:MM", time_str);
    }
}

#[must_use]
pub fn first_date_in_week_for(dt: DateTime<Local>) -> DateTime<Local> {
    let days = dt.weekday().num_days_from_monday();
    dt.sub(Days::new(u64::from(days)))
}

#[must_use]
pub fn last_date_in_week_for(dt: DateTime<Local>) -> DateTime<Local> {
    // Monday is 0 and Sunday is 6
    let days = 6 - dt.weekday().num_days_from_monday();
    dt.add(Days::new(u64::from(days)))
}

/// Splits a vector of day names and durations separated by ':' into
/// a vector of tuples, holding the Weekday and the duration
/// Given for instance \["mon:1,5h"\] the resulting vector will be
/// \[(Monday, "1,5h")\]
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn parse_worklog_durations(entries: Vec<String>) -> Vec<(Weekday, String)> {
    let mut result: Vec<(Weekday, String)> = Vec::new();

    // Iterates the pattern and extracts tuples of Weekday names and duration
    for entry in entries {
        if let Some(split_result) = entry.split_once(':') {
            let day_name = split_result.0;
            let week_day = String::from(day_name).parse::<Weekday>().unwrap();
            let duration = split_result.1.to_string();
            result.push((week_day, duration));
        } else {
            eprintln!("Unable to split string \"{entry}\", missing ':' ?");
        }
    }
    result
}

#[must_use]
pub fn last_weekday(weekday: Weekday) -> DateTime<Local> {
    last_weekday_from(Local::now(), weekday)
}

/// Given a Weekday, like for instance Friday, find the first Friday in the past given the
/// supplied starting point
/// Will return today's date if you supply today's weekday
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn last_weekday_from(starting_date: DateTime<Local>, weekday: Weekday) -> DateTime<Local> {
    let mut current_date: DateTime<Local> = starting_date;
    let one_day = Days::new(1);

    for _n in 0..7 {
        if current_date.weekday() == weekday {
            return current_date;
        }
        // Skips to previous day
        current_date = current_date.checked_sub_days(one_day).unwrap();
    }
    panic!(
        "Internal error in transforming {} into a date from starting date {}",
        weekday,
        starting_date.date_naive()
    )
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn month_name(n: u32) -> Month {
    Month::from_u32(n).unwrap()
}

#[must_use]
pub fn is_new_week(current_week: u32, dt: &NaiveDate) -> bool {
    dt.iso_week().week() > current_week
}

#[must_use]
pub fn seconds_to_hour_and_min(seconds: i32) -> String {
    let hour = seconds / 3600;
    let min = seconds % 3600 / 60;
    let duration = format!("{hour:02}:{min:02}");
    duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hour_and_minutes_to_seconds() {
        let seconds = parse_hour_and_minutes_to_seconds("01:30").unwrap();
        assert_eq!(seconds, 5400);
    }

    #[test]
    fn test_parse_invalid_hour_and_minutes_to_seconds() {
        if let Ok(_seconds) = parse_hour_and_minutes_to_seconds("1:30") {
            panic!("Parsing '1:30' did not fail!");
        }
    }

    #[test]
    fn test_as_date_time() {
        let dt = NaiveDateTime::parse_from_str("2023-05-25T08:00", "%Y-%m-%dT%H:%M").unwrap();
        assert_eq!(
            str_to_date_time("2023-05-25").unwrap(),
            Local.from_local_datetime(&dt).unwrap()
        );

        let expect = Local::now()
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(
            str_to_date_time("08:00").unwrap(),
            Local.from_local_datetime(&expect).unwrap()
        );

        let dt = Local
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2023-05-25T20:59", "%Y-%m-%dT%H:%M").unwrap(),
            )
            .unwrap();
        assert_eq!(str_to_date_time("2023-05-25T20:59").unwrap(), dt);
    }

    #[test]
    fn time_spent() {
        assert!(
            TimeSpent::from_str("1", 7.5, 5.0).is_err(),
            "The regexp should have failed"
        );

        assert_eq!(
            TimeSpent {
                time_spent: "1.5h".to_string(),
                time_spent_seconds: 5400,
            },
            TimeSpent::from_str("1.5h", 7.5, 5.0).unwrap()
        );

        assert_eq!(
            TimeSpent {
                time_spent: "1.2d".to_string(),
                time_spent_seconds: 32400,
            },
            TimeSpent::from_str("1.2d", 7.5, 5.0).unwrap()
        );

        assert_eq!(
            TimeSpent {
                time_spent: "1.2w".to_string(),
                time_spent_seconds: 162_000,
            },
            TimeSpent::from_str("1.2w", 7.5, 5.0).unwrap()
        );
        assert_eq!(
            TimeSpent {
                time_spent: "7h30m".to_string(),
                time_spent_seconds: 27000
            },
            TimeSpent::from_str("7h30m", 7.5, 5.0).unwrap()
        );

        assert_eq!(
            TimeSpent {
                time_spent: "1.5w0.5d7.5h30m".to_string(),
                time_spent_seconds: 244_800
            },
            TimeSpent::from_str("1.5w0.5d7.5h30m", 7.5, 5.0).unwrap()
        );
    }

    #[test]
    fn capture_regex_bug() {
        let r = Regex::new(r"\b\d+\b");
        // If this suddenly starts returning a "Some" value, the bug in Regex has been fixed
        assert!(
            r.unwrap().captures("rubbish").is_none(),
            "Seems they have fixed the bug in regex captures()"
        );
    }

    #[test]
    fn calculate_starting_point() {
        let t = calculate_started_time(None, 3600);
        assert!(t.is_ok(), "{t:?}");

        let current_hh_mm_str = Local::now().format("%H:%M").to_string();
        let t = calculate_started_time(Some(str_to_date_time(&current_hh_mm_str).unwrap()), 3600);
        assert!(t.is_err(), "{t:?}");

        let a_date_time = Local
            .with_ymd_and_hms(2024, 12, 17, 12, 25, 0)
            .single()
            .expect("Could not create local DateTime");
        // now less 1 hour
        let one_hour_ago = a_date_time.checked_sub_signed(Duration::hours(1)).unwrap();
        let one_hour_ago_str = one_hour_ago.format("%H:%M").to_string();
        let t = calculate_started_time(Some(str_to_date_time(&one_hour_ago_str).unwrap()), 3600);
        assert!(t.is_ok(), "{t:?}");

        // Now less 30min, adding 1 hour should fail
        let thirty_min_ago = Local::now()
            .checked_sub_signed(Duration::minutes(30))
            .unwrap();
        let thirty_min_ago_as_str = thirty_min_ago.format("%H:%M").to_string();
        let t = calculate_started_time(
            Some(str_to_date_time(&thirty_min_ago_as_str).unwrap()),
            3600, // Adding 1 hour should send us 30min into the future and fail
        );
        assert!(t.is_err(), "Result was not an error {t:?}");
    }

    #[test]
    fn weekday_to_date_backwards() {
        let d = last_weekday_from(
            Local.with_ymd_and_hms(2023, 5, 31, 0, 0, 0).unwrap(),
            Weekday::Wed,
        );
        assert_eq!(
            d,
            Local.with_ymd_and_hms(2023, 5, 31, 0, 0, 0).unwrap(),
            "Should have been same day"
        );

        let d = last_weekday_from(
            Local.with_ymd_and_hms(2023, 5, 31, 0, 0, 0).unwrap(),
            Weekday::Tue,
        );
        assert_eq!(
            d,
            Local.with_ymd_and_hms(2023, 5, 30, 0, 0, 0).unwrap(),
            "Should give day before"
        );

        let d = last_weekday_from(
            Local.with_ymd_and_hms(2023, 6, 1, 0, 0, 0).unwrap(),
            Weekday::Fri,
        );
        assert_eq!(
            d,
            Local.with_ymd_and_hms(2023, 5, 26, 0, 0, 0).unwrap(),
            "Should give day before"
        );
    }

    #[test]
    fn parse_durations() {
        assert_eq!(
            parse_worklog_durations(vec!["Mon:1,5h".to_string()]),
            vec![(chrono::Weekday::Mon, "1,5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Tue:1,5h".to_string()]),
            vec![(chrono::Weekday::Tue, "1,5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Wed:1,5h".to_string()]),
            vec![(chrono::Weekday::Wed, "1,5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Thu:1.5h".to_string()]),
            vec![(chrono::Weekday::Thu, "1.5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Fri:1,5h".to_string()]),
            vec![(chrono::Weekday::Fri, "1,5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Sat:1,5h".to_string()]),
            vec![(chrono::Weekday::Sat, "1,5h".to_string())]
        );
        assert_eq!(
            parse_worklog_durations(vec!["Sun:1,5h".to_string()]),
            vec![(chrono::Weekday::Sun, "1,5h".to_string())]
        );
    }

    #[test]
    fn decimal_duration() {
        match TimeSpent::from_str("1,2h", 7.5, 5.0) {
            Ok(result) => {
                assert_eq!(
                    result.time_spent_seconds, 4320,
                    "Invalid calculation of time spent"
                );
                println!("{} {}", result.time_spent_seconds, result.time_spent);
            }
            Err(e) => {
                panic!("{e}")
            }
        }
    }
    #[test]
    fn date_and_timezone_conversion() {
        let utc = chrono::Utc::now();
        println!("{utc}");

        let converted: DateTime<Local> = DateTime::from(utc);
        println!("{converted}");

        let c = utc.with_timezone(&Local);
        println!("{} {}", c, c.with_timezone(&Local));
        println!("{}", c.date_naive().format("%Y-%m-%d"));

        let hour = 45000 / 3600;
        let minutes = (45000 % 3600) / 60;
        println!("{hour}:{minutes}");
    }

    #[test]
    fn test_first_date_in_week_for() {
        let now = Local.with_ymd_and_hms(2024, 11, 22, 21, 36, 0);
        let first_date_in_week = first_date_in_week_for(now.unwrap());
        assert_eq!(
            first_date_in_week.date_naive(),
            NaiveDate::from_ymd_opt(2024, 11, 18).unwrap()
        );
    }

    #[test]
    fn test_last_date_in_week_for() {
        let now = Local.with_ymd_and_hms(2024, 11, 22, 21, 36, 0);
        let last_date_in_week = last_date_in_week_for(now.unwrap());
        assert_eq!(
            last_date_in_week.date_naive(),
            NaiveDate::from_ymd_opt(2024, 11, 24).unwrap()
        );
    }
}
