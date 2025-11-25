//! Integration test for database migration from v1 to v2
//!
//! This test:
//! 1. Creates a database with the OLD v1 schema
//! 2. Inserts sample data
//! 3. Runs the migration
//! 4. Verifies all data was correctly migrated

use chrono::{Local, TimeZone};
use rusqlite::{params, Connection};
use worklog_sqlite_storage::{create_backup, initialize_or_migrate};

#[test]
fn test_migration_v1_to_v2() {
    // Create a temporary database
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join("test_migration.db");

    // Clean up any existing test database
    let _ = std::fs::remove_file(&db_path);

    println!("📁 Test database: {}", db_path.display());

    // Step 1: Create OLD schema and populate with data
    {
        let conn = Connection::open(&db_path).expect("Failed to create database");
        create_v1_schema(&conn).expect("Failed to create v1 schema");
        populate_v1_data(&conn).expect("Failed to populate v1 data");

        // Verify v1 data
        verify_v1_data(&conn);
    }

    // Step 2: Run migration
    println!("\n🔄 Running migration...");
    {
        let mut conn = Connection::open(&db_path).expect("Failed to open database");

        // Create backup first
        create_backup(&db_path).expect("Failed to create backup");

        // Run migration
        initialize_or_migrate(&mut conn).expect("Migration failed");

        println!("✅ Migration completed");
    }

    // Step 3: Verify v2 data
    {
        let conn = Connection::open(&db_path).expect("Failed to open database");
        verify_v2_data(&conn);
    }

    println!("\n🎉 Migration test PASSED!");

    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Create the OLD v1 schema
fn create_v1_schema(conn: &Connection) -> rusqlite::Result<()> {
    println!("\n📋 Creating v1 schema...");

    // User table (v1)
    conn.execute(
        r"
        CREATE TABLE user (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            email TEXT
        )
        ",
        [],
    )?;

    // Issue table (v1)
    conn.execute(
        r"
        CREATE TABLE issue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_key TEXT UNIQUE NOT NULL,
            summary TEXT NOT NULL
        )
        ",
        [],
    )?;

    // Component table (v1)
    conn.execute(
        r"
        CREATE TABLE component (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            jira_id TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL
        )
        ",
        [],
    )?;

    // Issue-Component junction table (v1)
    conn.execute(
        r"
        CREATE TABLE issue_component (
            issue_id INTEGER NOT NULL,
            component_id INTEGER NOT NULL,
            PRIMARY KEY (issue_id, component_id),
            FOREIGN KEY (issue_id) REFERENCES issue(id),
            FOREIGN KEY (component_id) REFERENCES component(id)
        )
        ",
        [],
    )?;

    // Worklog table (v1) - completed worklogs
    conn.execute(
        r"
        CREATE TABLE worklog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_key TEXT NOT NULL,
            started DATETIME NOT NULL,
            time_spent_seconds INTEGER NOT NULL,
            comment TEXT NOT NULL,
            jira_worklog_id TEXT,
            FOREIGN KEY (issue_key) REFERENCES issue(issue_key)
        )
        ",
        [],
    )?;

    // Timer table (v1) - active timers
    conn.execute(
        r"
        CREATE TABLE timer (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_key TEXT NOT NULL,
            started DATETIME NOT NULL,
            comment TEXT
        )
        ",
        [],
    )?;

    println!("✅ V1 schema created");
    Ok(())
}

