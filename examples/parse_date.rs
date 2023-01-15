use std::error::Error;
use chrono;
use chrono::{Datelike, DateTime, Local, Months, NaiveDate, NaiveDateTime, NaiveTime, ParseResult, Utc};

fn main() {
    let arg = "2022-01-15".to_string();
    let s= arg + "T00:00";

    let dt = match  NaiveDateTime::parse_from_str(s.as_str(),"%Y-%m-%dT%H:%M") {
        Ok(d) => d,
        Err(err) => { panic!("Unable to parse {:?}", err)}
    };
    let dt2 = match NaiveDateTime::parse_from_str("2022-01-15T00:00", "%Y-%m-%dT%H:%M") {
        Ok(d) => d,
        Err(err) => { panic!("Unable to parse: {:?}", err)}
    };

    assert_eq!(dt.timestamp(), dt2.timestamp());

    let today = chrono::offset::Local::now();
    let r = today.checked_sub_months(Months::new(1)).unwrap();
    println!("Now less a month: {}", r.to_rfc3339());
    let midnight_last_month = NaiveDateTime::new(r.date_naive(), NaiveTime::from_hms_opt(0,0,0).unwrap());

    println!("Midnight one month ago {}, in milliseconds {}", midnight_last_month, midnight_last_month.timestamp_millis());
    let n = NaiveDateTime::from_timestamp_millis(midnight_last_month.timestamp_millis()).unwrap();
    println!("NaiveDateTime from time in milliseconds {}", n.to_string());
}