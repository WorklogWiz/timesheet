use jira_lib::{JiraIssue, JiraKey, Worklog};
use std::collections::{BTreeMap, HashMap};
use chrono::{Datelike, NaiveDate};
use log::debug;

pub fn table_report(
    worklog_entries: &mut [Worklog],
    issue_keys_by_command_line_order: &Vec<JiraKey>,
    issue_information: &HashMap<String, JiraIssue>,
) {
    // Holds the accumulated work hours per date and then per issue key
    let mut daily_totals_for_all_jira_key: BTreeMap<NaiveDate, BTreeMap<JiraKey, i32>> = BTreeMap::new();

    // print_table_header(issue_keys_by_command_line_order);

    // Iterates all work logs and accumulates them by date, Jira issue key
    for e in worklog_entries.iter() {
        let issue_key = match issue_information.get(&e.issueId) {
            None => panic!(
                "Internal programming error, there is no Jira key for issue {}",
                &e.issueId
            ),
            Some(issue) => &issue.key,
        };
        let date_entry = daily_totals_for_all_jira_key
            .entry(e.started.date_naive())
            .or_insert(BTreeMap::new());
        let _daily_total_for_jira_key = date_entry
            .entry(issue_key.to_owned())
            .and_modify(|v| *v += e.timeSpentSeconds)
            .or_insert(e.timeSpentSeconds);
    }

    print_table_header(issue_keys_by_command_line_order);

    //  { week_no, { jira_key, accumulated sum}}
    let mut weekly_totals_per_jira_key: BTreeMap<JiraKey, i32> = BTreeMap::new();
    let mut current_week = 0;
    for (date, daily_total_per_jira_key) in daily_totals_for_all_jira_key.iter() {
        if current_week == 0 {
            current_week = date.iso_week().week();
        }
        // End of previous week, report Weekly total
        if crate::is_new_week(current_week, date) {
            print_weekly_total_per_issue(
                &issue_keys_by_command_line_order,
                &mut weekly_totals_per_jira_key,
                &mut current_week,
            );

            current_week = date.iso_week().week();
            weekly_totals_per_jira_key.clear(); // Remove all entries, prep for next week
            print_table_header(issue_keys_by_command_line_order);
        }

        print_daily_entry(date,issue_keys_by_command_line_order, &mut weekly_totals_per_jira_key, daily_total_per_jira_key);
    }
    // In case the last week is incomplete, we need to print those too
    if weekly_totals_per_jira_key.is_empty() == false {
        print_weekly_total_per_issue(
            &issue_keys_by_command_line_order,
            &mut weekly_totals_per_jira_key,
            &mut current_week,
        );
    }

}

fn print_daily_entry(date: &NaiveDate,issue_keys_by_command_line_order: &Vec<JiraKey>, weekly_totals_per_jira_key: &mut BTreeMap<JiraKey, i32>,  daily_total_per_jira_key: &BTreeMap<JiraKey, i32>) {
    let default_value = 0;

    print!("{:10} {:3}", date, date.weekday());

    let daily_total: i32 = daily_total_per_jira_key.values().sum();

    // Accumulates the number of seconds per jira_issue_key for the given date
    // Prints the daily totals
    for jira_key in issue_keys_by_command_line_order.iter() {
        let seconds = daily_total_per_jira_key
            .get(jira_key)
            .unwrap_or(&default_value);
        let hh_mm_string = crate::seconds_to_hour_and_min(seconds);
        print!(
            " {:>8}",
            if *seconds == 0 {
                "-"
            } else {
                hh_mm_string.as_str()
            }
        );

        // Add the daily totals to the weekly totals
        let _s = weekly_totals_per_jira_key
            .entry(jira_key.to_owned())
            .and_modify(|v| *v += seconds)
            .or_insert(seconds.clone());
        debug!("Weekly total for {} {:?}", jira_key, _s);
    }
    print!(" {:>8}", crate::seconds_to_hour_and_min(&daily_total));

    println!();
}

fn print_table_header(issue_keys_by_command_line_order: &Vec<JiraKey>) {
    print!("{:10} {:3} ", "Date", "Day");
    for jira_issue in issue_keys_by_command_line_order {
        print!(" {:8}", jira_issue.value());
    }
    print!("{:>8}", "Total");
    println!();

    print_single_dashed_line(issue_keys_by_command_line_order);
}

#[cfg(test)]
mod tests {}

fn print_weekly_total_per_issue(
    issue_keys_by_command_line_order: &Vec<JiraKey>,
    sum_per_week_per_jira_key: &mut BTreeMap<JiraKey, i32>,
    current_week: &mut u32,
) {
    // Dashed line below the last date

    print_single_dashed_line(issue_keys_by_command_line_order);

    print!("ISO Week {:<2} {:2}", current_week, "");

    let default = 0;
    let mut week_grand_total = 0;
    for issue_key in issue_keys_by_command_line_order.iter() {
        let seconds = sum_per_week_per_jira_key.get(issue_key).unwrap_or(&default);
        let hh_mm_string = crate::seconds_to_hour_and_min(&seconds);
        print!(" {:>8}", hh_mm_string);
        week_grand_total += seconds;
    }
    print!(" {:>8}", crate::seconds_to_hour_and_min(&week_grand_total));
    println!();

    // prints the ================= below the totals

    print_double_line(issue_keys_by_command_line_order);
    println!();
}

fn print_double_line(issue_keys_by_command_line_order: &Vec<JiraKey>) {
    print!("{:=<14}", "");
    for _i in 0..issue_keys_by_command_line_order.len() {
        print!(" {:=<8}", "");
    }
    print!(" {:=<8}", "");  // The Total column

    println!();
}

fn print_single_dashed_line(issue_keys_by_command_line_order: &Vec<JiraKey>) {
    print!("{:-<14}", "");

    // print the dashes for each jira_key
    for _i in 0..issue_keys_by_command_line_order.len() {
        print!(" {:-<8}", "");
    }
    // prints the dashes for the total column
    print!(" {:-<8}", "");
    println!();
}

