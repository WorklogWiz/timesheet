use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    pub id: String,
    pub issue_key: Option<String>, // Optional - can be assigned later
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub comment: Option<String>,
    pub in_progress: bool,
    pub synced_to_tracker: bool, // Whether this has been synced to the issue tracker
    pub tracker_worklog_id: Option<String>, // Issue tracker's worklog ID if synced
    pub time_codes: Vec<TimeCode>,
}

#[allow(dead_code)]
impl WorkSession {
    pub fn new(start_time: DateTime<Utc>, end_time: Option<DateTime<Utc>>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            issue_key: None,
            start_date: start_time,
            end_date: end_time,
            comment: None,
            in_progress: end_time.is_none(),
            synced_to_tracker: false,
            tracker_worklog_id: None,
            time_codes: Vec::new(),
        }
    }

    pub fn get_duration(&self) -> Duration {
        let end = self.end_date.unwrap_or_else(Utc::now);
        end.signed_duration_since(self.start_date)
    }

    pub fn get_duration_string(&self) -> String {
        let duration = self.get_duration();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCode {
    pub id: String,
    pub name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
impl TimeCode {
    pub fn new(name: String, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            start_date,
            end_date: Some(end_date),
        }
    }

    pub fn get_duration(&self) -> Duration {
        if let Some(end) = self.end_date {
            end.signed_duration_since(self.start_date)
        } else {
            Duration::zero()
        }
    }

    pub fn get_duration_string(&self) -> String {
        let duration = self.get_duration();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        format!("{hours}h {minutes:02}m")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Project {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Worklog {
    pub issue_key: String,
    pub time_spent: String,
    pub time_spent_seconds: i32,
    pub created: DateTime<Utc>,
    pub started: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub issue_id: String,
    pub comment: Option<String>,
}
