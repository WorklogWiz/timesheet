use std::{fs, io};
use std::collections::HashSet;
use std::fs::File;
use std::path::{ PathBuf};
use std::process::exit;
use chrono::{DateTime, Local, };
use csv::{ReaderBuilder, WriterBuilder};
use log::debug;
use serde::{Deserialize, Serialize, Serializer};
use crate::{config, date_util, };


/// Represents the columns in the journal file, which is CSV formatted
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
// If you add or remove any fields from the JournalEntry struct, update this:
const NUM_JOURNAL_FIELDS: usize = 5;
const CSV_DELIMITER: u8 = b';';

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
            let mut csv_writer = WriterBuilder::new ()
                .delimiter(CSV_DELIMITER)
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
                let mut csv_writer = WriterBuilder::new()
                    .delimiter(CSV_DELIMITER)
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
        debug!("File {} seems to exist, great!", path_to_file.to_string_lossy());
    }
    if !path_to_file.is_file() {
        eprintln!("Unable to create the journal file {}", path_to_file.to_string_lossy());
        exit(4);
    }
    debug!("Opening file {}", path_to_file.to_string_lossy());

    fs::OpenOptions::new().append(true).create(true).open(path_to_file)
}

pub fn remove_entry_from_journal(path_buf: &PathBuf, worklog_id_to_remove: &str) {
    debug!("Removing key {} from file {}", worklog_id_to_remove, path_buf.to_string_lossy());

    let file = File::open(&path_buf).unwrap_or_else(|err| {
        eprintln!("Unable to open file {}, cause: {}", path_buf.to_string_lossy(), err);
        exit(4);
    });

    let mut rd = ReaderBuilder::new()
        .delimiter(CSV_DELIMITER)
        .has_headers(true).from_reader(file);
    let mut records_to_keep = Vec::new();

    for result in rd.records(){
        let record = result.unwrap_or_else(|err| {
            eprintln!("Unable to unwrap CSV record: {}", err);
            exit(4);
        });

        // In case the parsing of the record failed
        if record.len() < NUM_JOURNAL_FIELDS {
            if record.len() == 1 {
                eprintln!("Only a single column in CSV entry: {}", &record[0]);
                eprintln!("Invalid number of columns, did you remember the delimiter ';'?");
                exit(4);
            }
        }
        // Worklog id is in the second column 0,1,2,3
        let key = &record[1];
        if key != worklog_id_to_remove {
            records_to_keep.push(record);
        }
    }

    // Rewrite filtered data back to the CSV file
    let file = File::create(path_buf).unwrap();
    let mut csv_writer = WriterBuilder::new().has_headers(true).from_writer(file);

    for record in records_to_keep {
        csv_writer.write_record(&record).unwrap();
    }
    csv_writer.flush().unwrap();
}



fn find_unique_keys(p0: &PathBuf) -> Vec<String> {
    let file = File::open(p0).unwrap_or_else(|err| {
        eprintln!("Unable to open {}, cause: {}", p0.to_string_lossy(), err);
        exit(4);
    });
    let mut keys: HashSet<String>= HashSet::new();

    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(CSV_DELIMITER)
        .from_reader(file);

    for result in csv_reader.records(){
        let record = result.unwrap();
        let key = record.get(0).unwrap();
        keys.insert(key.to_string());
    }
    let mut result :Vec<String> = keys.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead,  Write};

    use super::*;

    #[test]
    fn test_create_or_open_worklog_journal() {
        let tmp_config_file = std::env::temp_dir().join("worklog.tmp");
        let journal_result = create_or_open_worklog_journal(&tmp_config_file.clone());
        assert!(journal_result.is_ok(), "Unable to create {:?}", &tmp_config_file.to_string_lossy());

        let mut journal = journal_result.unwrap();
        let _result = writeln!(journal, "Hello World");
        drop(journal);
        fs::remove_file(&tmp_config_file).unwrap();
        eprintln!("Created and removed {}", tmp_config_file.to_string_lossy());
    }

    /// Writes a sample journal file, attempts to remove a single entry
    /// and then check the file to ensure the record has been removed
    #[test]
    fn test_remove_entry() {
        let path_buf = create_sample_journal();

        // Removes a single record identified by the worklog id
        let _result = remove_entry_from_journal(&path_buf, "315100");
        eprintln!("Rewrote {}", path_buf.to_string_lossy());

        // Opens the journal file again and verifies the removal of the record
        let file = File::open(path_buf).unwrap();
        let buf = io::BufReader::new(file);

        let result: Vec<String>  = buf.lines().filter_map(|l| {
            if l.as_ref().unwrap().contains("315100") {
                l.ok() // Transforms Result<String> to Some<String>
            } else {
                None
            }
        }
        ).collect();
        assert!(result.is_empty(),"Entry not removed {:?}", result);
    }

    #[test]
    fn test_unique_time_codes_from_journal() {
        let file_name = create_sample_journal();
        let unique_keys: Vec<String> = find_unique_keys(&file_name);
        assert!(!unique_keys.is_empty());
        assert_eq!(vec!["TIME-117", "TIME-147", "TIME-148"], unique_keys);
    }

    fn create_sample_journal() -> PathBuf {
        let sample_date = r#"key;w_id;started;time spent;comment
TIME-147;314335;2024-09-19 20:21 +0200;02:00;jira_worklog
TIME-148;315100;2024-09-20 11:57 +0200;01:00;Information meeting on time codes
TIME-117;315377;2024-09-20 14:33 +0200;01:00;ASOS Product Roadmap
TIME-147;315633;2024-09-20 18:48 +0200;05:00;Admin
TIME-147;315634;2024-09-20 22:49 +0200;01:00;jira_worklog
"#;
        // Creates the temporary file
        let path_buf = std::env::temp_dir().join("tmp_journal.csv");
        let mut file = File::create(&path_buf).expect("Unable to create temporary file ");
        let _result = file.write(sample_date.as_bytes());
        drop(file); // Close the file
        path_buf
    }
}