/// Populate v1 database with test data
fn populate_v1_data(conn: &Connection) -> rusqlite::Result<()> {
    println!("\n📝 Populating v1 data...");

    // Insert users
    conn.execute(
        "INSERT INTO user (id, display_name, email) VALUES (?1, ?2, ?3)",
        params!["user123", "John Doe", "john@example.com"],
    )?;

    conn.execute(
        "INSERT INTO user (id, display_name, email) VALUES (?1, ?2, ?3)",
        params!["user456", "Jane Smith", "jane@example.com"],
    )?;

    // Insert issues
    conn.execute(
        "INSERT INTO issue (issue_key, summary) VALUES (?1, ?2)",
        params!["PROJ-123", "Implement user authentication"],
    )?;
    let issue1_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO issue (issue_key, summary) VALUES (?1, ?2)",
        params!["PROJ-456", "Fix memory leak in parser"],
    )?;
    let issue2_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO issue (issue_key, summary) VALUES (?1, ?2)",
        params!["PROJ-789", "Update documentation"],
    )?;
    let issue3_id = conn.last_insert_rowid();

    // Insert components
    conn.execute(
        "INSERT INTO component (jira_id, name) VALUES (?1, ?2)",
        params!["comp_1", "Backend"],
    )?;
    let comp1_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO component (jira_id, name) VALUES (?1, ?2)",
        params!["comp_2", "API"],
    )?;
    let comp2_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO component (jira_id, name) VALUES (?1, ?2)",
        params!["comp_3", "Documentation"],
    )?;
    let comp3_id = conn.last_insert_rowid();

    // Associate components with issues
    conn.execute(
        "INSERT INTO issue_component (issue_id, component_id) VALUES (?1, ?2)",
        params![issue1_id, comp1_id],
    )?;
    conn.execute(
        "INSERT INTO issue_component (issue_id, component_id) VALUES (?1, ?2)",
        params![issue1_id, comp2_id],
    )?;

    conn.execute(
        "INSERT INTO issue_component (issue_id, component_id) VALUES (?1, ?2)",
        params![issue2_id, comp1_id],
    )?;

    conn.execute(
        "INSERT INTO issue_component (issue_id, component_id) VALUES (?1, ?2)",
        params![issue3_id, comp3_id],
    )?;

    // Insert completed worklogs
    let worklog1_time = Local.with_ymd_and_hms(2024, 1, 15, 9, 0, 0).unwrap();
    conn.execute(
        "INSERT INTO worklog (issue_key, started, time_spent_seconds, comment, jira_worklog_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["PROJ-123", worklog1_time.to_rfc3339(), 7200, "Implemented OAuth flow", "jira_wl_1"],
    )?;

    let worklog2_time = Local.with_ymd_and_hms(2024, 1, 15, 14, 0, 0).unwrap();
    conn.execute(
        "INSERT INTO worklog (issue_key, started, time_spent_seconds, comment, jira_worklog_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["PROJ-456", worklog2_time.to_rfc3339(), 3600, "Fixed parser bug", "jira_wl_2"],
    )?;

    let worklog3_time = Local.with_ymd_and_hms(2024, 1, 16, 10, 0, 0).unwrap();
    conn.execute(
        "INSERT INTO worklog (issue_key, started, time_spent_seconds, comment, jira_worklog_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["PROJ-123", worklog3_time.to_rfc3339(), 5400, "Added tests", "jira_wl_3"],
    )?;

    // Insert active timers
    let timer1_time = Local.with_ymd_and_hms(2024, 1, 17, 9, 30, 0).unwrap();
    conn.execute(
        "INSERT INTO timer (issue_key, started, comment) VALUES (?1, ?2, ?3)",
        params!["PROJ-789", timer1_time.to_rfc3339(), "Writing README"],
    )?;

    let timer2_time = Local.with_ymd_and_hms(2024, 1, 17, 11, 0, 0).unwrap();
    conn.execute(
        "INSERT INTO timer (issue_key, started, comment) VALUES (?1, ?2, ?3)",
        params!["PROJ-456", timer2_time.to_rfc3339(), "Code review"],
    )?;

    println!("✅ V1 data populated:");
    println!("  • 2 users");
    println!("  • 3 issues");
    println!("  • 3 components");
    println!("  • 4 component associations");
    println!("  • 3 completed worklogs");
    println!("  • 2 active timers");

    Ok(())
}

/// Verify data exists in v1 schema
fn verify_v1_data(conn: &Connection) {
    println!("\n🔍 Verifying v1 data...");

    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM user", [], |row| row.get(0))
        .unwrap();
    assert_eq!(user_count, 2, "Expected 2 users in v1");

    let issue_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue", [], |row| row.get(0))
        .unwrap();
    assert_eq!(issue_count, 3, "Expected 3 issues in v1");

    let component_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM component", [], |row| row.get(0))
        .unwrap();
    assert_eq!(component_count, 3, "Expected 3 components in v1");

    let worklog_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM worklog", [], |row| row.get(0))
        .unwrap();
    assert_eq!(worklog_count, 3, "Expected 3 worklogs in v1");

    let timer_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM timer", [], |row| row.get(0))
        .unwrap();
    assert_eq!(timer_count, 2, "Expected 2 timers in v1");

    println!("✅ V1 data verified");
}

