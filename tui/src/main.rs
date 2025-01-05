use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    layout::Constraint,
    style::{Style, Stylize},
    widgets::{Block, Borders, Row, Table},
    DefaultTerminal,
};
use std::error::Error;
use worklog::{
    config,
    storage::{LocalWorklog, WorklogStorage},
};

use chrono::{
    offset::TimeZone, DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Weekday,
};

fn week_bounds(date: DateTime<Local>) -> (u32, DateTime<Local>, DateTime<Local>) {
    //let now = Local::now();
    let week = date.iso_week().week();
    let mon = NaiveDate::from_isoywd_opt(date.year(), week, Weekday::Mon)
        .expect("Failed to get start of week");
    let mon: DateTime<Local> = Local
        .from_local_datetime(&mon.and_time(NaiveTime::default()))
        .unwrap();
    let sun = NaiveDate::from_isoywd_opt(date.year(), week, Weekday::Sun)
        .expect("Failed to get end of week");
    let sun: DateTime<Local> = Local
        .from_local_datetime(&sun.and_time(NaiveTime::default()))
        .unwrap();
    (week, mon, sun)
}

#[allow(clippy::type_complexity)]
#[allow(clippy::cast_sign_loss)]
fn map_to_week_view(worklogs: &[LocalWorklog]) -> (Vec<(String, [u32; 7], u32)>, [u32; 7], u32) {
    let mut week_view: Vec<(String, [u32; 7], u32)> = vec![];
    let mut column_sums = [0u32; 7];
    let mut total_sum = 0u32;

    for worklog in worklogs.iter().take(7) {
        let day = worklog.started.weekday().num_days_from_monday();
        let mut found = false;
        for (code, times, row_sum) in &mut week_view {
            if code == &worklog.issueId {
                times[day as usize] += worklog.timeSpentSeconds as u32;
                *row_sum += worklog.timeSpentSeconds as u32;
                found = true;
                break;
            }
        }

        if !found {
            let mut times = [0u32; 7];
            times[day as usize] = worklog.timeSpentSeconds as u32;
            week_view.push((
                worklog.issueId.clone(),
                times,
                worklog.timeSpentSeconds as u32,
            ));
        }

        column_sums[day as usize] += worklog.timeSpentSeconds as u32;
        total_sum += worklog.timeSpentSeconds as u32;
    }

    (week_view, column_sums, total_sum)
}

#[allow(clippy::type_complexity)]
fn fetch_weekly_data(
    worklog_service: &WorklogStorage,
    start_of_week: DateTime<Local>,
) -> (Vec<(String, [u32; 7], u32)>, [u32; 7], u32) {
    /*
    let all_entries: Vec<Vec<Worklog>> =
        futures::future::join_all(time_codes.into_iter().map(|issue| {
            let client = &worklog_service;
            async move {
                match client
                    .find_worklogs_after(&issue, Some(start_of_week))
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("Failed to get work log for Issue {} [{e}]", &issue);
                        vec![]
                    }
                }
            }
        }))
        .await;
     */
    let mut all_local = match worklog_service.find_worklogs_after(start_of_week, &[], &[]) {
        Ok(worklogs) => worklogs,
        Err(e) => {
            panic!("Unable to retrieve worklogs from local work log database {e}");
        }
    };

    all_local.sort_by_key(|e| e.started);
    map_to_week_view(&all_local)
}

#[allow(clippy::unused_async)]
async fn run(mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
    let worklog_service = WorklogStorage::new(&config::worklog_file())?;
    let mut current_date = Local::now();

    loop {
        let (week, start_of_week, end_of_week) = week_bounds(current_date);
        let (week_data, column_sums, row_sums) = fetch_weekly_data(&worklog_service, start_of_week);

        let rows: Vec<Row> = week_data
            .iter()
            .map(|(code, times, row_sum)| {
                let mut cells = vec![code.clone()];
                cells.extend(
                    times
                        .iter()
                        .map(|&time_spent| format!("{} hours", time_spent / 3600)),
                );
                cells.push(format!("{} hours", row_sum / 3600));
                Row::new(cells)
            })
            .collect();

        let mut footer_cells = vec!["Total".to_string()];
        footer_cells.extend(
            column_sums
                .iter()
                .map(|&sum| format!("{} hours", sum / 3600)),
        );
        footer_cells.push(format!("{} hours", row_sums / 3600));

        terminal.draw(|frame| {
            let widths = [
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
            ];
            let table = Table::new(rows.clone(), widths)
                .column_spacing(1)
                .style(Style::new().blue())
                .header(
                    Row::new(vec![
                        "Time code",
                        "Monday",
                        "Tuesday",
                        "Wednesday",
                        "Thursday",
                        "Friday",
                        "Saturday",
                        "Sunday",
                        "Total",
                    ])
                    .style(Style::new().bold())
                    .bottom_margin(1),
                )
                .footer(Row::new(footer_cells))
                .block(Block::new().borders(Borders::ALL).title(format!(
                    "Week {week} [{} - {}]",
                    start_of_week.date_naive(),
                    end_of_week.date_naive()
                )))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">>");
            frame.render_widget(table, frame.area());
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('p') {
                current_date = start_of_week - Duration::days(7);
            }
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('n') {
                current_date = start_of_week + Duration::days(7);
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
