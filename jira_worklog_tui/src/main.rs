use std::{error::Error, path::PathBuf};

use ratatui::{
    crossterm::event::{
        self, KeyCode, KeyEventKind},
        layout::Constraint,
        style::{Style, Stylize},
        widgets::{Block, Row, Table}, DefaultTerminal
};

use jira_lib::{self, config, JiraClient};
use chrono::{offset::TimeZone, DateTime, Datelike, Local, NaiveDate, NaiveTime, Weekday};

fn week_bounds() -> (u32, DateTime<Local>, DateTime<Local>) {
    let now = Local::now();
    let week = now.iso_week().week() - 1;
    let mon = NaiveDate::from_isoywd_opt(now.year(), week, Weekday::Mon).expect("Failed to get start of week");
    let mon: DateTime<Local> = Local.from_local_datetime(&mon.and_time(NaiveTime::default())).unwrap();
    let sun = NaiveDate::from_isoywd_opt(now.year(), week, Weekday::Sun).expect("Failed to get end of week");
    let sun: DateTime<Local> = Local.from_local_datetime(&sun.and_time(NaiveTime::default())).unwrap();
    (week, mon, sun)
}

#[allow(clippy::unused_async)]
async fn run(mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
    let cfg = config::load()?;
    let client = JiraClient::from(&cfg)?;

    let (week, start_of_week, end_of_week) = week_bounds();

    // Make the TUI show a progress bar for this..
    println!("Sourcing initial data...");
    // Getting all worklogs for a user means querying all time code issues (incredibly slow)
    // let all_time_codes = client.get_issues_for_single_project("TIME".to_string()).await;
    let time_codes = jira_lib::journal::find_unique_keys(&PathBuf::from(&cfg.application_data.journal_data_file_name))?;
    let mut total_issues = 0;
    for issue in &time_codes {
        let entries = match client.get_worklogs_for_current_user(issue, Some(start_of_week)).await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to get work log for Issue {} [{e}]", &issue);
                continue;
            },
        };

        total_issues += entries.len();
    }

    loop {
        terminal.draw(|frame| {
            let rows = [
                Row::new(vec!["TIME-9", "1h", "2h", "3h", "1h", "2h", "9h"]),
                Row::new(vec!["TIME-160", "1h", "2h", "3h", "1h", "2h", "9h"])
            ];
            let widths = [
                Constraint::Length(15),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ];
            let table = Table::new(rows, widths)
                // ...and they can be separated by a fixed spacing.
                .column_spacing(1)
                // You can set the style of the entire Table.
                .style(Style::new().blue())
                // It has an optional header, which is simply a Row always visible at the top.
                .header(
                    Row::new(vec!["Time code", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Total"])
                        .style(Style::new().bold())
                        // To add space between the header and the rest of the rows, specify the margin
                        .bottom_margin(1),
                )
                // It has an optional footer, which is simply a Row always visible at the bottom.
                .footer(Row::new(vec!["Total", "2h", "4h", "6h", "2h", "4h", "18h"]))
                // As any other widget, a Table can be wrapped in a Block.
                .block(Block::new().title(
                    format!(
                        "Week {week} [{} - {}] ({} time code(s), {total_issues} issue(s))",
                        start_of_week.date_naive(),
                        end_of_week.date_naive(),
                        &time_codes.len(),)))
                // The selected row and its content can also be styled.
                .highlight_style(Style::new().reversed())
                // ...and potentially show a symbol in front of the selection.
                .highlight_symbol(">>");
            frame.render_widget(table, frame.area());
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal).await;
    ratatui::restore();
    app_result
}
