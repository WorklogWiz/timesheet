use crate::error::WorklogError;
use crate::repository::worklog_repository::WorkLogRepository;
use crate::types::LocalWorklog;
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use std::sync::Arc;

pub struct WorkLogService<R: WorkLogRepository> {
    repo: Arc<R>,
}

impl<R: WorkLogRepository> WorkLogService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError> {
        self.repo.remove_entry_by_worklog_id(wl.id.as_str())
    }

    pub fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError> {
        self.repo.remove_entry_by_worklog_id(wl_id)
    }

    pub fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError> {
        self.repo.add_entry(local_worklog)
    }

    pub(crate) fn add_worklog_entries(
        &self,
        worklogs: &[LocalWorklog],
    ) -> Result<(), WorklogError> {
        self.repo.add_worklog_entries(worklogs)
    }

    fn get_count(&self) -> Result<i64, WorklogError> {
        self.repo.get_count()
    }

    fn purge_entire_local_worklog(&self) -> Result<(), WorklogError> {
        self.repo.purge_entire_local_worklog()
    }

    fn find_worklog_by_id(&self, worklog_id: &str) -> Result<LocalWorklog, WorklogError> {
        self.repo.find_worklog_by_id(worklog_id)
    }

    pub fn find_worklogs_after(
        &self,
        start_datetime: DateTime<Local>,
        keys_filter: &[IssueKey],
        users_filter: &[User],
    ) -> Result<Vec<LocalWorklog>, WorklogError> {
        self.repo
            .find_worklogs_after(start_datetime, keys_filter, users_filter)
    }
}
