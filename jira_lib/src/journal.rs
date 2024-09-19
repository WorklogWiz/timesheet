use std::{fs, io};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::exit;
use chrono::{DateTime, Local, Weekday};
use serde::{Deserialize, Serialize};
use crate::{config, Worklog};

#[derive(Serialize, Deserialize,PartialEq,Clone)]
pub struct JournalEntry {
    pub issue_key: String,
    pub worklog_id: String,
    pub started: DateTime<Local>,
    pub time_spent_seconds: i32,
    pub comment: Option<String>
}

pub fn add_worklog_entries_to_journal(worklog: Vec<JournalEntry>) {

    let file_name = config::journal_data_file_name();
    let mut journal = match create_or_open_worklog_journal(&file_name){
        Ok(file) => {
            let mut csv_writer = csv::WriterBuilder::new ()
                .delimiter(b';')
                .has_headers(false)
                .from_writer(&file);

            for entry in worklog {
                csv_writer.serialize(entry);
            }
            csv_writer.flush();
        }
        Err(err) => {
            eprintln!("ERROR: Unable to write journal entry to {}, cause: {:?}", &file_name.to_string_lossy(),err);
            exit(4);
        }
    };


}

pub fn create_or_open_worklog_journal(path_to_file: &PathBuf) -> io::Result<File>{
    if let Some(parent_dir) = path_to_file.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    fs::OpenOptions::new().append(true).create(true).open(path_to_file)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;

    #[test]
    fn test_create_or_open_worklog_journal() {
        let tmp_config_file = std::env::temp_dir().join("worklog.tmp");
        let journal_result = create_or_open_worklog_journal(tmp_config_file.clone());
        assert!(journal_result.is_ok(), "Unable to create {:?}", &tmp_config_file.to_string_lossy());

        let mut journal = journal_result.unwrap();
        writeln!(journal, "Hello World");
        drop(journal);
        fs::remove_file(&tmp_config_file).unwrap();
        eprintln!("Created and removed {}", tmp_config_file.to_string_lossy());
    }
}