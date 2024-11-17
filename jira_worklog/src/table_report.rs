use chrono::{Datelike, NaiveDate};
use common::date;
use jira_lib::{JiraIssue, JiraKey, Worklog};
use log::debug;
use std::collections::{BTreeMap, HashMap};
use local_worklog::LocalWorklog;
//
// Prints a report with tables like this:
//
// Date       Day  time-147 time-117 time-40    Total
// -------------- -------- -------- -------- --------
// 2024-09-02 Mon    07:30        -        -    07:30
// 2024-09-03 Tue        -    07:30        -    07:30
// 2024-09-04 Wed    07:30        -        -    07:30
// 2024-09-05 Thu    07:30        -        -    07:30
// 2024-09-06 Fri    10:30        -        -    10:30
// 2024-09-07 Sat    03:00        -        -    03:00
// -------------- -------- -------- -------- --------
// ISO Week 36       36:00    07:30    00:00    43:30
// ============== ======== ======== ======== ========
//

pub fn table_report(
    worklog_entries: &mut [LocalWorklog],
    issue_keys_by_command_line_order: &Vec<JiraKey>,
) {
    // Holds the accumulated work hours per date and then per issue key
    let mut daily_totals_for_all_jira_key: BTreeMap<NaiveDate, BTreeMap<JiraKey, i32>> =
        BTreeMap::new();

    // Iterates all work logs and accumulates them by date, Jira issue key
    for e in worklog_entries.iter() {
        let date_entry = daily_totals_for_all_jira_key
            .entry(e.started.date_naive())
            .or_default(); // Inserts new  BTreeMap, which is default
        let _daily_total_for_jira_key = date_entry
            .entry(e.issue_key.to_owned())
            .and_modify(|v| *v += e.timeSpentSeconds)
            .or_insert(e.timeSpentSeconds);
    }

    print_weekly_table_header(issue_keys_by_command_line_order);

    let mut weekly_totals_per_jira_key: BTreeMap<JiraKey, i32> = BTreeMap::new();
    let mut current_week = 0;
    // year, {month, total}
    let mut monthly_total: BTreeMap<i32, BTreeMap<u32, i32>> = BTreeMap::new();

    for (date, daily_total_per_jira_key) in daily_totals_for_all_jira_key {
        let daily_total: i32 = daily_total_per_jira_key.values().sum();
        monthly_total
            .entry(date.year())
            .or_default()
            .entry(date.month())
            .and_modify(|current_month_total| *current_month_total += daily_total)
            .or_insert(daily_total);

        if current_week == 0 {
            current_week = date.iso_week().week();
        }

        // If this date is in the next week, summarize for current week
        if date::is_new_week(current_week, &date) {
            print_weekly_total_per_issue(
                issue_keys_by_command_line_order,
                &mut weekly_totals_per_jira_key,
                &mut current_week,
            );

            let current_week_totals: i32 = weekly_totals_per_jira_key.values().sum();
            debug!("Total for CW {} is {}", current_week, current_week_totals);

            current_week = date.iso_week().week(); // Skips to week of current date
            weekly_totals_per_jira_key.clear(); // Remove all entries, prep for next week
            print_weekly_table_header(issue_keys_by_command_line_order); // Table header for next week
        }

        print_daily_entry(
            date,
            issue_keys_by_command_line_order,
            &mut weekly_totals_per_jira_key,
            &daily_total_per_jira_key,
        );
    }

    // In case the last week is incomplete, we also need to print those entries
    if !weekly_totals_per_jira_key.is_empty() {
        print_weekly_total_per_issue(
            issue_keys_by_command_line_order,
            &mut weekly_totals_per_jira_key,
            &mut current_week,
        );
    }

    // Print totals for each (year) and month
    for (_year, monthly_total) in monthly_total {
        for (month_no, month_total) in monthly_total {
            println!(
                "{:<9}: {:>8}",
                date::month_name(month_no).name(),
                date::seconds_to_hour_and_min(&month_total)
            );
        }
    }
}

fn print_daily_entry(
    date: NaiveDate,
    issue_keys_by_command_line_order: &[JiraKey],
    weekly_totals_per_jira_key: &mut BTreeMap<JiraKey, i32>,
    daily_total_per_jira_key: &BTreeMap<JiraKey, i32>,
) {
    let default_value = 0;

    print!("{:10} {:3}", date, date.weekday());

    let daily_total: i32 = daily_total_per_jira_key.values().sum();

    // Accumulates the number of seconds per jira_issue_key for the given date
    // Prints the daily totals
    for jira_key in issue_keys_by_command_line_order {
        let seconds = daily_total_per_jira_key
            .get(jira_key)
            .unwrap_or(&default_value);
        let hh_mm_string = date::seconds_to_hour_and_min(seconds);
        print!(
            " {:>8}",
            if *seconds == 0 {
                "-"
            } else {
                hh_mm_string.as_str()
            }
        );

        // Add the daily totals to the weekly totals
        let s = weekly_totals_per_jira_key
            .entry(jira_key.to_owned())
            .and_modify(|v| *v += seconds)
            .or_insert(*seconds);
        debug!("Weekly total for {jira_key} {s:?}");
    }
    print!(" {:>8}", date::seconds_to_hour_and_min(&daily_total));

    println!();
}

fn print_weekly_table_header(issue_keys_by_command_line_order: &Vec<JiraKey>) {
    print!("{:10} {:3} ", "Date", "Day");
    for jira_issue in issue_keys_by_command_line_order {
        print!(" {:8}", jira_issue.value());
    }
    print!("{:>8}", "Total");
    println!();

    print_single_dashed_line(issue_keys_by_command_line_order);
}

fn print_weekly_total_per_issue(
    issue_keys_by_command_line_order: &[JiraKey],
    sum_per_week_per_jira_key: &mut BTreeMap<JiraKey, i32>,
    current_week: &mut u32,
) {
    // Dashed line below the last date

    print_single_dashed_line(issue_keys_by_command_line_order);

    print!("ISO Week {:<2} {:2}", current_week, "");

    let default = 0;
    let mut week_grand_total = 0;
    for issue_key in issue_keys_by_command_line_order {
        let seconds = sum_per_week_per_jira_key.get(issue_key).unwrap_or(&default);
        let hh_mm_string = date::seconds_to_hour_and_min(seconds);
        print!(" {hh_mm_string:>8}");
        week_grand_total += seconds;
    }
    // Rightmost "Total" column for the entire week
    print!(" {:>8}", date::seconds_to_hour_and_min(&week_grand_total));
    println!();

    // prints the ================= below the totals

    print_double_line(issue_keys_by_command_line_order);
    println!();
}

fn print_double_line(issue_keys_by_command_line_order: &[JiraKey]) {
    print!("{:=<14}", "");
    for _i in 0..issue_keys_by_command_line_order.len() {
        print!(" {:=<8}", "");
    }
    print!(" {:=<8}", ""); // The Total column

    println!();
}

fn print_single_dashed_line(issue_keys_by_command_line_order: &[JiraKey]) {
    print!("{:-<14}", "");

    // print the dashes for each jira_key
    for _i in 0..issue_keys_by_command_line_order.len() {
        print!(" {:-<8}", "");
    }
    // prints the dashes for the total column
    print!(" {:-<8}", "");
    println!();
}

#[cfg(test)]
mod tests {}
