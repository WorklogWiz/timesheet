
use chrono::{DateTime, Local, };
use serde::{Deserialize, Serialize, Serializer};

use crate::date;

pub mod journal_csv;

/// Represents the columns in the journal file, which is CSV formatted
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Entry {
    pub issue_key: String,
    pub worklog_id: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub started: DateTime<Local>,
    #[serde(serialize_with = "serialize_seconds")]
    pub time_spent_seconds: i32,
    pub comment: Option<String>
}

pub trait Journal {
    #[allow(clippy::missing_errors_doc)]
    fn add_worklog_entries(&self, worklog: Vec<Entry>) -> anyhow::Result<()>;
     #[allow(clippy::missing_errors_doc)]
    fn remove_entry(&self, worklog_id_to_remove: &str) -> anyhow::Result<()>;
    #[allow(clippy::missing_errors_doc)]
    fn find_unique_keys(&self) -> anyhow::Result<Vec<String>>;
}

/// Serializes a datetime into the textual format required in a CSV file
/// # Errors
/// On internal errors in the Serde `serialize_str()` function
fn serialize_datetime<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_date = date.format("%Y-%m-%d %H:%M %z").to_string();
    serializer.serialize_str(&formatted_date)
}

/// Serializes number of seconds into the ISO8601 textual format
/// # Errors
/// Fails if the internal Serde `serialize_str()` function fails
#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_seconds<S>(seconds: &i32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_duration = date::seconds_to_hour_and_min(seconds);
    serializer.serialize_str(&formatted_duration)
}