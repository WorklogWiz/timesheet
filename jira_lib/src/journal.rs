use std::{fs, io};
use std::fs::File;
use std::path::{ PathBuf};
use std::process::exit;
use chrono::{DateTime, Local, };
use log::debug;
use serde::{Deserialize, Serialize, Serializer};
use crate::{config, date_util, };

#[derive(Serialize, Deserialize,PartialEq,Clone)]
pub struct JournalEntry {
    pub issue_key: String,
    pub worklog_id: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub started: DateTime<Local>,
    #[serde(serialize_with = "serialize_seconds")]
    pub time_spent_seconds: i32,
    pub comment: Option<String>
}

fn serialize_datetime<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_date = date.format("%Y-%m-%d %H:%M %z").to_string();
    serializer.serialize_str(&formatted_date)
}

fn serialize_seconds<S>(seconds: &i32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_duration = date_util::seconds_to_hour_and_min(&seconds);
    serializer.serialize_str(&formatted_duration)
}

pub fn add_worklog_entries_to_journal(worklog: Vec<JournalEntry>) {

    let file_name = config::journal_data_file_name();
    match create_or_open_worklog_journal(&file_name){
        Ok(file) => {
            let mut csv_writer = csv::WriterBuilder::new ()
                .delimiter(b';')
                .has_headers(false)
                .from_writer(&file);

            for entry in worklog {
                match csv_writer.serialize(entry){
                    Ok(_) => {}
                    Err(err) => { eprintln!("Error writing journal entry {}", err);
                        exit(4);
                    }
                }
            }
            match csv_writer.flush() {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Unable to flush the journal entries: {}", err);
                    exit(4);
                }
            }
        }
        Err(err) => {
            eprintln!("ERROR: Unable to write journal entry to {}, cause: {:?}", &file_name.to_string_lossy(),err);
            exit(4);
        }
    };


}

pub fn create_or_open_worklog_journal(path_to_file: &PathBuf) -> io::Result<File>{
    if let Some(parent_dir) = path_to_file.parent() {
        if !parent_dir.exists() {
            debug!("Creating all intermittent directories for {}", path_to_file.to_string_lossy());
            fs::create_dir_all(parent_dir)?;
        }
    }

    // Creates the CSV header if the file is being created.
    if !path_to_file.try_exists()? {
        debug!("File {} does not exist, creating it", path_to_file.to_string_lossy());

        match File::create_new(path_to_file) {
            Ok(journal_file) => {
                let mut csv_writer = csv::WriterBuilder::new()
                    .delimiter(b';')
                    .from_writer(journal_file);
                debug!("Writing the CSV header");
                csv_writer.write_record(&["key", "w_id", "started", "time spent", "comment"])?;
                csv_writer.flush()?;
            }
            Err(err) => {
                eprintln!("Unable to create file {}, reason: {}", path_to_file.to_string_lossy(), err);
                exit(4);
            }
        }
    } else {
        debug!("File {} seems to exist", path_to_file.to_string_lossy());

    }
    if !path_to_file.is_file() {
        eprintln!("Unable to create the journal file {}", path_to_file.to_string_lossy());
        exit(4);
    }
    debug!("Opening file {}", path_to_file.to_string_lossy());

    fs::OpenOptions::new().append(true).create(true).open(path_to_file)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;

    #[test]
    fn test_create_or_open_worklog_journal() {
        let tmp_config_file = std::env::temp_dir().join("worklog.tmp");
        let journal_result = create_or_open_worklog_journal(&tmp_config_file.clone());
        assert!(journal_result.is_ok(), "Unable to create {:?}", &tmp_config_file.to_string_lossy());

        let mut journal = journal_result.unwrap();
        writeln!(journal, "Hello World");
        drop(journal);
        fs::remove_file(&tmp_config_file).unwrap();
        eprintln!("Created and removed {}", tmp_config_file.to_string_lossy());
    }
}