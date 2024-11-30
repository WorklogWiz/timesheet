use std::{cmp, collections::BTreeMap};

use chrono::{DateTime, Datelike, Days, Duration, Local, NaiveDate};
use log::debug;

use common::date::{self, seconds_to_hour_and_min};
use jira_lib::models::core::JiraKey;
use local_worklog::LocalWorklog;

/// Prints summary tables like this:
/// ````
/// CW 47 from 2024-11-18 to 2024-11-24
/// Time code         Mon   Tue   Wed   Thu   Fri   Sat   Sun Total
/// --------------- ----- ----- ----- ----- ----- ----- ----- -----
/// TIME-137          -     -     -     -     -     -     -   00:00
/// TIME-147          -   15:00 07:30 07:30   -     -     -   30:00
/// TIME-155        05:00   -     -     -     -     -     -   05:00
/// TIME-166        07:30   -     -     -     -     -     -   07:30
/// --------------- ----- ----- ----- ----- ----- ----- ----- -----
/// Week total      12:30 15:00 07:30 07:30   -     -     -   42:30
/// =============== ===== ===== ===== ===== ===== ===== ===== =====
/// ````
pub fn table_report_weekly(worklog_entries: &[LocalWorklog]) {
    if worklog_entries.is_empty() {
        eprintln!("No worklog entries to create report from!");
        return;
    }
    // Holds the accumulated work hours per date and then per issue key
    let mut daily_totals_for_all_jira_key: BTreeMap<&JiraKey, BTreeMap<NaiveDate, i32>> =
        BTreeMap::new();
    debug!("table_report() :- {:?}", &worklog_entries);

    // Iterates all work logs and accumulates them by date, Jira issue key
    for e in worklog_entries {
        daily_totals_for_all_jira_key
            .entry(&e.issue_key)
            //.or_insert_with(BTreeMap::<NaiveDate, i32>::new)
            .or_default()
            .entry(e.started.date_naive())
            .and_modify(|sum| *sum += e.timeSpentSeconds)
            .or_insert(e.timeSpentSeconds);
    }
    let (min_date, max_date) = find_min_max_started(worklog_entries).unwrap();
    let first_date = date::first_date_in_week_for(min_date);
    let last_date = date::last_date_in_week_for(max_date);

    // Process one week at a time
    let mut current_monday = first_date;
    while current_monday <= last_date {
        // Calculates the date of the sunday in the current week
        let sunday = current_monday + Days::new(6);

        println!(
            "CW {} from {} to {}",
            current_monday.iso_week().week(),
            current_monday.format("%Y-%m-%d"),
            sunday.format("%Y-%m-%d")
        );
        print_weekly_table_header();
        let mut total_per_week_day = BTreeMap::<NaiveDate, i32>::new();

        // Iterates the Jira keys in the current week and reports them
        for (key, date_spent_map) in &daily_totals_for_all_jira_key {
            print!("{:15}", key.to_string()); // The time code in the leftmost column

            let mut current_date = current_monday;
            let mut week_total_per_time_code = 0;
            while current_date <= sunday {
                let spent_seconds = date_spent_map.get(&current_date.date_naive()).unwrap_or(&0);
                week_total_per_time_code += spent_seconds;
                total_per_week_day
                    .entry(current_date.date_naive())
                    .and_modify(|total| *total += spent_seconds)
                    .or_insert(*spent_seconds);

                let hh_mm_string = date::seconds_to_hour_and_min(spent_seconds);
                print!(
                    " {:^5}",
                    if *spent_seconds == 0 {
                        "-"
                    } else {
                        hh_mm_string.as_str()
                    }
                );

                current_date += Duration::days(1);
            }
            println!(" {:5}", seconds_to_hour_and_min(&week_total_per_time_code));
        }
        print_single_dashed_line();

        // Jump back to monday and print the totals for each week day in this week

        print_week_total(&mut current_monday, sunday, &mut total_per_week_day);

        // Move to next week
        current_monday += Duration::weeks(1);
    }
    debug!("Table report done");
}

fn print_week_total(
    current_monday: &mut DateTime<Local>,
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
            seconds_to_hour_and_min(seconds)
        } else {
            "-".to_string()
        };
        print!(" {output:^5}");
        current_date += Duration::days(1); // Move to next day
    }
    print!(" {:^5}", seconds_to_hour_and_min(&week_total));
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
    use jira_lib::models::core::JiraKey;
    use local_worklog::LocalWorklog;
    use std::ops::Sub;

    #[test]
    fn test_find_min_max_started() {
        let now = Local::now();
        let worklogs = vec![
            LocalWorklog {
                issue_key: JiraKey::from("ISSUE-1"),
                id: "1".to_string(),
                author: "user1".to_string(),
                created: now,
                updated: now,
                started: now - chrono::Duration::days(2),
                timeSpent: "1h".to_string(),
                timeSpentSeconds: 3600,
                issueId: "101".to_string(),
                comment: Some("Worklog 1".to_string()),
            },
            LocalWorklog {
                issue_key: JiraKey::from("ISSUE-2"),
                id: "2".to_string(),
                author: "user2".to_string(),
                created: now,
                updated: now,
                started: now - chrono::Duration::days(1),
                timeSpent: "2h".to_string(),
                timeSpentSeconds: 7200,
                issueId: "102".to_string(),
                comment: Some("Worklog 2".to_string()),
            },
            LocalWorklog {
                issue_key: JiraKey::from("ISSUE-3"),
                id: "3".to_string(),
                author: "user3".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: "30m".to_string(),
                timeSpentSeconds: 1800,
                issueId: "103".to_string(),
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
