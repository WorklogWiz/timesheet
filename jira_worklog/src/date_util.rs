use chrono::offset::TimeZone;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseResult};
use lazy_static::lazy_static;
use std::error::Error;
use std::fmt::{Display, Formatter};

use regex::Regex;

/// Parses a date, a time or a datetime, which has been supplied
/// as:
/// `08:00` implicitly indicating today's date
/// `2023-05-26` implicitly indicating 08:00 on that date
/// `2023-05-26T09:00` exact specification
///
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
        panic!("Unable to parse {} into a DateTime<Local>", s);
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

#[derive(Debug)]
pub enum DateTimeError {
    InvalidInput(String),
    StartAndDurationExceedsNow,
}

impl Display for DateTimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DateTimeError::InvalidInput(s) => {
                write!(f, "Invalid input {}", s)
            }
            DateTimeError::StartAndDurationExceedsNow => {
                write!(f, "Starting point + duration > now")
            }
        }
    }
}

impl Error for DateTimeError {}

#[derive(Debug, PartialEq)]
pub struct TimeSpent {
    pub time_spent: String,
    pub time_spent_seconds: i32,
    pub unit: String,
}

impl TimeSpent {
    pub fn from_str(
        s: &str,
        work_hours_per_day: f32,
        working_days_per_week: f32,
    ) -> Result<TimeSpent, chrono::ParseError> {
        let (dur, unit) = Self::as_duration(s).unwrap();

        let seconds = match unit.as_str() {
            "h" | "H" => (dur  * 3600.0) as i32,
            "d" | "D" => (dur  * work_hours_per_day * 3600.0) as i32,
            "w" | "W" => {
                (dur  * working_days_per_week * work_hours_per_day * 3600.0) as i32
            }
            _ => {
                panic!("Don't know how to handle units of '{}'", unit)
            }
        };

        Ok(TimeSpent {
            time_spent: s.to_string(),
            time_spent_seconds: seconds,
            unit,
        })
    }

    pub fn as_duration(s: &str) -> Result<(f32, String), DateTimeError> {
        lazy_static! {
            static ref TIME_EXPR: Regex = Regex::new(r"^(\d+(?:[\.,]\d{1,2})?)(\w)$").unwrap();
        }
        match TIME_EXPR.captures(s) {
            Some(caps) => {
                let duration: &str = &caps[1];
                let duration = duration.replace(',', ".");
                match duration.parse::<f32>() {
                    Ok(d) => {
                        let unit = String::from(&caps[2]);
                        Ok((d, unit))
                    }
                    Err(_) => Err(DateTimeError::InvalidInput(format!(
                        "Could not parse '{}'",
                        duration
                    ))),
                }
            }
            None => Err(DateTimeError::InvalidInput(format!(
                "Could not obtain duration and unit from '{}'",
                s
            ))),
        }
    }
}

#[test]
fn test_time_spent() {
    assert_eq!(
        TimeSpent {
            time_spent: "1.5h".to_string(),
            time_spent_seconds: 5400,
            unit: "h".to_string(),
        },
        TimeSpent::from_str("1.5h", 7.5, 5.0).unwrap()
    );

    assert_eq!(
        TimeSpent {
            time_spent: "1.2d".to_string(),
            time_spent_seconds: 32400,
            unit: "d".to_string(),
        },
        TimeSpent::from_str("1.2d", 7.5, 5.0).unwrap()
    );

    assert_eq!(
        TimeSpent {
            time_spent: "1.2w".to_string(),
            time_spent_seconds: 162000,
            unit: "w".to_string(),
        },
        TimeSpent::from_str("1.2w", 7.5, 5.0).unwrap()
    );
}

#[allow(dead_code)]
pub fn to_jira_timestamp(datetime: &DateTime<Local>) -> String {
    datetime.format("%Y-%m-%dT%H:%M:%S.000%z").to_string()
}

#[test]
fn test_to_jira_timestamp() {
    to_jira_timestamp(&str_to_date_time("2023-05-25").unwrap());
}

/// Calculates and verifies the starting point. If no starting point is given,
/// `duration_seconds` is subtracted from the current time, else if a starting
/// point was supplied, we use that as-is.
/// Finally we ensure that the starting point with the addition of `duration_seconds` does
/// not go past the current time.
pub fn calculate_started_time(
    starting_point: Option<DateTime<Local>>,
    duration_seconds: i32,
) -> Result<DateTime<Local>, DateTimeError> {
    let now = Local::now();
    let duration = Duration::seconds(duration_seconds as i64);

    let proposed_starting_point =
        starting_point.map_or(now.checked_sub_signed(duration).unwrap(), |v| v);

    let end_time = proposed_starting_point
        .checked_add_signed(duration)
        .unwrap();
    if end_time.gt(&now) {
        Err(DateTimeError::StartAndDurationExceedsNow)
    } else {
        Ok(proposed_starting_point) // It's ok
    }
}

#[test]
fn test_calculate_starting_point() {
    let t = calculate_started_time(None, 3600);
    assert!(t.is_ok());

    let now = Local::now().format("%H:%M").to_string();
    let t = calculate_started_time(Some(str_to_date_time(&now).unwrap()), 3600);
    assert!(t.is_err());

    // now less 1 hour
    let one_hour_ago = Local::now().checked_sub_signed(Duration::hours(1)).unwrap();
    let one_hour_ago_str = one_hour_ago.format("%H:%M").to_string();
    let t = calculate_started_time(Some(str_to_date_time(&one_hour_ago_str).unwrap()), 3600);
    assert!(t.is_ok());

    // Now less 30min adding 1 hour should fail
    let thirty_min_ago = Local::now()
        .checked_sub_signed(Duration::minutes(30))
        .unwrap();
    let thirty_min_ago_as_str = thirty_min_ago.format("%H:%M").to_string();
    let t = calculate_started_time(
        Some(str_to_date_time(&thirty_min_ago_as_str).unwrap()),
        3600,
    );
    assert!(t.is_err());
}