/// Verify data after migration to v2
#[allow(clippy::too_many_lines)]
fn verify_v2_data(conn: &Connection) {
    println!("\n🔍 Verifying v2 data...");

    // Check schema version (now v3 with sync support)
    let schema_version: i32 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .expect("Schema version not found");
    assert_eq!(schema_version, 4, "Expected schema version 4");
    println!("✅ Schema version: 4");

    // Check users migrated
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap();
    assert_eq!(user_count, 2, "Expected 2 users in v2");
    println!("✅ Users migrated: {user_count}");

    // Check issues migrated with tags
    let issue_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))
        .unwrap();
    assert_eq!(issue_count, 3, "Expected 3 issues in v2");
    println!("✅ Issues migrated: {issue_count}");

    // Verify PROJ-123 has tags: ["Backend", "API"]
    let tags_json: String = conn
        .query_row(
            "SELECT tags FROM issues WHERE issue_key = ?1",
            params!["PROJ-123"],
            |row| row.get(0),
        )
        .expect("PROJ-123 not found");
    let tags: Vec<String> = serde_json::from_str(&tags_json).expect("Failed to parse tags");
    assert_eq!(tags.len(), 2, "PROJ-123 should have 2 tags");
    assert!(tags.contains(&"Backend".to_string()), "Missing Backend tag");
    assert!(tags.contains(&"API".to_string()), "Missing API tag");
    println!("✅ PROJ-123 tags: {tags:?}");

    // Verify PROJ-456 has tags: ["Backend"]
    let tags_json: String = conn
        .query_row(
            "SELECT tags FROM issues WHERE issue_key = ?1",
            params!["PROJ-456"],
            |row| row.get(0),
        )
        .expect("PROJ-456 not found");
    let tags: Vec<String> = serde_json::from_str(&tags_json).expect("Failed to parse tags");
    assert_eq!(tags.len(), 1, "PROJ-456 should have 1 tag");
    assert!(tags.contains(&"Backend".to_string()), "Missing Backend tag");
    println!("✅ PROJ-456 tags: {tags:?}");

    // Verify PROJ-789 has tags: ["Documentation"]
    let tags_json: String = conn
        .query_row(
            "SELECT tags FROM issues WHERE issue_key = ?1",
            params!["PROJ-789"],
            |row| row.get(0),
        )
        .expect("PROJ-789 not found");
    let tags: Vec<String> = serde_json::from_str(&tags_json).expect("Failed to parse tags");
    assert_eq!(tags.len(), 1, "PROJ-789 should have 1 tag");
    assert!(
        tags.contains(&"Documentation".to_string()),
        "Missing Documentation tag"
    );
    println!("✅ PROJ-789 tags: {tags:?}");

    // Check worklogs (should be 3 completed + 2 timers = 5 total)
    let worklog_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM worklogs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        worklog_count, 5,
        "Expected 5 worklogs in v2 (3 completed + 2 timers)"
    );
    println!("✅ Worklogs migrated: {worklog_count}");

    // Check completed worklogs (stopped_at IS NOT NULL)
    let completed_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE stopped_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(completed_count, 3, "Expected 3 completed worklogs");
    println!("✅ Completed worklogs: {completed_count}");

    // Check active timers (stopped_at IS NULL)
    let active_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE stopped_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(active_count, 2, "Expected 2 active timers");
    println!("✅ Active timers: {active_count}");

    // Verify synced worklogs are marked correctly
    let synced_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE synced_to_provider = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(synced_count, 3, "Expected 3 synced worklogs");
    println!("✅ Synced worklogs: {synced_count}");

    // Verify unsynced timers
    let unsynced_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE synced_to_provider = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unsynced_count, 2, "Expected 2 unsynced worklogs (timers)");
    println!("✅ Unsynced worklogs: {unsynced_count}");

    // Verify provider_worklog_id is set for synced entries
    let provider_id_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE provider_worklog_id IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(provider_id_count, 3, "Expected 3 entries with provider IDs");
    println!("✅ Provider worklog IDs: {provider_id_count}");

    // Verify a specific worklog's data integrity
    let (issue_key, time_spent, comment, synced): (String, i32, String, bool) = conn
        .query_row(
            "SELECT issue_key, time_spent_seconds, comment, synced_to_provider FROM worklogs WHERE provider_worklog_id = ?1",
            params!["jira_wl_1"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("Worklog jira_wl_1 not found");

    assert_eq!(issue_key, "PROJ-123");
    assert_eq!(time_spent, 7200);
    assert_eq!(comment, "Implemented OAuth flow");
    assert!(synced);
    println!("✅ Worklog data integrity verified");

    // Verify created_at and updated_at timestamps exist
    let timestamp_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worklogs WHERE created_at IS NOT NULL AND updated_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(timestamp_count, 5, "All worklogs should have timestamps");
    println!("✅ Timestamps added to all entries");

    println!("\n✅ All v2 data verified successfully!");
}
