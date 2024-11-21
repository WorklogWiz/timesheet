use std::collections::HashSet;
use std::path::PathBuf;
use std::process::exit;
use std::{
    fs::{self, File},
    io,
};

use crate::journal::{Entry, Journal};
use anyhow::{anyhow, Context};
use csv::{Reader, ReaderBuilder, Writer, WriterBuilder};
use log::debug;

// If you add or remove any fields from the JournalEntry struct, update this:
const NUM_JOURNAL_FIELDS: usize = 5;
const CSV_DELIMITER: u8 = b';';

#[allow(clippy::module_name_repetitions)]
pub struct JournalCsv {
    pub journal_file_name: PathBuf,
}

impl JournalCsv {
    #[must_use]
    pub fn new(journal_file_name: PathBuf) -> Self {
        JournalCsv { journal_file_name }
    }

    /// Ensures we always create the CSV writer with the same delimiter
    fn create_csv_writer<W: io::Write>(file: W) -> Writer<W> {
        WriterBuilder::new()
            .delimiter(CSV_DELIMITER)
            .has_headers(false)
            .from_writer(file)
    }

    /// Ensures we always create CSV readers with our standard delimiter
    fn create_csv_reader<R: io::Read>(rdr: R) -> Reader<R> {
        ReaderBuilder::new()
            .delimiter(CSV_DELIMITER)
            .has_headers(true)
            .from_reader(rdr)
    }

    #[allow(clippy::missing_errors_doc)]
    fn create_or_open_for_append(&self) -> io::Result<File> {
        let path_to_file = &self.journal_file_name;
        if let Some(parent_dir) = path_to_file.parent() {
            if !parent_dir.exists() {
                debug!(
                    "Creating all intermittent directories for {}",
                    path_to_file.to_string_lossy()
                );
                fs::create_dir_all(parent_dir)?;
            }
        }

        // Creates the CSV header if the file is being created.
        if path_to_file.try_exists()? {
            debug!(
                "File {} seems to exist, great!",
                path_to_file.to_string_lossy()
            );
        } else {
            debug!(
                "File {} does not exist, creating it",
                path_to_file.to_string_lossy()
            );

            let journal_file = File::create_new(path_to_file)?;
            let mut csv_writer = JournalCsv::create_csv_writer(journal_file);
            debug!("Writing the CSV header");
            csv_writer.write_record(["key", "w_id", "started", "time spent", "comment"])?;
            csv_writer.flush()?;
        }

        debug!("Opening file {}.", path_to_file.to_string_lossy());
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path_to_file)
    }
}

