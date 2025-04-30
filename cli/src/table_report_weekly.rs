use chrono::{DateTime, Datelike, Days, Duration, Local, NaiveDate};
use log::debug;

use jira::models::core::IssueKey;
use std::cmp;
use std::collections::BTreeMap;
use std::fmt::Write;
use worklog::{
    date::{self, seconds_to_hour_and_min},
    types::LocalWorklog,
};

pub fn table_report_weekly(worklog_entries: &[LocalWorklog]) {
    if worklog_entries.is_empty() {
        eprintln!("No worklog entries to create report from!");
        return;
    }
    debug!("table_report() :- {:?}", &worklog_entries);

    let mut daily_totals_by_issue: BTreeMap<&IssueKey, BTreeMap<NaiveDate, i32>> = BTreeMap::new();

    for entry in worklog_entries {
        daily_totals_by_issue
            .entry(&entry.issue_key)
            .or_default()
            .entry(entry.started.date_naive())
            .and_modify(|sum| *sum += entry.timeSpentSeconds)
            .or_insert(entry.timeSpentSeconds);
    }

    if let Some((min_date, max_date)) = find_min_max_started(worklog_entries) {
        let mut current_monday = date::first_date_in_week_for(min_date);
        let last_date = date::last_date_in_week_for(max_date);

        while current_monday <= last_date {
            let current_sunday = current_monday + Days::new(6);
            let week_label = format!(
                "CW {} from {} to {}",
                current_monday.iso_week().week(),
                current_monday.format("%Y-%m-%d"),
                current_sunday.format("%Y-%m-%d")
            );
            println!("{week_label}");

            print_weekly_table_header();
            // Holds the total for each column (day) to be printed at the bottom of each week
            let mut daily_total_per_week = BTreeMap::<NaiveDate, i32>::new();

            for (key, daily_total_per_key) in &daily_totals_by_issue {
                if !has_data_for_week(
                    daily_total_per_key,
                    current_monday.date_naive(),
                    current_sunday.date_naive(),
                ) {
                    continue;
                }

                // Prints a row for current key in current week and returns the daily total
                // for this key in the current week
                print!("{:15}", key.to_string());
                let daily_totals_for_this_key = print_and_accumulate_daily_totals(
                    daily_total_per_key,
                    current_monday.date_naive(), // Start of current week
                    current_sunday.date_naive(), // End of current week
                );

                // Add the daily totals for current key into current week
                for (date, total) in daily_totals_for_this_key {
                    daily_total_per_week
                        .entry(date)
                        .and_modify(|current_total| *current_total += total)
                        .or_insert(total);
                }
            }

            // All keys for this week has been printed, now show the weekly total
            print_single_dashed_line();
            print_week_total(&current_monday, current_sunday, &mut daily_total_per_week);
            current_monday += Duration::weeks(1);
        }
    }
    debug!("Table report done");
}

