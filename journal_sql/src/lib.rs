use std::path::PathBuf;

use anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::SecondsFormat;
use rusqlite::{params, Connection};

use common::journal::{Entry, Journal};

pub struct JournalSqlite {
    db_path: PathBuf,
    connection: Connection,
}

impl JournalSqlite {
    pub fn new(journal_file_name: &PathBuf) -> anyhow::Result<Self> {
        // TODO: Ensure the extension is ".db"
        let mut db_path = journal_file_name.clone();
        db_path.set_extension("db");
        let connection = connect_sqlite(&db_path)?;
        create_journal(&connection)?;
        Ok(JournalSqlite {
            db_path,
            connection,
        })
    }

    #[must_use]
    pub fn entry_count(&self) -> Result<i32> {
         let count:i32 = self.connection.query_row(
            "select count(*) from worklog",
            params![],
            |row|  row.get(0) )?;
        Ok(count)
    }
}

impl Journal for JournalSqlite {
    fn add_worklog_entries(&self, worklog: Vec<Entry>) -> Result<()> {
        for e in worklog {
            let _result = self.connection
                .execute("insert into worklog (issue_key, worklog_id, started, time_spent_seconds, comment) values(?,?,?,?,?)",
                                                 params![
                                                     &e.issue_key,
                                                     &e.worklog_id,
                                                     &e.started.to_rfc3339_opts(SecondsFormat::Secs,false),
                                                     e.time_spent_seconds,
                                                     &e.comment
                                                 ])?;
            let _last_id: String = self.connection.last_insert_rowid().to_string();
        }
        Ok(())
    }

    fn remove_entry(&self, worklog_id_to_remove: &str) -> Result<()> {
        self.connection.execute(
            "delete from worklog where worklog_id = ?",
            params![worklog_id_to_remove],
        )?;
        Ok(())
    }

    fn find_unique_keys(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.connection.prepare("select distinct(issue_key) from worklog")?;
        let issue_keys_iter = stmt.query_map([], |row|   row.get::<_,String>(0))?;
        let mut issue_keys = Vec::new();
        for issue_key in issue_keys_iter {
            issue_keys.push(issue_key?);
        }

        Ok(issue_keys)
    }
}

fn connect_sqlite(dbms_url: &PathBuf) -> Result<Connection> {
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

    Ok(connection
        .execute(&sql, [])
        .with_context(|| "Unable to create table 'worklog'")?)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::remove_file;
    use chrono::{DateTime, Local};

    use common::{config, date};

    use super::*;


    ///
    /// Creates a small sample database for testing purposes
    fn get_journal_for_test2() -> Result<JournalSqlite> {
        let data = r"
TIME-147;314335;2024-09-19 20:21 +0200;02:00;jira_worklog
TIME-148;315100;2024-09-20 11:57 +0200;01:00;Information meeting on time codes
TIME-117;315377;2024-09-20 14:33 +0200;01:00;ASOS Product Roadmap
TIME-147;315633;2024-09-20 18:48 +0200;05:00;Admin
TIME-147;315634;2024-09-20 22:49 +0200;01:00;jira_worklog
";
        let tmp_db = config::tmp_local_worklog_dbms_file_name()?;
        let journal = JournalSqlite::new(&tmp_db)?;
        let lines = data.lines();
        for line in lines {
            if line.len() <= 1 {
                continue
            }

            let fields: Vec<&str> = line.split(';').collect();
            let date_time = DateTime::parse_from_str(fields[2], "%Y-%m-%d %H:%M %z")
                .with_context(|| format!("Unable to parse {}", fields[2]))?;
            let started = date_time.with_timezone(&Local);
            // let i = fields[3].parse::<i32>().with_context(|| format!("Unable to parse {} to i32", fields[3]))?;
            let e = Entry {
                issue_key: fields[0].into(),
                worklog_id: fields[1].into(),
                started,
                time_spent_seconds: date::parse_hour_and_minutes_to_seconds(fields[3])?,
                comment: Some(fields[4].into()),
            };

            journal.add_worklog_entries(vec![e])?;
        }
        Ok(journal)
    }


    #[test]
    fn test_create_dbms() -> anyhow::Result<()> {
        let journal = get_journal_for_test2()?;

        let _p = connect_sqlite(&journal.db_path)?;
        assert!(journal.db_path.exists());
        Ok(())
    }

    #[test]
    fn test_add_journal_entry() -> Result<()> {
        let journal = get_journal_for_test2()?;

        let entry = Entry {
            issue_key: "TIME-147".to_string(),
            worklog_id: "315633".to_string(),
            time_spent_seconds: 3600,
            started: Local::now(),
            comment: Some("Rubbish".to_string()),
        };
        journal.add_worklog_entries(vec![entry])?;

        let count_after = journal.entry_count()?;
        assert_eq!(6, count_after);

        Ok(())
    }

    #[test]
    fn test_remove_journal_entry() -> Result<()> {
        let journal = get_journal_for_test2()?;
        journal.remove_entry("315633")?;

        Ok(())
    }

    #[test]
    fn test_find_unique_keys() -> Result<()> {
        let journal = get_journal_for_test2()?;
        let result = journal.find_unique_keys()?;
        assert_eq!(result.len(),3);
        Ok(())
    }


}
