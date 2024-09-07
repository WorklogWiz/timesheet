use chrono::offset::TimeZone;
use chrono::{Datelike, DateTime, Days, Duration, Local, Month, NaiveDate, NaiveDateTime, NaiveTime, ParseResult, Weekday};
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
    StartAndDurationExceedsNow{start : DateTime<Local>, duration: Duration , end: DateTime<Local>},
}

impl Display for DateTimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DateTimeError::InvalidInput(s) => {
                write!(f, "Invalid input {}", s)
            }
            DateTimeError::StartAndDurationExceedsNow{start, duration, end} => {
                write!(f, "Starting point {} + duration {} gives {}, which is greater than {} (now)", start, duration, end, Local::now())
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
        let (dur, unit) = Self::parse_to_unit_and_duration(s).unwrap();

        let seconds = match unit.as_str() {
            "h" | "H" => (dur * 3600.0) as i32,
            "d" | "D" => (dur * work_hours_per_day * 3600.0) as i32,
            "w" | "W" => {
                (dur * working_days_per_week * work_hours_per_day * 3600.0) as i32
            }
            _ => {
                panic!("Don't know how to handle units of '{}', expected 'h','d', or 'w'", unit)
            }
        };

        Ok(TimeSpent {
            time_spent: s.to_string(),
            time_spent_seconds: seconds,
            unit,
        })
    }

    // Parses strings like 1,5h -> (1.5 'h')
    pub fn parse_to_unit_and_duration(s: &str) -> Result<(f32, String), DateTimeError> {
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
fn test_parse_to_unit_and_duration() {

    assert!(TimeSpent::parse_to_unit_and_duration("1,5h").is_ok());
    assert!(TimeSpent::parse_to_unit_and_duration("Mon:1,5h").is_err());

}

#[test]
fn test_time_spent() {
    assert!(TimeSpent::from_str("1", 7.5, 5.0).is_err());

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
/// Finally, we ensure that the starting point with the addition of `duration_seconds` does
/// not go past the current time.
pub fn calculate_started_time(
    starting_point: Option<DateTime<Local>>,
    duration_seconds: i32,
) -> Result<DateTime<Local>, DateTimeError> {
    let now = Local::now();
    let duration = Duration::seconds(duration_seconds as i64);

    // Subtracts duration from current time to find the start time
    let proposed_starting_point =
        starting_point.map_or(now.checked_sub_signed(duration).unwrap(), |v| v);

    //  start + duration > now is an error!
    let end_time = proposed_starting_point
        .checked_add_signed(duration)
        .unwrap();
    if end_time.gt(&now) {
        Err(DateTimeError::StartAndDurationExceedsNow{start: proposed_starting_point, duration, end: end_time })
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

pub fn parse_worklog_durations(entries: Vec<String>) -> Vec<(Weekday, f32, String)> {
    lazy_static! {
        // Mon:1,5h
        // Capturing regexp
        static ref DURATION_EXPR: Regex = Regex::new(r"^(\w{3}):(\d+(?:[\.,]\d{1,2})?\w)$").unwrap();
    }

    let mut result: Vec<(Weekday, f32, String)> = Vec::new();

    // Iterates the pattern and extracts tuples of Weekday names and duration
    for s in entries.into_iter() {
        match DURATION_EXPR.captures(&s) {
            Some(captured) => {
                // Parses 3 character weekday abbreviation to Weekday enumerator
                let week_day = String::from(&captured[1]).parse::<Weekday>().unwrap();
                // Parses the duration into duration and unit as separate values
                let (duration, unit) = TimeSpent::parse_to_unit_and_duration(&captured[2]).expect("parsing error!");

                result.push((week_day, duration, unit));
            }
            None => panic!("Could not parse {} into weekday, duration and unit", s)
        }
    }
    result
}


pub fn date_of_last_weekday(weekday: Weekday) -> DateTime<Local> {
    last_weekday_from(Local::now(), weekday)
}

/// Given a Weekday, like for instance Friday, find the first Friday in the past given the
/// supplied starting point
/// Will return today's date if you supply today's weekday
pub fn last_weekday_from(starting_date : DateTime<Local>, weekday: Weekday) -> DateTime<Local> {

    let mut current_date : DateTime<Local> = starting_date;
    let one_day = Days::new(1);

    for _n in 0..7 {
        if current_date.weekday() == weekday {
            return current_date;
        }
        // Skips to previous day
        current_date = current_date.checked_sub_days(one_day).unwrap();
    }
    panic!("Internal error in transforming {} into a date from starting date {}", weekday, starting_date.date_naive() )
}

#[test]
fn test_weekday_to_date_backwards() {
    let d = last_weekday_from(Local.with_ymd_and_hms(2023, 5, 31, 0, 0, 0).unwrap(), Weekday::Wed);
    assert_eq!(d, Local.with_ymd_and_hms(2023,5,31, 0, 0, 0).unwrap(),"Should have been same day");

    let d = last_weekday_from(Local.with_ymd_and_hms(2023, 5, 31, 0, 0, 0).unwrap(), Weekday::Tue);
    assert_eq!(d, Local.with_ymd_and_hms(2023,5,30, 0, 0, 0).unwrap(), "Should give day before");

    let d = last_weekday_from(Local.with_ymd_and_hms(2023, 6, 1, 0, 0, 0).unwrap(), Weekday::Fri);
    assert_eq!(d, Local.with_ymd_and_hms(2023,5,26, 0, 0, 0).unwrap(), "Should give day before");

}

#[test]
fn test_parse_durations() {
    assert_eq!(parse_worklog_durations(vec!["Mon:1,5h".to_string()]), vec![(chrono::Weekday::Mon, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Tue:1,5h".to_string()]), vec![(chrono::Weekday::Tue, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Wed:1,5h".to_string()]), vec![(chrono::Weekday::Wed, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Thu:1.5h".to_string()]), vec![(chrono::Weekday::Thu, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Fri:1,5h".to_string()]), vec![(chrono::Weekday::Fri, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Sat:1,5h".to_string()]), vec![(chrono::Weekday::Sat, 1.5f32, "h".to_string())]);
    assert_eq!(parse_worklog_durations(vec!["Sun:1,5h".to_string()]), vec![(chrono::Weekday::Sun, 1.5f32, "h".to_string())]);
}

#[test]
fn test_date_and_timezone_conversion() {

    let utc = chrono::Utc::now();
    println!("{}", utc);

    let converted: DateTime<Local> = DateTime::from(utc);
    println!("{}", converted);

    let c = utc.with_timezone(&Local);
    println!("{} {}", c, c.with_timezone(&Local));
    println!("{}", c.date_naive().format("%Y-%m-%d"));

    let hour = 45000 / 3600;
    let minutes = (45000 % 3600) / 60;
    println!("{}:{}", hour, minutes);

}

// This ought to be part of the Rust runtime :-)
#[allow(dead_code)]
pub fn month_name(n: u32) -> Month {
    match n {
        1 => Month::January,
        2 => Month::February,
        3 => Month::March,
        4 => Month::April,
        5 => Month::May,
        6 => Month::June,
        7 => Month::July,
        8 => Month::August,
        9 => Month::September,
        10 => Month::October,
        11 => Month::November,
        12 => Month::December,
        _ => panic!("Invalid month number {}", n),
    }
}

pub fn is_new_week(current_week: u32, dt: &NaiveDate) -> bool {
    dt.iso_week().week() > current_week
}

pub fn seconds_to_hour_and_min(accum: &i32) -> String {
    let hour = *accum / 3600;
    let min = *accum % 3600 / 60;
    let duration = format!("{:02}:{:02}", hour, min);
    duration
}