fn has_data_for_week(
    date_spent_map: &BTreeMap<NaiveDate, i32>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> bool {
    let mut current_date = Some(start_date);
    //while current_date <= end_date {
    while let Some(date) = current_date {
        if date > end_date {
            break;
        }
        if date_spent_map.contains_key(&date) {
            return true;
        }
        current_date = date.succ_opt(); // Increment by one day
    }
    false
}

fn print_and_accumulate_daily_totals(
    daily_total_per_key: &BTreeMap<NaiveDate, i32>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> BTreeMap<NaiveDate, i32> {
    let mut outputs = String::new();
    let mut current_date = Some(start_date);
    let mut time_code_weekly_total = 0;

    let mut daily_total_current_week = BTreeMap::<NaiveDate, i32>::new();
    while let Some(date) = current_date {
        if date > end_date {
            break;
        }

        let spent_seconds = *daily_total_per_key.get(&date).unwrap_or(&0);
        time_code_weekly_total += spent_seconds;

        daily_total_current_week.insert(date, spent_seconds);

        let hh_mm = date::seconds_to_hour_and_min(spent_seconds);
        write!(
            &mut outputs,
            " {:^5}",
            if spent_seconds == 0 {
                "-"
            } else {
                hh_mm.as_str()
            }
        )
        .expect("Failed to write to string buffer");

        current_date = date.succ_opt(); // Safely move to the next day
    }

    println!(
        "{} {:5}",
        outputs,
        seconds_to_hour_and_min(time_code_weekly_total)
    );

    daily_total_current_week
}

fn print_week_total(
    current_monday: &DateTime<Local>,
    sunday: DateTime<Local>,
    total_per_week_day: &mut BTreeMap<NaiveDate, i32>,
) {
    print!("{:15}", "Week total");
    let mut current_date = *current_monday;
    let mut week_total = 0;

    while current_date <= sunday {
        let seconds = total_per_week_day
            .get(&current_date.date_naive())
            .unwrap_or(&0);
        week_total += *seconds;
        let output = if *seconds > 0 {
            seconds_to_hour_and_min(*seconds)
        } else {
            "-".to_string()
        };
        print!(" {output:^5}");
        current_date += Duration::days(1); // Move to next day
    }
    print!(" {:^5}", seconds_to_hour_and_min(week_total));
    println!();

    print_double_dashed_line();

    println!();
}

/// Find the earliest and latest date in the list of [`LocalWorklog`] entries.
fn find_min_max_started(worklogs: &[LocalWorklog]) -> Option<(DateTime<Local>, DateTime<Local>)> {
    if worklogs.is_empty() {
        return None; // No worklogs, no min/max
    }

    let min_max = worklogs.iter().fold(
        (worklogs[0].started, worklogs[0].started), // Initial min/max
        |(min, max), worklog| {
            (
                cmp::min(min, worklog.started),
                cmp::max(max, worklog.started),
            )
        },
    );

    Some(min_max)
}

fn print_weekly_table_header() {
    println!(
        "{:15} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5}",
        "Time code", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Total"
    );

    print_single_dashed_line();
}

fn print_single_dashed_line() {
    println!(
        "{:-<15} ----- ----- ----- ----- ----- ----- ----- -----",
        ""
    );
}
fn print_double_dashed_line() {
    println!(
        "{:=<15} ===== ===== ===== ===== ===== ===== ===== =====",
        ""
    );
}

#[cfg(test)]
mod tests {
    use crate::table_report_weekly::{find_min_max_started, table_report_weekly};
    use chrono::{Days, Local};
    use jira::models::core::IssueKey;
    use std::ops::Sub;
    use worklog::types::LocalWorklog;

    #[test]
    fn test_find_min_max_started() {
        let now = Local::now();
        let worklogs = vec![
            LocalWorklog {
                issue_key: IssueKey::from("ISSUE-1"),
                id: "1".to_string(),
                author: "user1".to_string(),
                created: now,
                updated: now,
                started: now - chrono::Duration::days(2),
                timeSpent: "1h".to_string(),
                timeSpentSeconds: 3600,
                issueId: 101,
                comment: Some("Worklog 1".to_string()),
            },
            LocalWorklog {
                issue_key: IssueKey::from("ISSUE-2"),
                id: "2".to_string(),
                author: "user2".to_string(),
                created: now,
                updated: now,
                started: now - chrono::Duration::days(1),
                timeSpent: "2h".to_string(),
                timeSpentSeconds: 7200,
                issueId: 102,
                comment: Some("Worklog 2".to_string()),
            },
            LocalWorklog {
                issue_key: IssueKey::from("ISSUE-3"),
                id: "3".to_string(),
                author: "user3".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: "30m".to_string(),
                timeSpentSeconds: 1800,
                issueId: 103,
                comment: None,
            },
        ];

        let early = now.sub(Days::new(2));

        if let Some((min_started, max_started)) = find_min_max_started(&worklogs) {
            assert_eq!(early.date_naive(), min_started.date_naive());
            assert_eq!(now.date_naive(), max_started.date_naive());
        } else {
            println!("No worklogs available.");
        }
    }

    #[test]
    fn test_table_report_weekly() {
        table_report_weekly(&[]);
    }
}