impl Journal for JournalCsv {
    #[allow(clippy::missing_errors_doc)]
    fn add_worklog_entries(&self, worklog: Vec<Entry>) -> anyhow::Result<()> {
        let journal_file = self.create_or_open_for_append()?;
        let mut csv_writer = Self::create_csv_writer(&journal_file);

        for entry in worklog {
            csv_writer.serialize(entry)?;
        }

        csv_writer.flush()?;

        Ok(())
    }
    #[allow(clippy::missing_errors_doc)]
    fn remove_entry(&self, worklog_id_to_remove: &str) -> anyhow::Result<()> {
        debug!(
            "Removing key {} from file {}",
            worklog_id_to_remove,
            self.journal_file_name.to_string_lossy()
        );
        // Return immediately if the journal file does not exist, i.e. no entries have been written
        if !PathBuf::from(&self.journal_file_name).try_exists().unwrap() {
            return Ok(());
        }
        let file = File::open(&self.journal_file_name).with_context(|| {
            format!(
                "error opening {}",
                &self.journal_file_name.to_string_lossy()
            )
        })?;

        let mut rd = JournalCsv::create_csv_reader(file);

        let mut records_to_keep = Vec::new();

        for result in rd.records() {
            let record = match result {
                Ok(r) => r,
                Err(err) => {
                    return Err(anyhow!("Unable to read CSV records: {err:?}"));
                }
            };

            // In case the parsing of the record failed
            if record.len() < NUM_JOURNAL_FIELDS && record.len() == 1 {
                eprintln!("Only a single column in CSV entry: {}", &record[0]);
                eprintln!("Invalid number of columns, did you remember the delimiter ';'?");
                exit(4);
            }

            // Worklog id is in the second column 0,1,2,3
            let key = &record[1];
            if key != worklog_id_to_remove {
                records_to_keep.push(record);
            }
        }

        // Rewrite filtered data back to the CSV file
        let file = File::create(&self.journal_file_name).with_context(|| {
            format!(
                "Unable to open journal: {}",
                self.journal_file_name.to_string_lossy()
            )
        })?;
        let mut csv_writer = JournalCsv::create_csv_writer(file);
        for record in records_to_keep {
            csv_writer.write_record(&record).with_context(|| {
                format!(
                    "Unable to write record {:?} to {}",
                    &record,
                    self.journal_file_name.to_string_lossy()
                )
            })?;
        }

        csv_writer.flush().with_context(|| {
            format!(
                "Unable to flush the CSV file {:?}",
                self.journal_file_name.to_string_lossy()
            )
        })?;
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn find_unique_keys(&self) -> anyhow::Result<Vec<String>> {
        // Return immediately if the journal does not exist, i.e. it is empty
        if !PathBuf::from(&self.journal_file_name).try_exists().unwrap() {
            return Ok(vec![]);
        }

        let file = File::open(&self.journal_file_name)?;
        let mut keys: HashSet<String> = HashSet::new();

        let mut csv_reader = JournalCsv::create_csv_reader(file);

        for result in csv_reader.records() {
            let record = result?;
            if let Some(key) = record.get(0) {
                keys.insert(key.to_string());
            }
        }
        let mut result: Vec<String> = keys.into_iter().collect();
        result.sort();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, Write};
    use std::sync::Once;

    use env_logger;
    use rand::Rng;

    use crate::journal::Journal;

    use super::*;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    #[test]
    fn test_create_or_open_worklog_journal() {
        let tmp_config_file = std::env::temp_dir().join("worklog.tmp");
        let _journal = JournalCsv::new(tmp_config_file.clone());
    }

    /// Writes a sample journal file, attempts to remove a single entry
    /// and then check the file to ensure the record has been removed
    #[test]
    fn test_remove_entry() -> Result<(), String> {
        setup();
        let path_buf = create_sample_journal();

        // Removes a single record identified by the worklog id
        JournalCsv::new(path_buf.clone())
            .remove_entry("315100")
            .map_err(|e| format!("unable to remove entry 315100: {e} "))?;
        eprintln!("Rewrote {}", path_buf.to_string_lossy());

        // Opens the journal file again and verifies the removal of the record
        let path_buf = JournalCsv::new(path_buf).journal_file_name;
        match File::open(path_buf.clone()) {
            Ok(file) => {
                let buf = io::BufReader::new(file);

                let result: Vec<String> = buf
                    .lines()
                    .filter_map(|l| {
                        if l.as_ref().unwrap().contains("315100") {
                            l.ok() // Transforms Result<String> to Some<String>
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(result.is_empty(), "Entry not removed {result:?}");
            }
            Err(err) => {
                panic!("Error: {} when opening {}", err, path_buf.to_string_lossy())
            }
        }
        Ok(())
    }

    #[test]
    fn test_unique_time_codes_from_journal() -> anyhow::Result<()> {
        let path = create_sample_journal();
        let journal = JournalCsv::new(path);
        let unique_keys: Vec<String> = journal.find_unique_keys()?;
        assert!(!unique_keys.is_empty());
        assert_eq!(vec!["TIME-117", "TIME-147", "TIME-148"], unique_keys);
        Ok(())
    }

    fn create_sample_journal() -> PathBuf {
        let data = r"key;w_id;started;time spent;comment
TIME-147;314335;2024-09-19 20:21 +0200;02:00;jira_worklog
TIME-148;315100;2024-09-20 11:57 +0200;01:00;Information meeting on time codes
TIME-117;315377;2024-09-20 14:33 +0200;01:00;ASOS Product Roadmap
TIME-147;315633;2024-09-20 18:48 +0200;05:00;Admin
TIME-147;315634;2024-09-20 22:49 +0200;01:00;jira_worklog
";
        // Creates the temporary file with "random" name
        let path_buf = std::env::temp_dir().join(format!(
            "journal-{}.csv",
            rand::thread_rng().gen_range(0..1000)
        ));

        let mut file = File::create(&path_buf).expect("Unable to create temporary file ");
        let _result = file.write_all(data.as_bytes());
        path_buf
    }
}
