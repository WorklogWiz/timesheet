use chrono::{DateTime, Months, NaiveDateTime, NaiveTime};

fn main() {
    let arg = "2022-01-15".to_string();
    let s = arg + "T00:00";

    let dt = match NaiveDateTime::parse_from_str(s.as_str(), "%Y-%m-%dT%H:%M") {
        Ok(d) => d,
        Err(err) => {
            panic!("Unable to parse {err:?}")
        }
    };
    let dt2 = match NaiveDateTime::parse_from_str("2022-01-15T00:00", "%Y-%m-%dT%H:%M") {
        Ok(d) => d,
        Err(err) => {
            panic!("Unable to parse: {err:?}")
        }
    };

    assert_eq!(dt.and_utc().timestamp(), dt2.and_utc().timestamp());

    let today = chrono::offset::Local::now();
    println!("Now: {today}");
    // Jira formatting
    println!("Now: {}", today.format("%Y-%m-%dT%H:%M:%S.%3f%z"));
    let r = today.checked_sub_months(Months::new(1)).unwrap();
    println!("Now less a month: {}", r.to_rfc3339());
    let midnight_last_month =
        NaiveDateTime::new(r.date_naive(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());

    println!(
        "Midnight one month ago {}, in milliseconds {}",
        midnight_last_month,
        midnight_last_month.and_utc().timestamp_millis()
    );
    let n =
        DateTime::from_timestamp_millis(midnight_last_month.and_utc().timestamp_millis()).unwrap();
    println!("NaiveDateTime from time in milliseconds {n}");
    // 2023-04-22T15:11:00.000+0700 -> 2023-04-22T10:11:00.000+0200
    // 2023-04-22T15:11:00.000-0700 -> 2023-04-23T00:11:00.000+0200
}

#[test]
pub fn parse_time_zone() {
    use chrono::TimeZone;
    use chrono_tz::Tz;
    use chrono_tz::UTC;
    let tz: Tz = "US/Mountain".parse().unwrap();
    let dt = tz.with_ymd_and_hms(2023, 4, 22, 12, 0, 0).unwrap();
    let utc = dt.with_timezone(&UTC);
    assert_eq!(utc.to_string(), "2023-04-22 18:00:00 UTC");
}
