use std::path::PathBuf;

use anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::SecondsFormat;
use rusqlite::{Connection, params};

use worklog_lib::journal::{Entry, Journal};

struct JournalSqlite {
    db_path: PathBuf,
    connection: Connection,
}

impl JournalSqlite {

    pub(crate) fn new(journal_file_name: &PathBuf) -> anyhow::Result<Self> {
        // TODO: Ensure the extension is ".db"
        let mut db_path = journal_file_name.clone();
        db_path.set_extension("db");
        let connection = create_connect(&db_path)?;
        create_journal(&connection)?;
        Ok(JournalSqlite { db_path, connection })
    }
}

impl Journal for JournalSqlite {
    fn add_worklog_entries(&self, worklog: Vec<Entry>) -> Result<()> {
        for e in worklog {
            let result = self.connection.execute("insert into worklog (issue_key, worklog_id, started, time_spent_seconds, comment) values(?,?,?,?,?)",
                                                 params![
                                                     &e.issue_key,
                                                     &e.worklog_id,
                                                     &e.started.to_rfc3339_opts(SecondsFormat::Secs,false),
                                                     e.time_spent_seconds,
                                                     &e.comment
                                                 ])?;
            let last_id: String = self.connection.last_insert_rowid().to_string();
        }
        Ok(())
    }

    fn remove_entry(&self, worklog_id_to_remove: &str) -> Result<()> {

        Ok(())
    }

    fn find_unique_keys(&self) -> anyhow::Result<Vec<String>> {
        todo!()
    }
}

fn create_connect(dbms_url: &PathBuf) -> Result<Connection> {
    Ok(Connection::open(dbms_url.as_path())?)
}

fn create_journal(connection: &Connection) -> anyhow::Result<usize> {
    let sql = r#"
    create table if not exists worklog (
        id integer primary key not null,
        issue_key varchar(16),
        worklog_id varchar(16),
        started datetime,
        time_spent_seconds integer,
        comment varchar(1024)
    );
    "#;

    Ok(connection.execute(&sql,[]).with_context(|| "Unable to create table 'worklog'")?)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Local};

    use worklog_lib::config;

    use super::*;

    fn get_journal_for_test() -> Result<JournalSqlite> {
        JournalSqlite::new(&config::journal_data_file_name())
    }

    #[test]
    fn test_create_dbms() -> anyhow::Result<()>{
        let journal = get_journal_for_test()?;
        let mut path = config::journal_data_file_name().clone();
        path.set_extension("db");
        assert_eq!(path, journal.db_path);

        let _p = create_connect(&journal.db_path)?;
        assert!(journal.db_path.exists());
        Ok(())
    }

    #[test]
     fn test_add_journal_entry() -> Result<()>{
        let journal = get_journal_for_test()?;
        let entry = Entry {
            issue_key: "TIME-147".to_string(),
            worklog_id: "315633".to_string(),
            time_spent_seconds: 3600,
            started: Local::now(),
            comment: Some("Rubbish".to_string()),
        };
        journal.add_worklog_entries(vec![entry])?;

        Ok(())
    }

    #[test]
    fn test_remove_journal_entry() -> Result<()>  {
        let journal = get_journal_for_test()?;
        journal.remove_entry("315633")?;

        Ok(())
    }
}
