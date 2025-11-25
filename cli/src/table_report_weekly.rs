use chrono::{DateTime, Datelike, Days, Duration, Local, NaiveDate};
use log::debug;

use std::cmp;
use std::collections::BTreeMap;
use std::fmt::Write;
use worklog_core::WorklogEntry;

use crate::date_utils::{first_date_in_week_for, last_date_in_week_for, seconds_to_hour_and_min};

pub fn table_report_weekly(worklog_entries: &[WorklogEntry]) {
    if worklog_entries.is_empty() {
        eprintln!("No worklog entries to create report from!");
        return;
    }
    debug!("table_report() :- {:?}", &worklog_entries);

    // Group by issue key (convert to IssueKey for display)
    let mut daily_totals_by_issue: BTreeMap<String, BTreeMap<NaiveDate, i32>> = BTreeMap::new();

    for entry in worklog_entries {
        let issue_key_str = entry.issue_key.as_deref().unwrap_or("UNASSIGNED");
        daily_totals_by_issue
            .entry(issue_key_str.to_string())
            .or_default()
            .entry(entry.started_at.date_naive())
            .and_modify(|sum| *sum += entry.time_spent_seconds.unwrap_or(0))
            .or_insert(entry.time_spent_seconds.unwrap_or(0));
    }

    if let Some((min_date, max_date)) = find_min_max_started(worklog_entries) {
        let mut current_monday = first_date_in_week_for(min_date);
        let last_date = last_date_in_week_for(max_date);

        let mut grand_total = 0;
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

                // Prints a row for the current key in the current week and returns the daily total
                // for this key in the current week
                print!("{key:15}");
                let daily_totals_for_this_key = print_and_accumulate_daily_totals(
                    daily_total_per_key,
                    current_monday.date_naive(), // Start of current week
                    current_sunday.date_naive(), // End of current week
                );

                // Add the daily totals for the current key into the current week
                for (date, total) in daily_totals_for_this_key {
                    daily_total_per_week
                        .entry(date)
                        .and_modify(|current_total| *current_total += total)
                        .or_insert(total);
                }
            }

            // All keys for this week have been printed, now show the weekly total
            print_single_dashed_line();
            let week_total =
                print_week_total(&current_monday, current_sunday, &mut daily_total_per_week);
            grand_total += week_total;
            current_monday += Duration::weeks(1);
        }
        println!(
            "Grand total for period from {} to {}: {} ",
            min_date.format("%Y-%m-%d"),
            max_date.format("%Y-%m-%d"),
            seconds_to_hour_and_min(grand_total)
        );
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

        let hh_mm = seconds_to_hour_and_min(spent_seconds);
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
) -> i32 {
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
        current_date += Duration::days(1); // Move to the next day
    }
    print!(" {:^5}", seconds_to_hour_and_min(week_total));
    println!();

    print_double_dashed_line();

    println!();

    week_total
}

/// Find the earliest and latest date in the list of [`WorklogEntry`] entries.
fn find_min_max_started(worklogs: &[WorklogEntry]) -> Option<(DateTime<Local>, DateTime<Local>)> {
    if worklogs.is_empty() {
        return None; // No worklogs, no min/max
    }

    let min_max = worklogs.iter().fold(
        (worklogs[0].started_at, worklogs[0].started_at), // Initial min/max
        |(min, max), worklog| {
            (
                cmp::min(min, worklog.started_at),
                cmp::max(max, worklog.started_at),
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
    use chrono::Local;
    use std::ops::Sub;
    use worklog_core::WorklogEntry;

    #[test]
    fn test_find_min_max_started() {
        let now = Local::now();
        let worklogs = vec![
            WorklogEntry {
                id: Some("1".to_string()),
                issue_key: Some("ISSUE-1".to_string()),
                started_at: now - chrono::Duration::days(2),
                stopped_at: Some(now - chrono::Duration::days(2) + chrono::Duration::hours(1)),
                comment: Some("Worklog 1".to_string()),
                tags: Vec::new(),
                time_spent_seconds: Some(3600),
                synced_to_provider: false,
                provider_worklog_id: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                last_synced_at: None,
                version: 1,
                has_conflict: false,
            },
            WorklogEntry {
                id: Some("2".to_string()),
                issue_key: Some("ISSUE-2".to_string()),
                started_at: now - chrono::Duration::days(1),
                stopped_at: Some(now - chrono::Duration::days(1) + chrono::Duration::hours(2)),
                comment: Some("Worklog 2".to_string()),
                tags: Vec::new(),
                time_spent_seconds: Some(7200),
                synced_to_provider: false,
                provider_worklog_id: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                last_synced_at: None,
                version: 1,
                has_conflict: false,
            },
            WorklogEntry {
                id: Some("3".to_string()),
                issue_key: Some("ISSUE-3".to_string()),
                started_at: now,
                stopped_at: Some(now + chrono::Duration::minutes(30)),
                comment: None,
                tags: Vec::new(),
                time_spent_seconds: Some(1800),
                synced_to_provider: false,
                provider_worklog_id: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                last_synced_at: None,
                version: 1,
                has_conflict: false,
            },
        ];

        let early = now.sub(chrono::Duration::days(2));

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
